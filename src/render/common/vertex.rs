use crate::moc3::{Moc3DrawableMesh, Moc3DrawableVertex};

#[derive(Debug, Copy, Clone, PartialEq)]
/// A renderer-ready Live2D drawable vertex.
///
/// The layout is two `f32` position values, two `f32` UV values, one `f32`
/// opacity value, three `f32` multiply-color channels, and three `f32`
/// screen-color channels.
pub struct DrawableVertex {
    position: [f32; 2],
    uv: [f32; 2],
    opacity: f32,
    multiply: [f32; 3],
    screen: [f32; 3],
}

impl DrawableVertex {
    /// Size in bytes of one encoded vertex.
    pub const STRIDE: usize = 44;

    /// Creates a vertex with default multiply and screen colors.
    pub fn new(position: [f32; 2], uv: [f32; 2], opacity: f32) -> Self {
        Self::with_colors(position, uv, opacity, [1.0, 1.0, 1.0], [0.0, 0.0, 0.0])
    }

    /// Creates a vertex with explicit multiply and screen colors.
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

    /// Returns the model-space vertex position.
    pub fn position(&self) -> [f32; 2] {
        self.position
    }

    /// Returns the texture coordinate.
    pub fn uv(&self) -> [f32; 2] {
        self.uv
    }

    /// Returns the drawable opacity copied onto this vertex.
    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    /// Returns the drawable multiply color copied onto this vertex.
    pub fn multiply(&self) -> [f32; 3] {
        self.multiply
    }

    /// Returns the drawable screen color copied onto this vertex.
    pub fn screen(&self) -> [f32; 3] {
        self.screen
    }
}

/// Converts all vertices from a drawable mesh into [`DrawableVertex`] values.
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

/// Encodes a drawable mesh's vertices into native-endian `f32` bytes.
///
/// The output buffer is cleared before new bytes are appended.
pub fn encode_vertices_from_drawable(mesh: &Moc3DrawableMesh, bytes: &mut Vec<u8>) {
    let byte_len = mesh.vertices().len() * DrawableVertex::STRIDE;
    bytes.clear();
    bytes.resize(byte_len, 0);

    let opacity = mesh.opacity();
    let multiply = mesh.multiply_color();
    let screen = mesh.screen_color();
    for (index, vertex) in mesh.vertices().iter().enumerate() {
        encode_vertex_into(
            vertex.position(),
            vertex.uv(),
            opacity,
            multiply,
            screen,
            &mut bytes[index * DrawableVertex::STRIDE..][..DrawableVertex::STRIDE],
        );
    }
}

/// Converts one raw MOC3 drawable vertex into a renderer-ready vertex.
pub fn vertex_from_drawable_vertex(
    vertex: &Moc3DrawableVertex,
    opacity: f32,
    multiply: [f32; 3],
    screen: [f32; 3],
) -> DrawableVertex {
    DrawableVertex::with_colors(vertex.position(), vertex.uv(), opacity, multiply, screen)
}

/// Encodes vertices into native-endian `f32` bytes.
pub fn encode_vertices(vertices: &[DrawableVertex]) -> Vec<u8> {
    let mut bytes = vec![0; vertices.len() * DrawableVertex::STRIDE];
    for (index, vertex) in vertices.iter().enumerate() {
        encode_vertex_into(
            vertex.position,
            vertex.uv,
            vertex.opacity,
            vertex.multiply,
            vertex.screen,
            &mut bytes[index * DrawableVertex::STRIDE..][..DrawableVertex::STRIDE],
        );
    }

    bytes
}

fn encode_vertex_into(
    position: [f32; 2],
    uv: [f32; 2],
    opacity: f32,
    multiply: [f32; 3],
    screen: [f32; 3],
    bytes: &mut [u8],
) {
    write_f32(bytes, 0, position[0]);
    write_f32(bytes, 4, position[1]);
    write_f32(bytes, 8, uv[0]);
    write_f32(bytes, 12, uv[1]);
    write_f32(bytes, 16, opacity);
    write_f32(bytes, 20, multiply[0]);
    write_f32(bytes, 24, multiply[1]);
    write_f32(bytes, 28, multiply[2]);
    write_f32(bytes, 32, screen[0]);
    write_f32(bytes, 36, screen[1]);
    write_f32(bytes, 40, screen[2]);
}

/// Encodes `u16` mesh indices into native-endian bytes.
pub fn encode_indices(indices: &[u16]) -> Vec<u8> {
    let mut bytes = vec![0; indices.len() * 2];
    for (chunk, index) in bytes.as_chunks_mut::<2>().0.iter_mut().zip(indices) {
        chunk.copy_from_slice(&index.to_ne_bytes());
    }

    bytes
}

fn write_f32(bytes: &mut [u8], offset: usize, value: f32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
}
