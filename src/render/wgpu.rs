use wgpu::util::DeviceExt;

use crate::moc3::{Moc3DrawableMesh, Moc3DrawableVertex};

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
        label: Some("rusty_live2d.drawable.vertices"),
        contents: &vertex_bytes,
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("rusty_live2d.drawable.indices"),
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
