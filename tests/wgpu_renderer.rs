use rusty_live2d::{
    core::Matrix44,
    moc3::{Moc3DrawableBlendMode, Moc3DrawableMesh, Moc3DrawableVertex},
    render::wgpu::{
        WgpuClippingLayoutError, WgpuClippingPlan, WgpuClippingRect, WgpuDrawableVertex,
        WgpuLive2dRenderer, WgpuMaskChannel, WgpuMeshBuffers, WgpuRenderError, WgpuTextureError,
        encode_wgpu_indices, encode_wgpu_mask_params, encode_wgpu_matrix, encode_wgpu_vertices,
        live2d_blend_state, live2d_masked_wgsl_source, live2d_wgsl_source, mask_wgsl_source,
        wgpu_mask_blend_state, wgpu_vertices_from_drawable,
    },
};

#[test]
fn encodes_wgpu_vertices_and_indices() {
    let mesh = Moc3DrawableMesh::from_parts(
        3,
        4,
        0.75,
        20.0,
        vec![
            Moc3DrawableVertex::new([1.0, 2.0], [0.25, 0.5]),
            Moc3DrawableVertex::new([3.0, 4.0], [0.75, 1.0]),
        ],
        vec![0, 1],
        vec![7],
    );

    let vertices = wgpu_vertices_from_drawable(&mesh);
    let vertex_bytes = encode_wgpu_vertices(&vertices);
    let index_bytes = encode_wgpu_indices(mesh.indices());

    assert_eq!(
        vertices,
        vec![
            WgpuDrawableVertex::new([1.0, 2.0], [0.25, 0.5], 0.75),
            WgpuDrawableVertex::new([3.0, 4.0], [0.75, 1.0], 0.75),
        ]
    );
    assert_eq!(vertex_bytes.len(), 40);
    assert_eq!(&vertex_bytes[0..4], &1.0f32.to_ne_bytes());
    assert_eq!(&vertex_bytes[12..16], &0.5f32.to_ne_bytes());
    assert_eq!(&vertex_bytes[16..20], &0.75f32.to_ne_bytes());
    assert_eq!(index_bytes, vec![0, 0, 1, 0]);
}

#[test]
fn live2d_wgsl_samples_texture_and_applies_opacity() {
    let source = live2d_wgsl_source();
    let shader_file = std::fs::read_to_string("src/render/shaders/live2d.wgsl").unwrap();

    assert_eq!(source, shader_file);
    assert!(source.contains("@location(0) position: vec2<f32>"));
    assert!(source.contains("@location(1) uv: vec2<f32>"));
    assert!(source.contains("@location(2) opacity: f32"));
    assert!(source.contains("@group(1) @binding(0)"));
    assert!(source.contains("live2d_transform * vec4<f32>(input.position, 0.0, 1.0)"));
    assert!(source.contains("textureSample"));
    assert!(source.contains("let alpha = sample.a * input.opacity"));
    assert!(source.contains("vec4<f32>(sample.rgb * alpha, alpha)"));
}

#[test]
fn mask_wgsl_uses_external_file_and_channel_params() {
    let source = mask_wgsl_source();
    let shader_file = std::fs::read_to_string("src/render/shaders/mask.wgsl").unwrap();

    assert_eq!(source, shader_file);
    assert!(source.contains("@group(2) @binding(0)"));
    assert!(source.contains("channel_flag: vec4<f32>"));
    assert!(source.contains("base_rect: vec4<f32>"));
    assert!(source.contains("step(mask_params.base_rect.x, pos.x)"));
    assert!(source.contains("textureSample(live2d_texture, live2d_sampler, input.uv).a"));
    assert!(source.contains("return mask_params.channel_flag * source_alpha"));
}

