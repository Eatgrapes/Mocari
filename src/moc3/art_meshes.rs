use crate::{Error, Result, core::draw_order_from_raw};

use super::{Endianness, Moc3CountInfo, Moc3Header, Moc3SectionOffsets};

const TEXTURE_INDICES_SLOT: usize = 41;
const DRAWABLE_FLAGS_SLOT: usize = 42;
const DRAWABLE_BLEND_ADDITIVE: u8 = 1 << 0;
const DRAWABLE_BLEND_MULTIPLICATIVE: u8 = 1 << 1;
const ART_MESH_KEYFORM_BINDING_BAND_INDICES_SLOT: usize = 34;
const KEYFORM_BEGIN_INDICES_SLOT: usize = 35;
const KEYFORM_COUNTS_SLOT: usize = 36;
const ART_MESH_PARENT_DEFORMER_INDICES_SLOT: usize = 40;
const VERTEX_COUNTS_SLOT: usize = 43;
const UV_BEGIN_INDICES_SLOT: usize = 44;
const POSITION_INDEX_BEGIN_INDICES_SLOT: usize = 45;
const POSITION_INDEX_COUNTS_SLOT: usize = 46;
const MASK_BEGIN_INDICES_SLOT: usize = 47;
const MASK_COUNTS_SLOT: usize = 48;
const UV_XYS_SLOT: usize = 78;
const POSITION_INDICES_SLOT: usize = 79;
const DRAWABLE_MASKS_SLOT: usize = 80;
const RENDER_ORDER_INDICES_SLOT: usize = 87;
const ART_MESH_KEYFORM_OPACITIES_SLOT: usize = 68;
const ART_MESH_KEYFORM_DRAW_ORDERS_SLOT: usize = 69;
const KEYFORM_POSITION_BEGIN_INDICES_SLOT: usize = 70;
const KEYFORM_POSITION_XYS_SLOT: usize = 71;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Moc3ArtMeshInfo {
    texture_index: i32,
    drawable_flags: u8,
    position_index_count: i32,
    uv_begin_index: i32,
    position_index_begin_index: i32,
    vertex_count: i32,
    mask_begin_index: i32,
    mask_count: i32,
}

