use crate::core::draw_order_from_raw;

use super::{Moc3ArtMeshKeyforms, Moc3ArtMeshes};

const DRAWABLE_BLEND_ADDITIVE: u8 = 1 << 0;
const DRAWABLE_BLEND_MULTIPLICATIVE: u8 = 1 << 1;
const DRAWABLE_MASK_INVERTED: u8 = 1 << 3;

#[derive(Debug, Copy, Clone, PartialEq)]
/// One vertex in a generated drawable mesh.
pub struct Moc3DrawableVertex {
    position: [f32; 2],
    uv: [f32; 2],
}

impl Moc3DrawableVertex {
    /// Creates a drawable vertex from model-space position and UV coordinates.
    pub fn new(position: [f32; 2], uv: [f32; 2]) -> Self {
        Self { position, uv }
    }

    /// Returns the model-space position.
    pub fn position(&self) -> [f32; 2] {
        self.position
    }

    /// Returns the texture coordinate.
    pub fn uv(&self) -> [f32; 2] {
        self.uv
    }
}

#[derive(Debug, Clone, PartialEq)]
/// A drawable mesh generated from MOC3 art mesh data.
///
/// Renderers draw these meshes with the texture, opacity, blend mode, mask
/// information, and colors carried by the mesh.
pub struct Moc3DrawableMesh {
    texture_index: i32,
    drawable_flags: u8,
    opacity: f32,
    draw_order: f32,
    render_order: i32,
    multiply_color: [f32; 3],
    screen_color: [f32; 3],
    vertices: Vec<Moc3DrawableVertex>,
    indices: Vec<u16>,
    masks: Vec<i32>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// Blend mode for a drawable mesh.
pub enum Moc3DrawableBlendMode {
    /// Standard alpha blending.
    Normal,
    /// Additive blending.
    Additive,
    /// Multiplicative blending.
    Multiplicative,
}

impl Moc3DrawableBlendMode {
    /// Decodes a blend mode from raw drawable flags.
    pub fn from_flags(flags: u8) -> Self {
        if flags & DRAWABLE_BLEND_ADDITIVE != 0 {
            Self::Additive
        } else if flags & DRAWABLE_BLEND_MULTIPLICATIVE != 0 {
            Self::Multiplicative
        } else {
            Self::Normal
        }
    }
}

impl Moc3DrawableMesh {
    /// Creates a drawable mesh from raw parts.
    pub fn from_parts(
        texture_index: i32,
        drawable_flags: u8,
        opacity: f32,
        draw_order: f32,
        vertices: Vec<Moc3DrawableVertex>,
        indices: Vec<u16>,
        masks: Vec<i32>,
    ) -> Self {
        Self::from_parts_with_render_order(
            texture_index,
            drawable_flags,
            opacity,
            draw_order,
            draw_order_from_raw(draw_order),
            vertices,
            indices,
            masks,
        )
    }

    #[allow(clippy::too_many_arguments)]
    /// Creates a drawable mesh from raw parts with an explicit render order.
    pub fn from_parts_with_render_order(
        texture_index: i32,
        drawable_flags: u8,
        opacity: f32,
        draw_order: f32,
        render_order: i32,
        vertices: Vec<Moc3DrawableVertex>,
        indices: Vec<u16>,
        masks: Vec<i32>,
    ) -> Self {
        Self {
            texture_index,
            drawable_flags,
            opacity,
            draw_order,
            render_order,
            multiply_color: [1.0, 1.0, 1.0],
            screen_color: [0.0, 0.0, 0.0],
            vertices,
            indices,
            masks,
        }
    }

    /// Returns the texture index used by this drawable.
    pub fn texture_index(&self) -> i32 {
        self.texture_index
    }

    /// Returns the multiply color applied to this drawable.
    pub fn multiply_color(&self) -> [f32; 3] {
        self.multiply_color
    }

    /// Returns the screen color applied to this drawable.
    pub fn screen_color(&self) -> [f32; 3] {
        self.screen_color
    }

    pub(crate) fn set_multiply_color(&mut self, color: [f32; 3]) {
        self.multiply_color = color;
    }

