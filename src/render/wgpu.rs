use wgpu::util::DeviceExt;

use crate::{
    core::Matrix44,
    moc3::{Moc3DrawableBlendMode, Moc3DrawableMesh, Moc3DrawableVertex},
};

pub const LIVE2D_WGSL: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) opacity: f32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) opacity: f32,
};

@group(0) @binding(0)
var live2d_texture: texture_2d<f32>;

@group(0) @binding(1)
var live2d_sampler: sampler;

@group(1) @binding(0)
var<uniform> live2d_transform: mat4x4<f32>;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = live2d_transform * vec4<f32>(input.position, 0.0, 1.0);
    output.uv = input.uv;
    output.opacity = input.opacity;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let sample = textureSample(live2d_texture, live2d_sampler, input.uv);
    let alpha = sample.a * input.opacity;
    return vec4<f32>(sample.rgb * alpha, alpha);
}
"#;

pub fn live2d_wgsl_source() -> &'static str {
    LIVE2D_WGSL
}

pub fn live2d_blend_state(blend_mode: Moc3DrawableBlendMode) -> wgpu::BlendState {
    match blend_mode {
        Moc3DrawableBlendMode::Normal => wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        },
        Moc3DrawableBlendMode::Additive => wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        },
        Moc3DrawableBlendMode::Multiplicative => wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Dst,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        },
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct WgpuDrawableVertex {
    position: [f32; 2],
    uv: [f32; 2],
    opacity: f32,
}

impl WgpuDrawableVertex {
    pub const STRIDE: wgpu::BufferAddress = 20;

    pub fn new(position: [f32; 2], uv: [f32; 2], opacity: f32) -> Self {
        Self {
            position,
            uv,
            opacity,
        }
    }

    pub fn position(&self) -> [f32; 2] {
        self.position
    }

    pub fn uv(&self) -> [f32; 2] {
        self.uv
    }

    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    pub fn buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 3] = [
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 8,
                shader_location: 1,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32,
                offset: 16,
                shader_location: 2,
            },
        ];

        wgpu::VertexBufferLayout {
            array_stride: Self::STRIDE,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES,
        }
    }
}

#[derive(Debug)]
pub struct WgpuDrawableBuffers {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    texture_index: i32,
    blend_mode: Moc3DrawableBlendMode,
    opacity: f32,
    draw_order: f32,
}

impl WgpuDrawableBuffers {
    pub fn vertex_buffer(&self) -> &wgpu::Buffer {
        &self.vertex_buffer
    }

    pub fn index_buffer(&self) -> &wgpu::Buffer {
        &self.index_buffer
    }

    pub fn index_count(&self) -> u32 {
        self.index_count
    }

    pub fn texture_index(&self) -> i32 {
        self.texture_index
    }

    pub fn blend_mode(&self) -> Moc3DrawableBlendMode {
        self.blend_mode
    }

    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    pub fn draw_order(&self) -> f32 {
        self.draw_order
    }
}

#[derive(Debug)]
pub struct WgpuMeshBuffers {
    drawables: Vec<WgpuDrawableBuffers>,
}

impl WgpuMeshBuffers {
    pub fn from_drawables(device: &wgpu::Device, meshes: &[Moc3DrawableMesh]) -> Option<Self> {
        let mut drawables = Vec::with_capacity(meshes.len());
        for mesh in meshes {
            drawables.push(create_wgpu_drawable_buffers(device, mesh)?);
        }

        Some(Self { drawables })
    }

    pub fn drawables(&self) -> &[WgpuDrawableBuffers] {
        &self.drawables
    }

    pub fn draw_order_indices(&self) -> Vec<usize> {
        let mut indices = (0..self.drawables.len()).collect::<Vec<_>>();
        indices.sort_by(|left, right| {
            self.drawables[*left]
                .draw_order
                .total_cmp(&self.drawables[*right].draw_order)
                .then_with(|| left.cmp(right))
        });
        indices
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WgpuRenderError {
    InvalidTextureIndex { texture_index: i32 },
    MissingTexture { texture_index: i32 },
}

impl std::fmt::Display for WgpuRenderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTextureIndex { texture_index } => {
                write!(formatter, "invalid texture index {texture_index}")
            }
            Self::MissingTexture { texture_index } => {
                write!(formatter, "missing texture bind group {texture_index}")
            }
        }
    }
}

