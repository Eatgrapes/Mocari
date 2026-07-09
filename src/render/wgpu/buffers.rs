use wgpu::util::DeviceExt;

use crate::moc3::{Moc3DrawableBlendMode, Moc3DrawableMesh};
use crate::render::common::{
    ClippingRect, DrawableInfo, DrawableVertex, draw_order_indices_from, encode_indices,
    encode_vertices, encode_vertices_from_drawable, vertices_from_drawable,
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
    vertex_bytes: Vec<u8>,
    indices: Vec<u16>,
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

    pub fn is_visible(&self) -> bool {
        !self.is_empty() && self.info.is_visible()
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
    draw_order_indices: Vec<usize>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct WgpuMeshUpdate {
    uploaded_drawables: usize,
    bounds_changed: bool,
    visibility_changed: bool,
}

impl WgpuMeshUpdate {
    pub fn uploaded_drawables(&self) -> usize {
        self.uploaded_drawables
    }

    pub fn bounds_changed(&self) -> bool {
        self.bounds_changed
    }

    pub fn visibility_changed(&self) -> bool {
        self.visibility_changed
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WgpuMeshUpdateError {
    #[error("drawable count changed from {expected} to {actual}")]
    DrawableCount { expected: usize, actual: usize },
    #[error("drawable {drawable_index} vertex count changed from {expected} to {actual}")]
    VertexCount {
        drawable_index: usize,
        expected: usize,
        actual: usize,
    },
    #[error("drawable {drawable_index} index count changed from {expected} to {actual}")]
    IndexCount {
        drawable_index: usize,
        expected: usize,
        actual: usize,
    },
    #[error("drawable {drawable_index} indices changed")]
    Indices { drawable_index: usize },
    #[error("drawable {drawable_index} texture index changed from {expected} to {actual}")]
    TextureIndex {
        drawable_index: usize,
        expected: i32,
        actual: i32,
    },
    #[error("drawable {drawable_index} blend mode changed from {expected:?} to {actual:?}")]
    BlendMode {
        drawable_index: usize,
        expected: Moc3DrawableBlendMode,
        actual: Moc3DrawableBlendMode,
    },
    #[error("drawable {drawable_index} masks changed")]
    Masks { drawable_index: usize },
    #[error("drawable {drawable_index} inverted mask changed from {expected} to {actual}")]
    InvertedMask {
        drawable_index: usize,
        expected: bool,
        actual: bool,
    },
}

impl WgpuMeshBuffers {
    pub fn from_drawables(device: &wgpu::Device, meshes: &[Moc3DrawableMesh]) -> Option<Self> {
        let mut drawables = Vec::with_capacity(meshes.len());
        for mesh in meshes {
            drawables.push(create_wgpu_drawable_buffers(device, mesh)?);
        }
        let draw_order_indices = draw_order_indices_from(
            drawables.len(),
            |index| drawables[index].draw_order(),
            |index| drawables[index].render_order(),
        );

        Some(Self {
            drawables,
            draw_order_indices,
        })
    }

    pub fn drawables(&self) -> &[WgpuDrawableBuffers] {
        &self.drawables
    }

    pub fn drawable_infos(&self) -> Vec<DrawableInfo> {
        self.drawables.iter().map(|d| d.info.clone()).collect()
    }

    pub(crate) fn iter_drawable_infos(&self) -> impl Iterator<Item = &DrawableInfo> {
        self.drawables.iter().map(WgpuDrawableBuffers::info)
    }

    pub(crate) fn drawable_bounds(&self, drawable_index: usize) -> Option<ClippingRect> {
        self.drawables
            .get(drawable_index)
            .and_then(WgpuDrawableBuffers::bounds)
    }

    pub fn draw_order_indices(&self) -> &[usize] {
        &self.draw_order_indices
    }

    pub fn update_drawables(
        &mut self,
        queue: &wgpu::Queue,
        meshes: &[Moc3DrawableMesh],
    ) -> Result<WgpuMeshUpdate, WgpuMeshUpdateError> {
        if self.drawables.len() != meshes.len() {
            return Err(WgpuMeshUpdateError::DrawableCount {
                expected: self.drawables.len(),
                actual: meshes.len(),
            });
        }

        for (drawable_index, (drawable, mesh)) in self.drawables.iter().zip(meshes).enumerate() {
            validate_drawable_update(drawable_index, drawable, mesh)?;
        }

        let mut vertex_bytes = Vec::new();
        let mut uploads = 0;
        let mut bounds_changed = false;
        let mut visibility_changed = false;
        for (drawable, mesh) in self.drawables.iter_mut().zip(meshes) {
            encode_vertices_from_drawable(mesh, &mut vertex_bytes);
            if !vertex_bytes.is_empty() && vertex_bytes != drawable.vertex_bytes {
                queue.write_buffer(&drawable.vertex_buffer, 0, &vertex_bytes);
                drawable.vertex_bytes.clear();
                drawable.vertex_bytes.extend_from_slice(&vertex_bytes);
                uploads += 1;
            }
            let was_visible = drawable.is_visible();
            let info = DrawableInfo::from_mesh(mesh);
            let is_visible = !drawable.is_empty() && info.is_visible();
            bounds_changed |= drawable.info.bounds() != info.bounds();
            visibility_changed |= was_visible != is_visible;
            drawable.info = info;
        }
        self.draw_order_indices = draw_order_indices_from(
            self.drawables.len(),
            |index| self.drawables[index].draw_order(),
            |index| self.drawables[index].render_order(),
        );

        Ok(WgpuMeshUpdate {
            uploaded_drawables: uploads,
            bounds_changed,
            visibility_changed,
        })
    }
}

fn validate_drawable_update(
    drawable_index: usize,
    drawable: &WgpuDrawableBuffers,
    mesh: &Moc3DrawableMesh,
) -> Result<(), WgpuMeshUpdateError> {
    validate_count(
        drawable.vertex_count as usize,
        mesh.vertices().len(),
        WgpuMeshUpdateError::VertexCount {
            drawable_index,
            expected: drawable.vertex_count as usize,
            actual: mesh.vertices().len(),
        },
    )?;
    validate_count(
        drawable.index_count as usize,
        mesh.indices().len(),
        WgpuMeshUpdateError::IndexCount {
            drawable_index,
            expected: drawable.index_count as usize,
            actual: mesh.indices().len(),
        },
    )?;
    validate_unchanged(
        drawable.indices.as_slice(),
        mesh.indices(),
        WgpuMeshUpdateError::Indices { drawable_index },
    )?;
    validate_unchanged(
        &drawable.texture_index(),
        &mesh.texture_index(),
        WgpuMeshUpdateError::TextureIndex {
            drawable_index,
            expected: drawable.texture_index(),
            actual: mesh.texture_index(),
        },
    )?;
    validate_unchanged(
        &drawable.blend_mode(),
        &mesh.blend_mode(),
        WgpuMeshUpdateError::BlendMode {
            drawable_index,
            expected: drawable.blend_mode(),
            actual: mesh.blend_mode(),
        },
    )?;
    validate_unchanged(
        drawable.masks(),
        mesh.masks(),
        WgpuMeshUpdateError::Masks { drawable_index },
    )?;
    validate_unchanged(
        &drawable.inverted_mask(),
        &mesh.is_inverted_mask(),
        WgpuMeshUpdateError::InvertedMask {
            drawable_index,
            expected: drawable.inverted_mask(),
            actual: mesh.is_inverted_mask(),
        },
    )?;

    Ok(())
}

fn validate_count(
    expected: usize,
    actual: usize,
    error: WgpuMeshUpdateError,
) -> Result<(), WgpuMeshUpdateError> {
    if expected == actual {
        Ok(())
    } else {
        Err(error)
    }
}

fn validate_unchanged<T: PartialEq + ?Sized>(
    expected: &T,
    actual: &T,
    error: WgpuMeshUpdateError,
) -> Result<(), WgpuMeshUpdateError> {
    if expected == actual {
        Ok(())
    } else {
        Err(error)
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
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
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
        vertex_bytes,
        indices: mesh.indices().to_vec(),
        info: DrawableInfo::from_mesh(mesh),
    })
}