#[test]
fn live2d_masked_wgsl_samples_inverse_mask_channel() {
    let source = live2d_masked_wgsl_source();
    let shader_file = std::fs::read_to_string("src/render/shaders/live2d_masked.wgsl").unwrap();

    assert_eq!(source, shader_file);
    assert!(source.contains("@group(2) @binding(0)"));
    assert!(source.contains("@group(3) @binding(0)"));
    assert!(source.contains("clip_matrix: mat4x4<f32>"));
    assert!(source.contains("channel_flag: vec4<f32>"));
    assert!(source.contains("clip_params.clip_matrix * position"));
    assert!(source.contains("vec4<f32>(sample.rgb * alpha, alpha)"));
    assert!(source.contains("dot(vec4<f32>(1.0) - mask_sample, clip_params.channel_flag)"));
}

#[test]
fn encodes_wgpu_transform_matrix() {
    let mut matrix = Matrix44::identity();
    matrix.scale(2.0, 3.0);
    matrix.translate(4.0, 5.0);

    let bytes = encode_wgpu_matrix(&matrix);

    assert_eq!(bytes.len(), 64);
    assert_eq!(&bytes[0..4], &2.0f32.to_ne_bytes());
    assert_eq!(&bytes[20..24], &3.0f32.to_ne_bytes());
    assert_eq!(&bytes[48..52], &4.0f32.to_ne_bytes());
    assert_eq!(&bytes[52..56], &5.0f32.to_ne_bytes());
}

#[test]
fn encodes_mask_params_from_layout_channel_and_bounds() {
    let layout = rusty_live2d::render::wgpu::WgpuClippingLayout::new(
        WgpuMaskChannel::Green,
        WgpuClippingRect::new(0.5, 0.0, 0.5, 1.0),
    );

    let bytes = encode_wgpu_mask_params(layout);

    assert_eq!(bytes.len(), 32);
    assert_eq!(&bytes[0..4], &0.0f32.to_ne_bytes());
    assert_eq!(&bytes[4..8], &1.0f32.to_ne_bytes());
    assert_eq!(&bytes[8..12], &0.0f32.to_ne_bytes());
    assert_eq!(&bytes[12..16], &0.0f32.to_ne_bytes());
    assert_eq!(&bytes[16..20], &0.0f32.to_ne_bytes());
    assert_eq!(&bytes[20..24], &(-1.0f32).to_ne_bytes());
    assert_eq!(&bytes[24..28], &1.0f32.to_ne_bytes());
    assert_eq!(&bytes[28..32], &1.0f32.to_ne_bytes());
}

