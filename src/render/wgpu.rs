use wgpu::util::DeviceExt;

use crate::moc3::{Moc3DrawableMesh, Moc3DrawableVertex};

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

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.uv = input.uv;
    output.opacity = input.opacity;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let sample = textureSample(live2d_texture, live2d_sampler, input.uv);
    return vec4<f32>(sample.rgb, sample.a * input.opacity);
}
"#;

pub fn live2d_wgsl_source() -> &'static str {
    LIVE2D_WGSL
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

#[derive(Debug)]
pub struct WgpuLive2dRenderer {
    pipeline: wgpu::RenderPipeline,
    texture_bind_group_layout: wgpu::BindGroupLayout,
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
        let bind_group_layouts = [Some(&texture_bind_group_layout)];
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("live2d.pipeline.layout"),
            bind_group_layouts: &bind_group_layouts,
            immediate_size: 0,
        });
        let vertex_buffers = [WgpuDrawableVertex::buffer_layout()];
        let color_targets = [Some(wgpu::ColorTargetState {
            format: color_format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("live2d.pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
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
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &color_targets,
            }),
            multiview_mask: None,
            cache: None,
        });

        Self {
            pipeline,
            texture_bind_group_layout,
            sampler,
        }
    }

    pub fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }

    pub fn texture_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.texture_bind_group_layout
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

    pub fn draw(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh_buffers: &WgpuMeshBuffers,
        texture_bind_groups: &[wgpu::BindGroup],
    ) -> Result<u32, WgpuRenderError> {
        render_pass.set_pipeline(&self.pipeline);

        let mut drawn = 0;
        for drawable_index in mesh_buffers.draw_order_indices() {
            let drawable = &mesh_buffers.drawables[drawable_index];
            let texture_index = usize::try_from(drawable.texture_index).map_err(|_| {
                WgpuRenderError::InvalidTextureIndex {
                    texture_index: drawable.texture_index,
                }
            })?;
            let texture_bind_group =
                texture_bind_groups
                    .get(texture_index)
                    .ok_or(WgpuRenderError::MissingTexture {
                        texture_index: drawable.texture_index,
                    })?;

            render_pass.set_bind_group(0, texture_bind_group, &[]);
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
        opacity: mesh.opacity(),
        draw_order: mesh.draw_order(),
    })
}