impl std::error::Error for WgpuRenderError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WgpuTextureError {
    InvalidTextureSize {
        width: u32,
        height: u32,
    },
    InvalidRgbaLength {
        width: u32,
        height: u32,
        expected: usize,
        actual: usize,
    },
}

impl std::fmt::Display for WgpuTextureError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTextureSize { width, height } => {
                write!(formatter, "invalid texture size {width}x{height}")
            }
            Self::InvalidRgbaLength {
                width,
                height,
                expected,
                actual,
            } => write!(
                formatter,
                "invalid rgba8 texture length for {width}x{height}: expected {expected}, got {actual}"
            ),
        }
    }
}

impl std::error::Error for WgpuTextureError {}

#[derive(Debug)]
pub struct WgpuTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
}

impl WgpuTexture {
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}

#[derive(Debug)]
pub struct WgpuTransform {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl WgpuTransform {
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

#[derive(Debug)]
pub struct WgpuLive2dRenderer {
    normal_pipeline: wgpu::RenderPipeline,
    additive_pipeline: wgpu::RenderPipeline,
    multiplicative_pipeline: wgpu::RenderPipeline,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    transform_bind_group_layout: wgpu::BindGroupLayout,
    identity_transform: WgpuTransform,
    sampler: wgpu::Sampler,
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
        let identity_transform =
            create_wgpu_transform(device, &transform_bind_group_layout, &Matrix44::identity());

        Self {
            normal_pipeline,
            additive_pipeline,
            multiplicative_pipeline,
            texture_bind_group_layout,
            transform_bind_group_layout,
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

    pub fn texture_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.texture_bind_group_layout
    }

    pub fn transform_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.transform_bind_group_layout
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
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
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

    pub fn draw(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh_buffers: &WgpuMeshBuffers,
        texture_bind_groups: &[wgpu::BindGroup],
    ) -> Result<u32, WgpuRenderError> {
        self.draw_with_bind_groups_and_transform(
            render_pass,
            mesh_buffers,
            texture_bind_groups,
            &self.identity_transform,
        )
    }

    pub fn draw_with_bind_groups_and_transform(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh_buffers: &WgpuMeshBuffers,
        texture_bind_groups: &[wgpu::BindGroup],
        transform: &WgpuTransform,
    ) -> Result<u32, WgpuRenderError> {
        self.draw_with_bind_group_provider(render_pass, mesh_buffers, transform, |texture_index| {
            texture_bind_group_at(texture_bind_groups, texture_index)
        })
    }

    pub fn draw_with_textures(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh_buffers: &WgpuMeshBuffers,
        textures: &[WgpuTexture],
    ) -> Result<u32, WgpuRenderError> {
        self.draw_with_textures_and_transform(
            render_pass,
            mesh_buffers,
            textures,
            &self.identity_transform,
        )
    }

    pub fn draw_with_textures_and_transform(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh_buffers: &WgpuMeshBuffers,
        textures: &[WgpuTexture],
        transform: &WgpuTransform,
    ) -> Result<u32, WgpuRenderError> {
        self.draw_with_bind_group_provider(render_pass, mesh_buffers, transform, |texture_index| {
            texture_bind_group_from_textures(textures, texture_index)
        })
    }

    fn draw_with_bind_group_provider<'a>(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh_buffers: &WgpuMeshBuffers,
        transform: &WgpuTransform,
        mut bind_group_for_texture: impl FnMut(i32) -> Result<&'a wgpu::BindGroup, WgpuRenderError>,
    ) -> Result<u32, WgpuRenderError> {
        let mut drawn = 0;
        for drawable_index in mesh_buffers.draw_order_indices() {
            let drawable = &mesh_buffers.drawables[drawable_index];
            let texture_bind_group = bind_group_for_texture(drawable.texture_index)?;

            render_pass.set_pipeline(self.pipeline_for_blend_mode(drawable.blend_mode));
            render_pass.set_bind_group(0, texture_bind_group, &[]);
            render_pass.set_bind_group(1, transform.bind_group(), &[]);
            render_pass.set_vertex_buffer(0, drawable.vertex_buffer.slice(..));
            render_pass
                .set_index_buffer(drawable.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..drawable.index_count, 0, 0..1);
            drawn += 1;
        }

        Ok(drawn)
    }
}

pub fn wgpu_vertices_from_drawable(mesh: &Moc3DrawableMesh) -> Vec<WgpuDrawableVertex> {
    mesh.vertices()
        .iter()
        .map(|vertex| wgpu_vertex_from_drawable_vertex(vertex, mesh.opacity()))
        .collect()
}

pub fn wgpu_vertex_from_drawable_vertex(
    vertex: &Moc3DrawableVertex,
    opacity: f32,
) -> WgpuDrawableVertex {
    WgpuDrawableVertex::new(vertex.position(), vertex.uv(), opacity)
}

pub fn encode_wgpu_vertices(vertices: &[WgpuDrawableVertex]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vertices.len() * WgpuDrawableVertex::STRIDE as usize);
    for vertex in vertices {
        bytes.extend_from_slice(&vertex.position[0].to_ne_bytes());
        bytes.extend_from_slice(&vertex.position[1].to_ne_bytes());
        bytes.extend_from_slice(&vertex.uv[0].to_ne_bytes());
        bytes.extend_from_slice(&vertex.uv[1].to_ne_bytes());
        bytes.extend_from_slice(&vertex.opacity.to_ne_bytes());
    }