#[test]
fn creates_mask_params_bind_group() {
    let (device, _queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let renderer = WgpuLive2dRenderer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let layout = rusty_live2d::render::wgpu::WgpuClippingLayout::new(
        WgpuMaskChannel::Red,
        WgpuClippingRect::new(0.0, 0.0, 1.0, 1.0),
    );

    let params = renderer.create_mask_params(&device, layout);

    let _ = params.buffer();
    let _ = params.bind_group();
    let _ = renderer.mask_params_bind_group_layout();
}

#[test]
fn exposes_live2d_premultiplied_blend_states() {
    let normal = live2d_blend_state(Moc3DrawableBlendMode::Normal);
    assert_eq!(normal.color.src_factor, wgpu::BlendFactor::One);
    assert_eq!(normal.color.dst_factor, wgpu::BlendFactor::OneMinusSrcAlpha);
    assert_eq!(normal.alpha.src_factor, wgpu::BlendFactor::One);
    assert_eq!(normal.alpha.dst_factor, wgpu::BlendFactor::OneMinusSrcAlpha);

    let additive = live2d_blend_state(Moc3DrawableBlendMode::Additive);
    assert_eq!(additive.color.src_factor, wgpu::BlendFactor::One);
    assert_eq!(additive.color.dst_factor, wgpu::BlendFactor::One);
    assert_eq!(additive.alpha.src_factor, wgpu::BlendFactor::Zero);
    assert_eq!(additive.alpha.dst_factor, wgpu::BlendFactor::One);

    let multiplicative = live2d_blend_state(Moc3DrawableBlendMode::Multiplicative);
    assert_eq!(multiplicative.color.src_factor, wgpu::BlendFactor::Dst);
    assert_eq!(
        multiplicative.color.dst_factor,
        wgpu::BlendFactor::OneMinusSrcAlpha
    );
    assert_eq!(multiplicative.alpha.src_factor, wgpu::BlendFactor::Zero);
    assert_eq!(multiplicative.alpha.dst_factor, wgpu::BlendFactor::One);
}

#[test]
fn exposes_inverse_mask_blend_state() {
    let blend = wgpu_mask_blend_state();

    assert_eq!(blend.color.src_factor, wgpu::BlendFactor::Zero);
    assert_eq!(blend.color.dst_factor, wgpu::BlendFactor::OneMinusSrc);
    assert_eq!(blend.alpha.src_factor, wgpu::BlendFactor::Zero);
    assert_eq!(blend.alpha.dst_factor, wgpu::BlendFactor::OneMinusSrcAlpha);
}

#[test]
fn creates_pipeline_and_encodes_draw_calls() {
    let (device, _queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let renderer = WgpuLive2dRenderer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);

    let mesh = Moc3DrawableMesh::from_parts(
        0,
        0,
        1.0,
        10.0,
        vec![
            Moc3DrawableVertex::new([-0.5, -0.5], [0.0, 1.0]),
            Moc3DrawableVertex::new([0.5, -0.5], [1.0, 1.0]),
            Moc3DrawableVertex::new([0.0, 0.5], [0.5, 0.0]),
        ],
        vec![0, 1, 2],
        vec![],
    );
    let buffers = WgpuMeshBuffers::from_drawables(&device, &[mesh]).unwrap();

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("live2d.test.texture"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bind_group = renderer.create_texture_bind_group(&device, &texture_view);

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("live2d.test.target"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("live2d.test.encoder"),
    });

    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("live2d.test.pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        let drawn = renderer.draw(&mut pass, &buffers, &[bind_group]).unwrap();
        assert_eq!(drawn, 1);
    }

    let _ = encoder.finish();
}

#[test]
fn creates_mask_pipeline_and_encodes_mask_draw_call() {
    let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let renderer = WgpuLive2dRenderer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let texture = renderer
        .create_rgba8_texture(&device, &queue, 1, 1, &[255, 255, 255, 255])
        .unwrap();
    let transform = renderer.create_transform(&device, &Matrix44::identity());
    let params = renderer.create_mask_params(
        &device,
        rusty_live2d::render::wgpu::WgpuClippingLayout::new(
            WgpuMaskChannel::Red,
            WgpuClippingRect::new(0.0, 0.0, 1.0, 1.0),
        ),
    );
    let mask_target = renderer.create_mask_render_target(&device, 16).unwrap();
    let mesh = test_mesh_with_draw_order(0, 0.0);
    let buffers = WgpuMeshBuffers::from_drawables(&device, &[mesh]).unwrap();
    let drawable = &buffers.drawables()[0];
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("live2d.test.mask_pipeline_encoder"),
    });

    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("live2d.test.mask_pipeline_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: mask_target.view(),
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        pass.set_pipeline(renderer.mask_pipeline());
        pass.set_bind_group(0, texture.bind_group(), &[]);
        pass.set_bind_group(1, transform.bind_group(), &[]);
        pass.set_bind_group(2, params.bind_group(), &[]);
        pass.set_vertex_buffer(0, drawable.vertex_buffer().slice(..));
        pass.set_index_buffer(drawable.index_buffer().slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..drawable.index_count(), 0, 0..1);
    }

    let _ = encoder.finish();
}

