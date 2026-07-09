use crate::{core::Matrix44, moc3::Moc3DrawableBlendMode};

use crate::render::common::{
    ClippingLayout as WgpuClippingLayout, ClippingLayoutError as WgpuClippingLayoutError,
    MaskChannel as WgpuMaskChannel,
};

use super::{
    WgpuLive2dRenderer,
    buffers::drawable_vertex_layout,
    clipping::{
        WgpuClippingPlan, WgpuClippingResources, WgpuMaskRenderTarget, WgpuPreparedClippingContext,
    },
    texture::{
        WgpuClipParams, WgpuMaskParams, WgpuTexture, WgpuTextureError, WgpuTransform,
        create_wgpu_clip_params, create_wgpu_mask_params, create_wgpu_transform, rgba8_len,
    },
};

pub fn live2d_wgsl_source() -> &'static str {
    include_str!("../shaders/live2d.wgsl")
}

pub fn live2d_masked_wgsl_source() -> &'static str {
    include_str!("../shaders/live2d_masked.wgsl")
}

pub fn mask_wgsl_source() -> &'static str {
    include_str!("../shaders/mask.wgsl")
}

pub fn preferred_surface_format(formats: &[wgpu::TextureFormat]) -> Option<wgpu::TextureFormat> {
    if let Some(format) = formats.iter().copied().find_map(|format| {
        let unorm = format.remove_srgb_suffix();
        (unorm != format && formats.contains(&unorm)).then_some(unorm)
    }) {
        return Some(format);
    }

    if let Some(format) = formats.iter().copied().find(|format| !format.is_srgb()) {
        return Some(format);
    }

    formats.first().copied()
}

fn mask_drawable_indices_match(
    indices: &[usize],
    masks: &[i32],
) -> Result<bool, WgpuClippingLayoutError> {
    let mut matches = indices.len() == masks.len();

    // Validate every mask index even when the first mismatch is enough to rebuild resources.
    for (position, &drawable_index) in masks.iter().enumerate() {
        let drawable_index = usize::try_from(drawable_index)
            .map_err(|_| WgpuClippingLayoutError::InvalidMaskDrawableIndex { drawable_index })?;
        matches &= indices.get(position).copied() == Some(drawable_index);
    }

    Ok(matches)
}

pub fn live2d_blend_state(blend_mode: Moc3DrawableBlendMode) -> wgpu::BlendState {
    match blend_mode {
        Moc3DrawableBlendMode::Normal => blend_state(
            (wgpu::BlendFactor::One, wgpu::BlendFactor::OneMinusSrcAlpha),
            (wgpu::BlendFactor::One, wgpu::BlendFactor::OneMinusSrcAlpha),
        ),
        Moc3DrawableBlendMode::Additive => blend_state(
            (wgpu::BlendFactor::One, wgpu::BlendFactor::One),
            (wgpu::BlendFactor::Zero, wgpu::BlendFactor::One),
        ),
        Moc3DrawableBlendMode::Multiplicative => blend_state(
            (wgpu::BlendFactor::Dst, wgpu::BlendFactor::OneMinusSrcAlpha),
            (wgpu::BlendFactor::Zero, wgpu::BlendFactor::One),
        ),
    }
}

pub fn wgpu_mask_blend_state() -> wgpu::BlendState {
    blend_state(
        (wgpu::BlendFactor::One, wgpu::BlendFactor::One),
        (wgpu::BlendFactor::One, wgpu::BlendFactor::One),
    )
}

fn blend_state(
    color: (wgpu::BlendFactor, wgpu::BlendFactor),
    alpha: (wgpu::BlendFactor, wgpu::BlendFactor),
) -> wgpu::BlendState {
    wgpu::BlendState {
        color: blend_component(color),
        alpha: blend_component(alpha),
    }
}

fn blend_component(factors: (wgpu::BlendFactor, wgpu::BlendFactor)) -> wgpu::BlendComponent {
    wgpu::BlendComponent {
        src_factor: factors.0,
        dst_factor: factors.1,
        operation: wgpu::BlendOperation::Add,
    }
}