    pub(crate) fn set_screen_color(&mut self, color: [f32; 3]) {
        self.screen_color = color;
    }

    /// Returns the raw drawable flags from the model.
    pub fn drawable_flags(&self) -> u8 {
        self.drawable_flags
    }

    /// Returns the decoded blend mode.
    pub fn blend_mode(&self) -> Moc3DrawableBlendMode {
        Moc3DrawableBlendMode::from_flags(self.drawable_flags)
    }

    /// Returns whether this drawable uses inverted mask semantics.
    pub fn is_inverted_mask(&self) -> bool {
        self.drawable_flags & DRAWABLE_MASK_INVERTED != 0
    }

    /// Returns the drawable opacity.
    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    pub(crate) fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity;
    }

    /// Returns the raw draw order value.
    pub fn draw_order(&self) -> f32 {
        self.draw_order
    }

    pub(crate) fn set_draw_order(&mut self, draw_order: f32) {
        self.draw_order = draw_order;
    }

    /// Returns the resolved render order value.
    pub fn render_order(&self) -> i32 {
        self.render_order
    }

    pub(crate) fn set_render_order(&mut self, render_order: i32) {
        self.render_order = render_order;
    }

    /// Returns mesh vertices in model coordinates.
    pub fn vertices(&self) -> &[Moc3DrawableVertex] {
        &self.vertices
    }

    pub(crate) fn vertices_mut(&mut self) -> &mut [Moc3DrawableVertex] {
        &mut self.vertices
    }

    /// Returns triangle indices into [`vertices`](Self::vertices).
    pub fn indices(&self) -> &[u16] {
        &self.indices
    }

    /// Returns drawable indices used as masks for this drawable.
    pub fn masks(&self) -> &[i32] {
        &self.masks
    }
}

/// Builds one drawable mesh for an art mesh keyform.
pub fn build_moc3_drawable_mesh(
    art_meshes: &Moc3ArtMeshes,
    keyforms: &Moc3ArtMeshKeyforms,
    art_mesh_index: usize,
    local_keyform_index: usize,
) -> Option<Moc3DrawableMesh> {
    let mesh = *art_meshes.meshes().get(art_mesh_index)?;
    let keyform = *keyforms
        .art_mesh_keyforms(art_mesh_index)?
        .get(local_keyform_index)?;
    let positions = keyforms.art_mesh_keyform_positions(art_mesh_index, local_keyform_index)?;
    let uvs = art_meshes.art_mesh_uvs(art_mesh_index)?;
    if positions.len() != uvs.len() || positions.len() % 2 != 0 {
        return None;
    }

    let vertices = positions
        .as_chunks::<2>()
        .0
        .iter()
        .zip(uvs.as_chunks::<2>().0)
        .map(|(position, uv)| Moc3DrawableVertex::new([position[0], position[1]], [uv[0], uv[1]]))
        .collect::<Vec<_>>();

    let mut indices = Vec::with_capacity(mesh.position_index_count() as usize);
    for position_index in art_meshes.art_mesh_position_indices(art_mesh_index)? {
        let position_index = u16::try_from(*position_index).ok()?;
        if usize::from(position_index) >= vertices.len() {
            return None;
        }
        indices.push(position_index);
    }

    Some(Moc3DrawableMesh::from_parts_with_render_order(
        mesh.texture_index(),
        mesh.drawable_flags(),
        keyform.opacity(),
        keyform.draw_order(),
        art_meshes.art_mesh_render_order(art_mesh_index)?,
        vertices,
        indices,
        art_meshes.art_mesh_masks(art_mesh_index)?.to_vec(),
    ))
}

/// Builds drawable meshes for all art meshes using their first keyform.
pub fn build_moc3_drawable_meshes(
    art_meshes: &Moc3ArtMeshes,
    keyforms: &Moc3ArtMeshKeyforms,
) -> Option<Vec<Moc3DrawableMesh>> {
    let mut meshes = Vec::with_capacity(art_meshes.meshes().len());

    for art_mesh_index in 0..art_meshes.meshes().len() {
        meshes.push(build_moc3_drawable_mesh(
            art_meshes,
            keyforms,
            art_mesh_index,
            0,
        )?);
    }

    Some(meshes)
}