#[test]
fn draws_prepared_mask_contexts_into_mask_target() {
    let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let renderer = WgpuLive2dRenderer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let texture = renderer
        .create_rgba8_texture(&device, &queue, 1, 1, &[255, 255, 255, 255])
        .unwrap();
    let clipped = test_mesh_with_masks(0, 0.0, vec![1]);
    let mask = test_mesh_with_draw_order(0, 1.0);
    let buffers = WgpuMeshBuffers::from_drawables(&device, &[clipped, mask]).unwrap();
    let mut plan = WgpuClippingPlan::from_mesh_buffers(&buffers);
    plan.prepare_single_texture_masks(&buffers).unwrap();
    let clipping_resources = renderer.create_clipping_resources(&device, &plan).unwrap();
    assert_eq!(clipping_resources.contexts().len(), 1);

    let mask_target = renderer.create_mask_render_target(&device, 16).unwrap();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("live2d.test.draw_masks_encoder"),
    });

    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("live2d.test.draw_masks_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: mask_target.view(),
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        let drawn = renderer
            .draw_masks_with_textures(&mut pass, &buffers, &clipping_resources, &[texture])
            .unwrap();
        assert_eq!(drawn, 1);
    }

    let _ = encoder.finish();
}

#[test]
fn mesh_buffers_expose_stable_draw_order_indices() {
    let (device, _queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let meshes = [
        test_mesh_with_draw_order(0, 30.0),
        test_mesh_with_draw_order(1, 10.0),
        test_mesh_with_draw_order(2, 10.0),
    ];
    let buffers = WgpuMeshBuffers::from_drawables(&device, &meshes).unwrap();

    assert_eq!(buffers.draw_order_indices(), vec![1, 2, 0]);
}

#[test]
fn draw_returns_error_for_missing_texture_bind_group() {
    let (device, _queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let renderer = WgpuLive2dRenderer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let mesh = test_mesh_with_draw_order(2, 0.0);
    let buffers = WgpuMeshBuffers::from_drawables(&device, &[mesh]).unwrap();
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("live2d.test.missing_texture_target"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("live2d.test.missing_texture_encoder"),
    });

    let error = {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("live2d.test.missing_texture_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        renderer.draw(&mut pass, &buffers, &[]).unwrap_err()
    };

    assert_eq!(error, WgpuRenderError::MissingTexture { texture_index: 2 });
}

#[test]
fn draw_returns_error_for_masked_drawable_until_clipping_is_available() {
    let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let renderer = WgpuLive2dRenderer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let texture = renderer
        .create_rgba8_texture(&device, &queue, 1, 1, &[255, 255, 255, 255])
        .unwrap();
    let mesh = test_mesh_with_masks(0, 0.0, vec![3, 4]);
    let buffers = WgpuMeshBuffers::from_drawables(&device, &[mesh]).unwrap();

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("live2d.test.masked_target"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("live2d.test.masked_encoder"),
    });

    let error = {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("live2d.test.masked_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        renderer
            .draw_with_textures(&mut pass, &buffers, &[texture])
            .unwrap_err()
    };

    assert_eq!(
        error,
        WgpuRenderError::UnsupportedClippingMasks {
            drawable_index: 0,
            mask_count: 2
        }
    );
}

#[test]
fn builds_clipping_plan_from_masked_drawables() {
    let (device, _queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let meshes = [
        test_mesh_with_masks(0, 0.0, vec![1, 2]),
        test_mesh_with_draw_order(0, 1.0),
        test_mesh_with_masks(0, 2.0, vec![1, 2]),
        test_mesh_with_masks(0, 3.0, vec![3]),
    ];
    let buffers = WgpuMeshBuffers::from_drawables(&device, &meshes).unwrap();

    let plan = WgpuClippingPlan::from_mesh_buffers(&buffers);

    assert_eq!(plan.unmasked_drawable_indices(), &[1]);
    assert_eq!(plan.contexts().len(), 2);
    assert_eq!(plan.contexts()[0].masks(), &[1, 2]);
    assert_eq!(plan.contexts()[0].drawable_indices(), &[0, 2]);
    assert_eq!(plan.contexts()[1].masks(), &[3]);
    assert_eq!(plan.contexts()[1].drawable_indices(), &[3]);
}

#[test]
fn merges_clipping_contexts_with_same_mask_set_regardless_of_order() {
    let (device, _queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let meshes = [
        test_mesh_with_masks(0, 0.0, vec![1, 2]),
        test_mesh_with_masks(0, 1.0, vec![2, 1]),
    ];
    let buffers = WgpuMeshBuffers::from_drawables(&device, &meshes).unwrap();

    let plan = WgpuClippingPlan::from_mesh_buffers(&buffers);

    assert_eq!(plan.contexts().len(), 1);
    assert_eq!(plan.contexts()[0].masks(), &[1, 2]);
    assert_eq!(plan.contexts()[0].drawable_indices(), &[0, 1]);
}

#[test]
fn assigns_single_texture_clipping_layouts_by_channel_and_cell() {
    let (device, _queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let meshes = [
        test_mesh_with_masks(0, 0.0, vec![10]),
        test_mesh_with_masks(0, 1.0, vec![11]),
        test_mesh_with_masks(0, 2.0, vec![12]),
        test_mesh_with_masks(0, 3.0, vec![13]),
        test_mesh_with_masks(0, 4.0, vec![14]),
    ];
    let buffers = WgpuMeshBuffers::from_drawables(&device, &meshes).unwrap();
    let mut plan = WgpuClippingPlan::from_mesh_buffers(&buffers);

    plan.assign_single_texture_layouts().unwrap();

    assert_eq!(
        plan.contexts()[0].layout().unwrap().channel(),
        WgpuMaskChannel::Red
    );
    assert_eq!(
        plan.contexts()[0].layout().unwrap().bounds(),
        WgpuClippingRect::new(0.0, 0.0, 0.5, 1.0)
    );
    assert_eq!(
        plan.contexts()[1].layout().unwrap().channel(),
        WgpuMaskChannel::Red
    );
    assert_eq!(
        plan.contexts()[1].layout().unwrap().bounds(),
        WgpuClippingRect::new(0.5, 0.0, 0.5, 1.0)
    );
    assert_eq!(
        plan.contexts()[2].layout().unwrap().channel(),
        WgpuMaskChannel::Green
    );
    assert_eq!(
        plan.contexts()[2].layout().unwrap().bounds(),
        WgpuClippingRect::new(0.0, 0.0, 1.0, 1.0)
    );
    assert_eq!(
        plan.contexts()[4].layout().unwrap().channel_flag(),
        [0.0, 0.0, 0.0, 1.0]
    );
}

#[test]
fn rejects_more_than_single_texture_clipping_layout_capacity() {
    let (device, _queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let meshes = (0..37)
        .map(|index| test_mesh_with_masks(0, index as f32, vec![index]))
        .collect::<Vec<_>>();
    let buffers = WgpuMeshBuffers::from_drawables(&device, &meshes).unwrap();
    let mut plan = WgpuClippingPlan::from_mesh_buffers(&buffers);

    let error = plan.assign_single_texture_layouts().unwrap_err();

    assert_eq!(
        error,
        WgpuClippingLayoutError::TooManyMasksForSingleTexture { mask_count: 37 }
    );
}

#[test]
fn prepares_clipping_bounds_and_matrices_from_clipped_drawables() {
    let (device, _queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let clipped = Moc3DrawableMesh::from_parts(
        0,
        0,
        1.0,
        0.0,
        vec![
            Moc3DrawableVertex::new([-1.0, -2.0], [0.0, 0.0]),
            Moc3DrawableVertex::new([3.0, -2.0], [1.0, 0.0]),
            Moc3DrawableVertex::new([3.0, 4.0], [1.0, 1.0]),
        ],
        vec![0, 1, 2],
        vec![1],
    );
    let mask = test_mesh_with_draw_order(0, 1.0);
    let buffers = WgpuMeshBuffers::from_drawables(&device, &[clipped, mask]).unwrap();
    let mut plan = WgpuClippingPlan::from_mesh_buffers(&buffers);

    plan.prepare_single_texture_masks(&buffers).unwrap();

    let context = &plan.contexts()[0];
    assert_rect_close(
        context.all_clipped_draw_rect().unwrap(),
        WgpuClippingRect::new(-1.2, -2.3, 4.4, 6.6),
    );

    let draw_matrix = context.matrix_for_draw().unwrap();
    assert_f32_close(draw_matrix.transform_x(-1.2), 0.0);
    assert_f32_close(draw_matrix.transform_x(3.2), 1.0);
    assert_f32_close(draw_matrix.transform_y(-2.3), 0.0);
    assert_f32_close(draw_matrix.transform_y(4.3), 1.0);

    let mask_matrix = context.matrix_for_mask().unwrap();
    assert_f32_close(mask_matrix.transform_x(-1.2), -1.0);
    assert_f32_close(mask_matrix.transform_x(3.2), 1.0);
    assert_f32_close(mask_matrix.transform_y(-2.3), -1.0);
    assert_f32_close(mask_matrix.transform_y(4.3), 1.0);
}

#[test]
fn creates_rgba8_texture_with_bind_group() {
    let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let renderer = WgpuLive2dRenderer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);

    let texture = renderer
        .create_rgba8_texture(
            &device,
            &queue,
            2,
            2,
            &[
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
        )
        .unwrap();

    assert_eq!(texture.width(), 2);
    assert_eq!(texture.height(), 2);
    let _ = texture.texture();
    let _ = texture.view();
    let _ = texture.bind_group();
}

#[test]
fn creates_mask_render_target_that_can_be_cleared() {
    let (device, _queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let renderer = WgpuLive2dRenderer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);

    let mask = renderer.create_mask_render_target(&device, 256).unwrap();

    assert_eq!(mask.width(), 256);
    assert_eq!(mask.height(), 256);
    let _ = mask.texture();
    let _ = mask.bind_group();

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("live2d.test.mask_target_encoder"),
    });
    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("live2d.test.mask_target_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: mask.view(),
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    }

    let _ = encoder.finish();
}

#[test]
fn rejects_zero_sized_mask_render_target() {
    let (device, _queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let renderer = WgpuLive2dRenderer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);

    let error = renderer.create_mask_render_target(&device, 0).unwrap_err();

    assert_eq!(
        error,
        WgpuTextureError::InvalidTextureSize {
            width: 0,
            height: 0
        }
    );
}

#[test]
fn draws_with_uploaded_textures() {
    let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let renderer = WgpuLive2dRenderer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let texture = renderer
        .create_rgba8_texture(&device, &queue, 1, 1, &[255, 255, 255, 255])
        .unwrap();
    let mesh = test_mesh_with_draw_order(0, 0.0);
    let buffers = WgpuMeshBuffers::from_drawables(&device, &[mesh]).unwrap();

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("live2d.test.uploaded_texture_target"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("live2d.test.uploaded_texture_encoder"),
    });

    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("live2d.test.uploaded_texture_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        let drawn = renderer
            .draw_with_textures(&mut pass, &buffers, &[texture])
            .unwrap();
        assert_eq!(drawn, 1);
    }

    let _ = encoder.finish();
}

#[test]
fn draws_with_uploaded_textures_and_transform() {
    let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let renderer = WgpuLive2dRenderer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let texture = renderer
        .create_rgba8_texture(&device, &queue, 1, 1, &[255, 255, 255, 255])
        .unwrap();
    let mut matrix = Matrix44::identity();
    matrix.scale(0.5, 0.5);
    let transform = renderer.create_transform(&device, &matrix);
    let mesh = test_mesh_with_draw_order(0, 0.0);
    let buffers = WgpuMeshBuffers::from_drawables(&device, &[mesh]).unwrap();

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("live2d.test.transform_target"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("live2d.test.transform_encoder"),
    });

    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("live2d.test.transform_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        let drawn = renderer
            .draw_with_textures_and_transform(&mut pass, &buffers, &[texture], &transform)
            .unwrap();
        assert_eq!(drawn, 1);
    }

    let _ = encoder.finish();
}

#[test]
fn draws_additive_and_multiplicative_drawables() {
    let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let renderer = WgpuLive2dRenderer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let texture = renderer
        .create_rgba8_texture(&device, &queue, 1, 1, &[255, 255, 255, 255])
        .unwrap();
    let meshes = [
        test_mesh_with_flags(0, 1 << 0, 0.0),
        test_mesh_with_flags(0, 1 << 1, 1.0),
    ];
    let buffers = WgpuMeshBuffers::from_drawables(&device, &meshes).unwrap();
    assert_eq!(
        buffers.drawables()[0].blend_mode(),
        Moc3DrawableBlendMode::Additive
    );
    assert_eq!(
        buffers.drawables()[1].blend_mode(),
        Moc3DrawableBlendMode::Multiplicative
    );

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("live2d.test.blend_pipeline_target"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("live2d.test.blend_pipeline_encoder"),
    });

    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("live2d.test.blend_pipeline_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        let drawn = renderer
            .draw_with_textures(&mut pass, &buffers, &[texture])
            .unwrap();
        assert_eq!(drawn, 2);
    }

    let _ = encoder.finish();
}

#[test]
fn rejects_rgba8_texture_with_wrong_byte_len() {
    let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let renderer = WgpuLive2dRenderer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);

    let error = renderer
        .create_rgba8_texture(&device, &queue, 2, 2, &[0; 15])
        .unwrap_err();

    assert_eq!(
        error,
        WgpuTextureError::InvalidRgbaLength {
            width: 2,
            height: 2,
            expected: 16,
            actual: 15
        }
    );
}

fn test_mesh_with_draw_order(texture_index: u8, draw_order: f32) -> Moc3DrawableMesh {
    test_mesh_with_flags(texture_index, 0, draw_order)
}

fn test_mesh_with_flags(
    texture_index: u8,
    drawable_flags: u8,
    draw_order: f32,
) -> Moc3DrawableMesh {
    test_mesh(texture_index, drawable_flags, draw_order, vec![])
}

fn test_mesh_with_masks(texture_index: u8, draw_order: f32, masks: Vec<i32>) -> Moc3DrawableMesh {
    test_mesh(texture_index, 0, draw_order, masks)
}

fn test_mesh(
    texture_index: u8,
    drawable_flags: u8,
    draw_order: f32,
    masks: Vec<i32>,
) -> Moc3DrawableMesh {
    Moc3DrawableMesh::from_parts(
        i32::from(texture_index),
        drawable_flags,
        1.0,
        draw_order,
        vec![
            Moc3DrawableVertex::new([-0.5, -0.5], [0.0, 1.0]),
            Moc3DrawableVertex::new([0.5, -0.5], [1.0, 1.0]),
            Moc3DrawableVertex::new([0.0, 0.5], [0.5, 0.0]),
        ],
        vec![0, 1, 2],
        masks,
    )
}

fn assert_rect_close(actual: WgpuClippingRect, expected: WgpuClippingRect) {
    assert_f32_close(actual.x(), expected.x());
    assert_f32_close(actual.y(), expected.y());
    assert_f32_close(actual.width(), expected.width());
    assert_f32_close(actual.height(), expected.height());
}

fn assert_f32_close(actual: f32, expected: f32) {
    let difference = (actual - expected).abs();
    assert!(
        difference <= 0.00001,
        "expected {actual} to be within 0.00001 of {expected}, difference {difference}"
    );
}