impl Moc3ArtMeshInfo {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        texture_index: i32,
        drawable_flags: u8,
        position_index_count: i32,
        uv_begin_index: i32,
        position_index_begin_index: i32,
        vertex_count: i32,
        mask_begin_index: i32,
        mask_count: i32,
    ) -> Self {
        Self {
            texture_index,
            drawable_flags,
            position_index_count,
            uv_begin_index,
            position_index_begin_index,
            vertex_count,
            mask_begin_index,
            mask_count,
        }
    }

    pub fn texture_index(&self) -> i32 {
        self.texture_index
    }

    pub fn drawable_flags(&self) -> u8 {
        self.drawable_flags
    }

    pub fn position_index_count(&self) -> i32 {
        self.position_index_count
    }

    pub fn uv_begin_index(&self) -> i32 {
        self.uv_begin_index
    }

    pub fn position_index_begin_index(&self) -> i32 {
        self.position_index_begin_index
    }

    pub fn vertex_count(&self) -> i32 {
        self.vertex_count
    }

    pub fn mask_begin_index(&self) -> i32 {
        self.mask_begin_index
    }

    pub fn mask_count(&self) -> i32 {
        self.mask_count
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Moc3ArtMeshes {
    meshes: Vec<Moc3ArtMeshInfo>,
    keyform_binding_band_indices: Vec<i32>,
    parent_deformer_indices: Vec<i32>,
    render_orders: Vec<i32>,
    uv_xys: Vec<f32>,
    position_indices: Vec<i16>,
    drawable_masks: Vec<i32>,
}

impl Moc3ArtMeshes {
    pub fn from_parts(
        meshes: Vec<Moc3ArtMeshInfo>,
        uv_xys: Vec<f32>,
        position_indices: Vec<i16>,
        drawable_masks: Vec<i32>,
    ) -> Result<Self> {
        let mesh_count = meshes.len();
        Self::from_parts_with_hierarchy(
            meshes,
            vec![0; mesh_count],
            vec![-1; mesh_count],
            uv_xys,
            position_indices,
            drawable_masks,
        )
    }

    pub fn from_parts_with_hierarchy(
        meshes: Vec<Moc3ArtMeshInfo>,
        keyform_binding_band_indices: Vec<i32>,
        parent_deformer_indices: Vec<i32>,
        uv_xys: Vec<f32>,
        position_indices: Vec<i16>,
        drawable_masks: Vec<i32>,
    ) -> Result<Self> {
        let render_orders = default_render_orders(meshes.len());
        Self::from_parts_with_hierarchy_and_render_orders(
            meshes,
            keyform_binding_band_indices,
            parent_deformer_indices,
            render_orders,
            uv_xys,
            position_indices,
            drawable_masks,
        )
    }

    pub fn from_parts_with_hierarchy_and_render_orders(
        meshes: Vec<Moc3ArtMeshInfo>,
        keyform_binding_band_indices: Vec<i32>,
        parent_deformer_indices: Vec<i32>,
        render_orders: Vec<i32>,
        uv_xys: Vec<f32>,
        position_indices: Vec<i16>,
        drawable_masks: Vec<i32>,
    ) -> Result<Self> {
        if keyform_binding_band_indices.len() != meshes.len()
            || parent_deformer_indices.len() != meshes.len()
            || render_orders.len() != meshes.len()
        {
            return Err(invalid_art_meshes(
                "art mesh hierarchy metadata lengths do not match",
            ));
        }

        for (index, mesh) in meshes.iter().copied().enumerate() {
            validate_mesh_ranges(
                index,
                mesh,
                uv_xys.len(),
                position_indices.len(),
                drawable_masks.len(),
            )?;
        }

        Ok(Self {
            meshes,
            keyform_binding_band_indices,
            parent_deformer_indices,
            render_orders,
            uv_xys,
            position_indices,
            drawable_masks,
        })
    }

    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let header = Moc3Header::parse(bytes)?;
        let offsets = Moc3SectionOffsets::parse(bytes)?;
        let counts = Moc3CountInfo::parse(bytes)?;
        let art_mesh_count = to_usize(counts.art_meshes(), "art mesh count")?;
        let render_orders = parse_render_orders(bytes, &offsets, &counts, art_mesh_count)?;

        let keyform_binding_band_indices = read_i32_section_or_default(
            bytes,
            &offsets,
            ART_MESH_KEYFORM_BINDING_BAND_INDICES_SLOT,
            art_mesh_count,
            header.endianness(),
            0,
        )?;
        let texture_indices = read_i32_section(
            bytes,
            &offsets,
            TEXTURE_INDICES_SLOT,
            art_mesh_count,
            header.endianness(),
        )?;
        let drawable_flags = read_u8_section(bytes, &offsets, DRAWABLE_FLAGS_SLOT, art_mesh_count)?;
        let vertex_counts = read_i32_section(
            bytes,
            &offsets,
            VERTEX_COUNTS_SLOT,
            art_mesh_count,
            header.endianness(),
        )?;
        let uv_begin_indices = read_i32_section(
            bytes,
            &offsets,
            UV_BEGIN_INDICES_SLOT,
            art_mesh_count,
            header.endianness(),
        )?;
        let position_index_begin_indices = read_i32_section(
            bytes,
            &offsets,
            POSITION_INDEX_BEGIN_INDICES_SLOT,
            art_mesh_count,
            header.endianness(),
        )?;
        let position_index_counts = read_i32_section(
            bytes,
            &offsets,
            POSITION_INDEX_COUNTS_SLOT,
            art_mesh_count,
            header.endianness(),
        )?;
        let mask_begin_indices = read_i32_section(
            bytes,
            &offsets,
            MASK_BEGIN_INDICES_SLOT,
            art_mesh_count,
            header.endianness(),
        )?;
        let mask_counts = read_i32_section(
            bytes,
            &offsets,
            MASK_COUNTS_SLOT,
            art_mesh_count,
            header.endianness(),
        )?;
        let parent_deformer_indices = read_i32_section_or_default(
            bytes,
            &offsets,
            ART_MESH_PARENT_DEFORMER_INDICES_SLOT,
            art_mesh_count,
            header.endianness(),
            -1,
        )?;
        let uv_xys = read_f32_section(
            bytes,
            &offsets,
            UV_XYS_SLOT,
            to_usize(counts.uvs(), "uv count")?,
            header.endianness(),
        )?;
        let position_indices = read_i16_section(
            bytes,
            &offsets,
            POSITION_INDICES_SLOT,
            to_usize(counts.position_indices(), "position index count")?,
            header.endianness(),
        )?;
        let drawable_masks = read_i32_section(
            bytes,
            &offsets,
            DRAWABLE_MASKS_SLOT,
            to_usize(counts.drawable_masks(), "drawable mask count")?,
            header.endianness(),
        )?;

        let mut meshes = Vec::with_capacity(art_mesh_count);
        for index in 0..art_mesh_count {
            meshes.push(Moc3ArtMeshInfo::new(
                texture_indices[index],
                drawable_flags[index],
                position_index_counts[index],
                uv_begin_indices[index],
                position_index_begin_indices[index],
                vertex_counts[index],
                mask_begin_indices[index],
                mask_counts[index],
            ));
        }

        Self::from_parts_with_hierarchy_and_render_orders(
            meshes,
            keyform_binding_band_indices,
            parent_deformer_indices,
            render_orders,
            uv_xys,
            position_indices,
            drawable_masks,
        )
    }

    pub fn meshes(&self) -> &[Moc3ArtMeshInfo] {
        &self.meshes
    }

    pub fn keyform_binding_band_indices(&self) -> &[i32] {
        &self.keyform_binding_band_indices
    }

    pub fn parent_deformer_indices(&self) -> &[i32] {
        &self.parent_deformer_indices
    }

    pub fn render_orders(&self) -> &[i32] {
        &self.render_orders
    }

    pub fn uv_xys(&self) -> &[f32] {
        &self.uv_xys
    }

    pub fn position_indices(&self) -> &[i16] {
        &self.position_indices
    }

    pub fn drawable_masks(&self) -> &[i32] {
        &self.drawable_masks
    }

    pub fn art_mesh_uvs(&self, index: usize) -> Option<&[f32]> {
        let mesh = self.meshes.get(index)?;
        let start = usize::try_from(mesh.uv_begin_index).ok()?;
        let len = usize::try_from(mesh.vertex_count).ok()?.checked_mul(2)?;
        self.uv_xys.get(start..start.checked_add(len)?)
    }

    pub fn art_mesh_position_indices(&self, index: usize) -> Option<&[i16]> {
        let mesh = self.meshes.get(index)?;
        let start = usize::try_from(mesh.position_index_begin_index).ok()?;
        let len = usize::try_from(mesh.position_index_count).ok()?;
        self.position_indices.get(start..start.checked_add(len)?)
    }

    pub fn art_mesh_masks(&self, index: usize) -> Option<&[i32]> {
        let mesh = self.meshes.get(index)?;
        let start = usize::try_from(mesh.mask_begin_index).ok()?;
        let len = usize::try_from(mesh.mask_count).ok()?;
        self.drawable_masks.get(start..start.checked_add(len)?)
    }

    pub fn art_mesh_keyform_binding_band_index(&self, index: usize) -> Option<i32> {
        self.keyform_binding_band_indices.get(index).copied()
    }

    pub fn art_mesh_parent_deformer_index(&self, index: usize) -> Option<i32> {
        self.parent_deformer_indices.get(index).copied()
    }

    pub fn art_mesh_render_order(&self, index: usize) -> Option<i32> {
        self.render_orders.get(index).copied()
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Moc3ArtMeshKeyformInfo {
    opacity: f32,
    draw_order: f32,
    position_begin_index: i32,
}

impl Moc3ArtMeshKeyformInfo {
    pub fn new(opacity: f32, draw_order: f32, position_begin_index: i32) -> Self {
        Self {
            opacity,
            draw_order,
            position_begin_index,
        }
    }

    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    pub fn draw_order(&self) -> f32 {
        self.draw_order
    }

    pub fn position_begin_index(&self) -> i32 {
        self.position_begin_index
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Moc3ArtMeshKeyforms {
    keyform_begin_indices: Vec<i32>,
    keyform_counts: Vec<i32>,
    vertex_counts: Vec<i32>,
    keyforms: Vec<Moc3ArtMeshKeyformInfo>,
    position_xys: Vec<f32>,
}

impl Moc3ArtMeshKeyforms {
    pub fn from_parts(
        keyform_begin_indices: Vec<i32>,
        keyform_counts: Vec<i32>,
        vertex_counts: Vec<i32>,
        keyforms: Vec<Moc3ArtMeshKeyformInfo>,
        position_xys: Vec<f32>,
    ) -> Result<Self> {
        if keyform_begin_indices.len() != keyform_counts.len()
            || keyform_begin_indices.len() != vertex_counts.len()
        {
            return Err(invalid_art_meshes(
                "art mesh keyform metadata lengths do not match",
            ));
        }

        for mesh_index in 0..keyform_begin_indices.len() {
            validate_keyform_ranges(
                mesh_index,
                keyform_begin_indices[mesh_index],
                keyform_counts[mesh_index],
                vertex_counts[mesh_index],
                &keyforms,
                position_xys.len(),
            )?;
        }

        Ok(Self {
            keyform_begin_indices,
            keyform_counts,
            vertex_counts,
            keyforms,
            position_xys,
        })
    }

    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let header = Moc3Header::parse(bytes)?;
        let offsets = Moc3SectionOffsets::parse(bytes)?;
        let counts = Moc3CountInfo::parse(bytes)?;
        let art_mesh_count = to_usize(counts.art_meshes(), "art mesh count")?;
        let art_mesh_keyform_count =
            to_usize(counts.art_mesh_keyforms(), "art mesh keyform count")?;

        let keyform_begin_indices = read_i32_section(
            bytes,
            &offsets,
            KEYFORM_BEGIN_INDICES_SLOT,
            art_mesh_count,
            header.endianness(),
        )?;
        let keyform_counts = read_i32_section(
            bytes,
            &offsets,
            KEYFORM_COUNTS_SLOT,
            art_mesh_count,
            header.endianness(),
        )?;
        let vertex_counts = read_i32_section(
            bytes,
            &offsets,
            VERTEX_COUNTS_SLOT,
            art_mesh_count,
            header.endianness(),
        )?;
        let opacities = read_f32_section(
            bytes,
            &offsets,
            ART_MESH_KEYFORM_OPACITIES_SLOT,
            art_mesh_keyform_count,
            header.endianness(),
        )?;
        let draw_orders = read_f32_section(
            bytes,
            &offsets,
            ART_MESH_KEYFORM_DRAW_ORDERS_SLOT,
            art_mesh_keyform_count,
            header.endianness(),
        )?;
        let position_begin_indices = read_i32_section(
            bytes,
            &offsets,
            KEYFORM_POSITION_BEGIN_INDICES_SLOT,
            art_mesh_keyform_count,
            header.endianness(),
        )?;
        let position_xys = read_f32_section(
            bytes,
            &offsets,
            KEYFORM_POSITION_XYS_SLOT,
            to_usize(counts.keyform_positions(), "keyform position count")?,
            header.endianness(),
        )?;

        let keyforms = opacities
            .iter()
            .zip(draw_orders.iter())
            .zip(position_begin_indices.iter())
            .map(|((opacity, draw_order), position_begin_index)| {
                Moc3ArtMeshKeyformInfo::new(*opacity, *draw_order, *position_begin_index)
            })
            .collect::<Vec<_>>();

        Self::from_parts(
            keyform_begin_indices,
            keyform_counts,
            vertex_counts,
            keyforms,
            position_xys,
        )
    }

    pub fn keyforms(&self) -> &[Moc3ArtMeshKeyformInfo] {
        &self.keyforms
    }

    pub fn position_xys(&self) -> &[f32] {
        &self.position_xys
    }

    pub fn art_mesh_keyforms(&self, mesh_index: usize) -> Option<&[Moc3ArtMeshKeyformInfo]> {
        let start = usize::try_from(*self.keyform_begin_indices.get(mesh_index)?).ok()?;
        let len = usize::try_from(*self.keyform_counts.get(mesh_index)?).ok()?;
        self.keyforms.get(start..start.checked_add(len)?)
    }

    pub fn art_mesh_keyform_positions(
        &self,
        mesh_index: usize,
        local_keyform_index: usize,
    ) -> Option<&[f32]> {
        let keyform = self
            .art_mesh_keyforms(mesh_index)?
            .get(local_keyform_index)?;
        let vertex_count = *self.vertex_counts.get(mesh_index)?;
        let start = usize::try_from(keyform.position_begin_index).ok()?;
        let len = usize::try_from(vertex_count).ok()?.checked_mul(2)?;
        self.position_xys.get(start..start.checked_add(len)?)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Moc3DrawableVertex {
    position: [f32; 2],
    uv: [f32; 2],
}

impl Moc3DrawableVertex {
    pub fn new(position: [f32; 2], uv: [f32; 2]) -> Self {
        Self { position, uv }
    }

    pub fn position(&self) -> [f32; 2] {
        self.position
    }

    pub fn uv(&self) -> [f32; 2] {
        self.uv
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Moc3DrawableMesh {
    texture_index: i32,
    drawable_flags: u8,
    opacity: f32,
    draw_order: f32,
    render_order: i32,
    vertices: Vec<Moc3DrawableVertex>,
    indices: Vec<u16>,
    masks: Vec<i32>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Moc3DrawableBlendMode {
    Normal,
    Additive,
    Multiplicative,
}

impl Moc3DrawableBlendMode {
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
            vertices,
            indices,
            masks,
        }
    }

    pub fn texture_index(&self) -> i32 {
        self.texture_index
    }

    pub fn drawable_flags(&self) -> u8 {
        self.drawable_flags
    }

    pub fn blend_mode(&self) -> Moc3DrawableBlendMode {
        Moc3DrawableBlendMode::from_flags(self.drawable_flags)
    }

    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    pub fn draw_order(&self) -> f32 {
        self.draw_order
    }

    pub fn render_order(&self) -> i32 {
        self.render_order
    }

    pub fn vertices(&self) -> &[Moc3DrawableVertex] {
        &self.vertices
    }

    pub fn indices(&self) -> &[u16] {
        &self.indices
    }

    pub fn masks(&self) -> &[i32] {
        &self.masks
    }
}

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
        .chunks_exact(2)
        .zip(uvs.chunks_exact(2))
        .map(|(position, uv)| Moc3DrawableVertex::new([position[0], position[1]], [uv[0], uv[1]]))
        .collect::<Vec<_>>();

    let mut indices = Vec::with_capacity(mesh.position_index_count as usize);
    for position_index in art_meshes.art_mesh_position_indices(art_mesh_index)? {
        let position_index = u16::try_from(*position_index).ok()?;
        if usize::from(position_index) >= vertices.len() {
            return None;
        }
        indices.push(position_index);
    }

    Some(Moc3DrawableMesh::from_parts_with_render_order(
        mesh.texture_index,
        mesh.drawable_flags,
        keyform.opacity,
        keyform.draw_order,
        art_meshes.art_mesh_render_order(art_mesh_index)?,
        vertices,
        indices,
        art_meshes.art_mesh_masks(art_mesh_index)?.to_vec(),
    ))
}

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

fn read_i32_section(
    bytes: &[u8],
    offsets: &Moc3SectionOffsets,
    slot: usize,
    count: usize,
    endianness: Endianness,
) -> Result<Vec<i32>> {
    read_section(bytes, offsets, slot, count, 4, |bytes, offset| {
        let raw = [
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ];
        match endianness {
            Endianness::Little => i32::from_le_bytes(raw),
            Endianness::Big => i32::from_be_bytes(raw),
        }
    })
}

fn read_i32_section_or_default(
    bytes: &[u8],
    offsets: &Moc3SectionOffsets,
    slot: usize,
    count: usize,
    endianness: Endianness,
    default: i32,
) -> Result<Vec<i32>> {
    match offsets.section_offset(slot) {
        Some(0) | None => Ok(vec![default; count]),
        Some(_) => read_i32_section(bytes, offsets, slot, count, endianness),
    }
}

fn parse_render_orders(
    bytes: &[u8],
    offsets: &Moc3SectionOffsets,
    counts: &Moc3CountInfo,
    art_mesh_count: usize,
) -> Result<Vec<i32>> {
    let mut render_orders = default_render_orders(art_mesh_count);
    let object_count = to_usize(
        counts.draw_order_group_objects(),
        "draw order group object count",
    )?;
    if object_count == 0
        || offsets
            .section_offset(RENDER_ORDER_INDICES_SLOT)
            .unwrap_or(0)
            == 0
    {
        return Ok(render_orders);
    }

    let order_indices = read_i32_section(
        bytes,
        offsets,
        RENDER_ORDER_INDICES_SLOT,
        object_count,
        Moc3Header::parse(bytes)?.endianness(),
    )?;

    for (rank, drawable_index) in order_indices.into_iter().enumerate() {
        let Ok(index) = usize::try_from(drawable_index) else {
            continue;
        };
        if let Some(render_order) = render_orders.get_mut(index) {
            *render_order = i32::try_from(rank)
                .map_err(|_| invalid_art_meshes("render order rank is too large"))?;
        }
    }

    Ok(render_orders)
}

fn default_render_orders(mesh_count: usize) -> Vec<i32> {
    (0..mesh_count)
        .map(|index| i32::try_from(index).unwrap_or(i32::MAX))
        .collect()
}

fn read_i16_section(
    bytes: &[u8],
    offsets: &Moc3SectionOffsets,
    slot: usize,
    count: usize,
    endianness: Endianness,
) -> Result<Vec<i16>> {
    read_section(bytes, offsets, slot, count, 2, |bytes, offset| {
        let raw = [bytes[offset], bytes[offset + 1]];
        match endianness {
            Endianness::Little => i16::from_le_bytes(raw),
            Endianness::Big => i16::from_be_bytes(raw),
        }
    })
}

fn read_f32_section(
    bytes: &[u8],
    offsets: &Moc3SectionOffsets,
    slot: usize,
    count: usize,
    endianness: Endianness,
) -> Result<Vec<f32>> {
    read_section(bytes, offsets, slot, count, 4, |bytes, offset| {
        let raw = [
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ];
        match endianness {
            Endianness::Little => f32::from_le_bytes(raw),
            Endianness::Big => f32::from_be_bytes(raw),
        }
    })
}

fn read_u8_section(
    bytes: &[u8],
    offsets: &Moc3SectionOffsets,
    slot: usize,
    count: usize,
) -> Result<Vec<u8>> {
    read_section(bytes, offsets, slot, count, 1, |bytes, offset| {
        bytes[offset]
    })
}

fn read_section<T>(
    bytes: &[u8],
    offsets: &Moc3SectionOffsets,
    slot: usize,
    count: usize,
    element_size: usize,
    read: impl Fn(&[u8], usize) -> T,
) -> Result<Vec<T>> {
    if count == 0 {
        return Ok(Vec::new());
    }

    let offset = offsets.section_offset(slot).ok_or_else(|| {
        invalid_art_meshes(format!("section slot {slot} is outside offset table"))
    })?;
    if offset == 0 {
        return Err(invalid_art_meshes(format!(
            "section slot {slot} has no offset"
        )));
    }

    let offset = usize::try_from(offset)
        .map_err(|_| invalid_art_meshes(format!("section slot {slot} offset is too large")))?;
    let byte_len = count
        .checked_mul(element_size)
        .ok_or_else(|| invalid_art_meshes(format!("section slot {slot} size overflows")))?;
    if bytes.len().saturating_sub(offset) < byte_len {
        return Err(invalid_art_meshes(format!(
            "section slot {slot} is incomplete"
        )));
    }

    let mut values = Vec::with_capacity(count);
    for index in 0..count {
        values.push(read(bytes, offset + index * element_size));
    }

    Ok(values)
}

fn validate_mesh_ranges(
    index: usize,
    mesh: Moc3ArtMeshInfo,
    uv_count: usize,
    position_index_count: usize,
    drawable_mask_count: usize,
) -> Result<()> {
    let uv_len = nonnegative_range_len(mesh.vertex_count, 2, "vertex count")?;
    validate_range(mesh.uv_begin_index, uv_len, uv_count, index, "uv")?;

    let position_len = nonnegative_range_len(mesh.position_index_count, 1, "position index count")?;
    validate_range(
        mesh.position_index_begin_index,
        position_len,
        position_index_count,
        index,
        "position index",
    )?;

    let mask_len = nonnegative_range_len(mesh.mask_count, 1, "mask count")?;
    validate_range(
        mesh.mask_begin_index,
        mask_len,
        drawable_mask_count,
        index,
        "mask",
    )
}

fn validate_keyform_ranges(
    mesh_index: usize,
    keyform_begin_index: i32,
    keyform_count: i32,
    vertex_count: i32,
    keyforms: &[Moc3ArtMeshKeyformInfo],
    position_count: usize,
) -> Result<()> {
    let keyform_len = nonnegative_range_len(keyform_count, 1, "art mesh keyform count")?;
    validate_range(
        keyform_begin_index,
        keyform_len,
        keyforms.len(),
        mesh_index,
        "keyform",
    )?;

    let keyform_begin_index = usize::try_from(keyform_begin_index).map_err(|_| {
        invalid_art_meshes(format!(
            "art mesh {mesh_index} keyform begin index is too large"
        ))
    })?;
    let position_len = nonnegative_range_len(vertex_count, 2, "vertex count")?;

    for keyform in keyforms.iter().skip(keyform_begin_index).take(keyform_len) {
        validate_range(
            keyform.position_begin_index,
            position_len,
            position_count,
            mesh_index,
            "keyform position",
        )?;
    }

    Ok(())
}

fn nonnegative_range_len(value: i32, scale: usize, name: &'static str) -> Result<usize> {
    if value < 0 {
        return Err(invalid_art_meshes(format!("{name} is negative")));
    }

    usize::try_from(value)
        .ok()
        .and_then(|value| value.checked_mul(scale))
        .ok_or_else(|| invalid_art_meshes(format!("{name} range size overflows")))
}

fn validate_range(
    begin: i32,
    len: usize,
    source_len: usize,
    mesh_index: usize,
    name: &'static str,
) -> Result<()> {
    if begin < 0 {
        return Err(invalid_art_meshes(format!(
            "art mesh {mesh_index} {name} begin index is negative"
        )));
    }

    let begin = usize::try_from(begin).map_err(|_| {
        invalid_art_meshes(format!("art mesh {mesh_index} {name} begin is too large"))
    })?;
    let end = begin.checked_add(len).ok_or_else(|| {
        invalid_art_meshes(format!("art mesh {mesh_index} {name} range overflows"))
    })?;

    if end > source_len {
        return Err(invalid_art_meshes(format!(
            "art mesh {mesh_index} {name} range is outside section"
        )));
    }

    Ok(())
}

fn to_usize(value: u32, name: &'static str) -> Result<usize> {
    usize::try_from(value).map_err(|_| invalid_art_meshes(format!("{name} is too large")))
}

fn invalid_art_meshes(message: impl Into<String>) -> Error {
    Error::InvalidMoc3 {
        message: message.into(),
    }
}
