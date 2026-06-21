use wgpu::util::DeviceExt;

use crate::moc3::{Moc3DrawableBlendMode, Moc3DrawableMesh};
use crate::render::common::{
    ClippingRect, DrawableInfo, DrawableVertex, draw_order_indices, encode_indices,
    encode_vertices, vertices_from_drawable,
};

pub fn drawable_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    const ATTRIBUTES: [wgpu::VertexAttribute; 5] = [
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
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x3,
            offset: 20,
            shader_location: 3,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x3,
            offset: 32,
            shader_location: 4,
        },
    ];

    wgpu::VertexBufferLayout {
        array_stride: DrawableVertex::STRIDE as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ATTRIBUTES,
    }
}

#[derive(Debug)]
pub struct WgpuDrawableBuffers {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    vertex_count: u32,
    index_count: u32,
    info: DrawableInfo,
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

    pub fn is_empty(&self) -> bool {
        self.vertex_count == 0 || self.index_count == 0
    }

    pub fn info(&self) -> &DrawableInfo {
        &self.info
    }

    pub fn texture_index(&self) -> i32 {
        self.info.texture_index()
    }

    pub fn blend_mode(&self) -> Moc3DrawableBlendMode {
        self.info.blend_mode()
    }

    pub fn opacity(&self) -> f32 {
        self.info.opacity()
    }

    pub fn draw_order(&self) -> f32 {
        self.info.draw_order()
    }

    pub fn render_order(&self) -> i32 {
        self.info.render_order()
    }

    pub fn masks(&self) -> &[i32] {
        self.info.masks()
    }

    pub fn inverted_mask(&self) -> bool {
        self.info.inverted_mask()
    }

    pub fn bounds(&self) -> Option<ClippingRect> {
        self.info.bounds()
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

    pub fn drawable_infos(&self) -> Vec<DrawableInfo> {
        self.drawables.iter().map(|d| d.info.clone()).collect()
    }

    pub fn draw_order_indices(&self) -> Vec<usize> {
        draw_order_indices(&self.drawable_infos())
    }
}

pub fn create_wgpu_drawable_buffers(
    device: &wgpu::Device,
    mesh: &Moc3DrawableMesh,
) -> Option<WgpuDrawableBuffers> {
    let vertices = vertices_from_drawable(mesh);
    let vertex_bytes = encode_vertices(&vertices);
    let index_bytes = encode_indices(mesh.indices());
    let vertex_count = u32::try_from(vertices.len()).ok()?;
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
        vertex_count,
        index_count,
        info: DrawableInfo::from_mesh(mesh),
    })
}