impl WgpuLive2dRenderer {
    pub fn new(device: &wgpu::Device, color_format: wgpu::TextureFormat) -> Self {
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("live2d.texture.bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let transform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("live2d.transform.bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(64),
                    },
                    count: None,
                }],
            });
        let mask_params_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("live2d.mask.params.bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(32),
                    },
                    count: None,
                }],
            });
        let clip_params_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("live2d.clip.params.bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(96),
                    },
                    count: None,
                }],
            });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("live2d.texture.sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("live2d.shader"),
            source: wgpu::ShaderSource::Wgsl(live2d_wgsl_source().into()),
        });
        let masked_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("live2d.masked.shader"),
            source: wgpu::ShaderSource::Wgsl(live2d_masked_wgsl_source().into()),
        });
        let mask_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("live2d.mask.shader"),
            source: wgpu::ShaderSource::Wgsl(mask_wgsl_source().into()),
        });
        let bind_group_layouts = [
            Some(&texture_bind_group_layout),
            Some(&transform_bind_group_layout),
        ];
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("live2d.pipeline.layout"),
            bind_group_layouts: &bind_group_layouts,
            immediate_size: 0,
        });
        let normal_pipeline = create_live2d_pipeline(
            device,
            &pipeline_layout,
            &shader,
            color_format,
            Moc3DrawableBlendMode::Normal,
            "live2d.pipeline.normal",
        );
        let additive_pipeline = create_live2d_pipeline(
            device,
            &pipeline_layout,
            &shader,
            color_format,
            Moc3DrawableBlendMode::Additive,
            "live2d.pipeline.additive",
        );
        let multiplicative_pipeline = create_live2d_pipeline(
            device,
            &pipeline_layout,
            &shader,
            color_format,
            Moc3DrawableBlendMode::Multiplicative,
            "live2d.pipeline.multiplicative",
        );
        let mask_bind_group_layouts = [
            Some(&texture_bind_group_layout),
            Some(&transform_bind_group_layout),
            Some(&mask_params_bind_group_layout),
        ];
        let mask_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("live2d.mask.pipeline.layout"),
            bind_group_layouts: &mask_bind_group_layouts,
            immediate_size: 0,
        });
        let mask_pipeline = create_live2d_mask_pipeline(
            device,
            &mask_pipeline_layout,
            &mask_shader,
            "live2d.mask.pipeline",
        );
        let masked_bind_group_layouts = [
            Some(&texture_bind_group_layout),
            Some(&transform_bind_group_layout),
            Some(&texture_bind_group_layout),
            Some(&clip_params_bind_group_layout),
        ];
        let masked_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("live2d.masked.pipeline.layout"),
                bind_group_layouts: &masked_bind_group_layouts,
                immediate_size: 0,
            });
        let masked_normal_pipeline = create_live2d_pipeline(
            device,
            &masked_pipeline_layout,
            &masked_shader,
            color_format,
            Moc3DrawableBlendMode::Normal,
            "live2d.masked.pipeline.normal",
        );
        let masked_additive_pipeline = create_live2d_pipeline(
            device,
            &masked_pipeline_layout,
            &masked_shader,
            color_format,
            Moc3DrawableBlendMode::Additive,
            "live2d.masked.pipeline.additive",
        );
        let masked_multiplicative_pipeline = create_live2d_pipeline(
            device,
            &masked_pipeline_layout,
            &masked_shader,
            color_format,
            Moc3DrawableBlendMode::Multiplicative,
            "live2d.masked.pipeline.multiplicative",
        );
        let identity_transform =
            create_wgpu_transform(device, &transform_bind_group_layout, &Matrix44::identity());

        Self {
            normal_pipeline,
            additive_pipeline,
            multiplicative_pipeline,
            mask_pipeline,
            masked_normal_pipeline,
            masked_additive_pipeline,
            masked_multiplicative_pipeline,
            texture_bind_group_layout,
            transform_bind_group_layout,
            mask_params_bind_group_layout,
            clip_params_bind_group_layout,
            identity_transform,
            sampler,
        }
    }

    pub fn pipeline(&self) -> &wgpu::RenderPipeline {
        self.pipeline_for_blend_mode(Moc3DrawableBlendMode::Normal)
    }

    pub fn pipeline_for_blend_mode(
        &self,
        blend_mode: Moc3DrawableBlendMode,
    ) -> &wgpu::RenderPipeline {
        match blend_mode {
            Moc3DrawableBlendMode::Normal => &self.normal_pipeline,
            Moc3DrawableBlendMode::Additive => &self.additive_pipeline,
            Moc3DrawableBlendMode::Multiplicative => &self.multiplicative_pipeline,
        }
    }

    pub fn mask_pipeline(&self) -> &wgpu::RenderPipeline {
        &self.mask_pipeline
    }

    pub fn masked_pipeline_for_blend_mode(
        &self,
        blend_mode: Moc3DrawableBlendMode,
    ) -> &wgpu::RenderPipeline {
        match blend_mode {
            Moc3DrawableBlendMode::Normal => &self.masked_normal_pipeline,
            Moc3DrawableBlendMode::Additive => &self.masked_additive_pipeline,
            Moc3DrawableBlendMode::Multiplicative => &self.masked_multiplicative_pipeline,
        }
    }

    pub fn texture_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.texture_bind_group_layout
    }

    pub fn transform_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.transform_bind_group_layout
    }

    pub fn mask_params_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.mask_params_bind_group_layout
    }

    pub fn clip_params_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.clip_params_bind_group_layout
    }

    pub fn identity_transform(&self) -> &WgpuTransform {
        &self.identity_transform
    }

    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    pub fn create_texture_bind_group(
        &self,
        device: &wgpu::Device,
        texture_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("live2d.texture.bind_group"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    pub fn create_transform(&self, device: &wgpu::Device, matrix: &Matrix44) -> WgpuTransform {
        create_wgpu_transform(device, &self.transform_bind_group_layout, matrix)
    }

    pub fn create_mask_params(
        &self,
        device: &wgpu::Device,
        layout: WgpuClippingLayout,
    ) -> WgpuMaskParams {
        create_wgpu_mask_params(device, &self.mask_params_bind_group_layout, layout)
    }

    pub fn create_clip_params(
        &self,
        device: &wgpu::Device,
        matrix: &Matrix44,
        channel: WgpuMaskChannel,
        inverted: bool,
    ) -> WgpuClipParams {
        create_wgpu_clip_params(
            device,
            &self.clip_params_bind_group_layout,
            matrix,
            channel,
            inverted,
        )
    }

    pub fn create_clipping_resources(
        &self,
        device: &wgpu::Device,
        plan: &WgpuClippingPlan,
    ) -> Result<WgpuClippingResources, WgpuClippingLayoutError> {
        let mut contexts = Vec::with_capacity(plan.contexts().len());
        let mut drawable_context_indices = Vec::<Option<usize>>::new();

        for (context_index, context) in plan.contexts().iter().enumerate() {
            let layout = context
                .layout()
                .ok_or(WgpuClippingLayoutError::MissingLayout { context_index })?;
            let mask_transform = self.create_transform(
                device,
                &context
                    .matrix_for_mask()
                    .ok_or(WgpuClippingLayoutError::MissingMaskMatrix { context_index })?,
            );
            let clip_params = self.create_clip_params(
                device,
                &context
                    .matrix_for_draw()
                    .ok_or(WgpuClippingLayoutError::MissingDrawMatrix { context_index })?,
                layout.channel(),
                context.inverted(),
            );
            let mask_params = self.create_mask_params(device, layout);
            let mask_drawable_indices = context
                .masks()
                .iter()
                .map(|&drawable_index| {
                    usize::try_from(drawable_index).map_err(|_| {
                        WgpuClippingLayoutError::InvalidMaskDrawableIndex { drawable_index }
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;

            contexts.push(WgpuPreparedClippingContext {
                mask_drawable_indices,
                drawable_indices: context.drawable_indices().to_vec(),
                mask_transform,
                mask_params,
                clip_params,
            });

            for &drawable_index in context.drawable_indices() {
                if drawable_context_indices.len() <= drawable_index {
                    drawable_context_indices.resize(drawable_index + 1, None);
                }
                drawable_context_indices[drawable_index] = Some(context_index);
            }
        }

        Ok(WgpuClippingResources {
            contexts,
            drawable_context_indices,
        })
    }

    pub fn update_clipping_resources(
        &self,
        queue: &wgpu::Queue,
        resources: &mut WgpuClippingResources,
        plan: &WgpuClippingPlan,
    ) -> Result<bool, WgpuClippingLayoutError> {
        if resources.contexts.len() != plan.contexts().len() {
            return Ok(false);
        }

        for (context_index, (resource, context)) in resources
            .contexts
            .iter_mut()
            .zip(plan.contexts())
            .enumerate()
        {
            if !mask_drawable_indices_match(&resource.mask_drawable_indices, context.masks())?
                || resource.drawable_indices != context.drawable_indices()
            {
                return Ok(false);
            }

            let layout = context
                .layout()
                .ok_or(WgpuClippingLayoutError::MissingLayout { context_index })?;
            let mask_matrix = context
                .matrix_for_mask()
                .ok_or(WgpuClippingLayoutError::MissingMaskMatrix { context_index })?;
            let draw_matrix = context
                .matrix_for_draw()
                .ok_or(WgpuClippingLayoutError::MissingDrawMatrix { context_index })?;
            resource.mask_transform.update_matrix(queue, &mask_matrix);
            resource.mask_params.update_layout(queue, layout);
            resource.clip_params.update_params(
                queue,
                &draw_matrix,
                layout.channel(),
                context.inverted(),
            );
        }

        Ok(true)
    }

    pub fn create_rgba8_texture(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<WgpuTexture, WgpuTextureError> {
        let expected = rgba8_len(width, height)?;
        if rgba.len() != expected {
            return Err(WgpuTextureError::InvalidRgbaLength {
                width,
                height,
                expected,
                actual: rgba.len(),
            });
        }

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("live2d.texture.rgba8"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        queue.write_texture(
            texture.as_image_copy(),
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.create_texture_bind_group(device, &view);

        Ok(WgpuTexture {
            texture,
            view,
            bind_group,
            width,
            height,
        })
    }

    pub fn create_mask_render_target(
        &self,
        device: &wgpu::Device,
        size: u32,
    ) -> Result<WgpuMaskRenderTarget, WgpuTextureError> {
        if size == 0 {
            return Err(WgpuTextureError::InvalidTextureSize {
                width: size,
                height: size,
            });
        }

        let extent = wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("live2d.mask.texture"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.create_texture_bind_group(device, &view);

        Ok(WgpuMaskRenderTarget {
            texture,
            view,
            bind_group,
            width: size,
            height: size,
        })
    }
}

fn create_live2d_pipeline(
    device: &wgpu::Device,
    pipeline_layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    color_format: wgpu::TextureFormat,
    blend_mode: Moc3DrawableBlendMode,
    label: &'static str,
) -> wgpu::RenderPipeline {
    create_textured_triangle_pipeline(
        device,
        pipeline_layout,
        shader,
        color_format,
        live2d_blend_state(blend_mode),
        label,
    )
}

fn create_live2d_mask_pipeline(
    device: &wgpu::Device,
    pipeline_layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    label: &'static str,
) -> wgpu::RenderPipeline {
    create_textured_triangle_pipeline(
        device,
        pipeline_layout,
        shader,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu_mask_blend_state(),
        label,
    )
}

fn create_textured_triangle_pipeline(
    device: &wgpu::Device,
    pipeline_layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    color_format: wgpu::TextureFormat,
    blend_state: wgpu::BlendState,
    label: &'static str,
) -> wgpu::RenderPipeline {
    let vertex_buffers = [Some(drawable_vertex_layout())];
    let color_targets = [Some(wgpu::ColorTargetState {
        format: color_format,
        blend: Some(blend_state),
        write_mask: wgpu::ColorWrites::ALL,
    })];

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &vertex_buffers,
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &color_targets,
        }),
        multiview_mask: None,
        cache: None,
    })
}
