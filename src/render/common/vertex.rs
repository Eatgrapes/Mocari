use crate::moc3::{Moc3DrawableMesh, Moc3DrawableVertex};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DrawableVertex {
    position: [f32; 2],
    uv: [f32; 2],
    opacity: f32,
    multiply: [f32; 3],
    screen: [f32; 3],
}

impl DrawableVertex {
    pub const STRIDE: usize = 44;

    pub fn new(position: [f32; 2], uv: [f32; 2], opacity: f32) -> Self {
        Self::with_colors(position, uv, opacity, [1.0, 1.0, 1.0], [0.0, 0.0, 0.0])
    }

    pub fn with_colors(
        position: [f32; 2],
        uv: [f32; 2],
        opacity: f32,
        multiply: [f32; 3],
        screen: [f32; 3],
    ) -> Self {
        Self {
            position,
            uv,
            opacity,
            multiply,
            screen,
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

    pub fn multiply(&self) -> [f32; 3] {
        self.multiply
    }

    pub fn screen(&self) -> [f32; 3] {
        self.screen
    }
}

pub fn vertices_from_drawable(mesh: &Moc3DrawableMesh) -> Vec<DrawableVertex> {
    mesh.vertices()
        .iter()
        .map(|vertex| {
            vertex_from_drawable_vertex(
                vertex,
                mesh.opacity(),
                mesh.multiply_color(),
                mesh.screen_color(),
            )
        })
        .collect()
}

pub fn encode_vertices_from_drawable(mesh: &Moc3DrawableMesh, bytes: &mut Vec<u8>) {
    bytes.clear();
    bytes.reserve(mesh.vertices().len() * DrawableVertex::STRIDE);
    for vertex in mesh.vertices() {
        encode_vertex(
            vertex.position(),
            vertex.uv(),
            mesh.opacity(),
            mesh.multiply_color(),
            mesh.screen_color(),
            bytes,
        );
    }
}

pub fn vertex_from_drawable_vertex(
    vertex: &Moc3DrawableVertex,
    opacity: f32,
    multiply: [f32; 3],
    screen: [f32; 3],
) -> DrawableVertex {
    DrawableVertex::with_colors(vertex.position(), vertex.uv(), opacity, multiply, screen)
}

pub fn encode_vertices(vertices: &[DrawableVertex]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vertices.len() * DrawableVertex::STRIDE);
    for vertex in vertices {
        encode_vertex(
            vertex.position,
            vertex.uv,
            vertex.opacity,
            vertex.multiply,
            vertex.screen,
            &mut bytes,
        );
    }

    bytes
}

fn encode_vertex(
    position: [f32; 2],
    uv: [f32; 2],
    opacity: f32,
    multiply: [f32; 3],
    screen: [f32; 3],
    bytes: &mut Vec<u8>,
) {
    bytes.extend_from_slice(&position[0].to_ne_bytes());
    bytes.extend_from_slice(&position[1].to_ne_bytes());
    bytes.extend_from_slice(&uv[0].to_ne_bytes());
    bytes.extend_from_slice(&uv[1].to_ne_bytes());
    bytes.extend_from_slice(&opacity.to_ne_bytes());
    for channel in multiply {
        bytes.extend_from_slice(&channel.to_ne_bytes());
    }
    for channel in screen {
        bytes.extend_from_slice(&channel.to_ne_bytes());
    }
}

pub fn encode_indices(indices: &[u16]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(indices.len() * 2);
    for index in indices {
        bytes.extend_from_slice(&index.to_ne_bytes());
    }

    bytes
}