    bytes
}

pub fn encode_wgpu_indices(indices: &[u16]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(indices.len() * 2);
    for index in indices {
        bytes.extend_from_slice(&index.to_ne_bytes());
    }

    bytes
}

pub fn encode_wgpu_matrix(matrix: &Matrix44) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(64);
    for value in matrix.as_slice() {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }

    bytes
}

pub fn create_wgpu_drawable_buffers(
    device: &wgpu::Device,
    mesh: &Moc3DrawableMesh,
) -> Option<WgpuDrawableBuffers> {
    let vertices = wgpu_vertices_from_drawable(mesh);
    let vertex_bytes = encode_wgpu_vertices(&vertices);
    let index_bytes = encode_wgpu_indices(mesh.indices());
    let index_count = u32::try_from(mesh.indices().len()).ok()?;

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("live2d.drawable.vertices"),
        contents: &vertex_bytes,
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("live2d.drawable.indices"),
        contents: &index_bytes,
        usage: wgpu::BufferUsages::INDEX,
    });

    Some(WgpuDrawableBuffers {
        vertex_buffer,
        index_buffer,
        index_count,
        texture_index: mesh.texture_index(),
        blend_mode: mesh.blend_mode(),
        opacity: mesh.opacity(),
        draw_order: mesh.draw_order(),
    })
}

fn create_wgpu_transform(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    matrix: &Matrix44,
) -> WgpuTransform {
    let matrix_bytes = encode_wgpu_matrix(matrix);
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("live2d.transform.uniform"),
        contents: &matrix_bytes,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("live2d.transform.bind_group"),
        layout: bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    });

    WgpuTransform { buffer, bind_group }
}

fn rgba8_len(width: u32, height: u32) -> Result<usize, WgpuTextureError> {
    if width == 0 || height == 0 {
        return Err(WgpuTextureError::InvalidTextureSize { width, height });
    }

    let len = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(WgpuTextureError::InvalidTextureSize { width, height })?;

    Ok(len)
}

fn create_live2d_pipeline(
    device: &wgpu::Device,
    pipeline_layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    color_format: wgpu::TextureFormat,
    blend_mode: Moc3DrawableBlendMode,
    label: &'static str,
) -> wgpu::RenderPipeline {
    let vertex_buffers = [WgpuDrawableVertex::buffer_layout()];
    let color_targets = [Some(wgpu::ColorTargetState {
        format: color_format,
        blend: Some(live2d_blend_state(blend_mode)),
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

fn texture_bind_group_at(
    texture_bind_groups: &[wgpu::BindGroup],
    texture_index: i32,
) -> Result<&wgpu::BindGroup, WgpuRenderError> {
    let texture_index_usize = usize::try_from(texture_index)
        .map_err(|_| WgpuRenderError::InvalidTextureIndex { texture_index })?;
    texture_bind_groups
        .get(texture_index_usize)
        .ok_or(WgpuRenderError::MissingTexture { texture_index })
}

fn texture_bind_group_from_textures(
    textures: &[WgpuTexture],
    texture_index: i32,
) -> Result<&wgpu::BindGroup, WgpuRenderError> {
    let texture_index_usize = usize::try_from(texture_index)
        .map_err(|_| WgpuRenderError::InvalidTextureIndex { texture_index })?;
    textures
        .get(texture_index_usize)
        .map(WgpuTexture::bind_group)
        .ok_or(WgpuRenderError::MissingTexture { texture_index })
}
