use crate::{
    Error, Result,
    core::{
        Vector2, WarpInterpolation, rotation_deformer_transform_point,
        warp_deformer_transform_target,
    },
};

use super::{
    Endianness, Moc3ArtMeshKeyforms, Moc3ArtMeshes, Moc3CountInfo, Moc3DrawableMesh,
    Moc3DrawableVertex, Moc3Header, Moc3SectionOffsets, build_moc3_drawable_mesh,
};

const DEFORMER_PARENT_DEFORMER_INDICES_SLOT: usize = 16;
const DEFORMER_TYPES_SLOT: usize = 17;
const DEFORMER_SPECIFIC_INDICES_SLOT: usize = 18;
const WARP_KEYFORM_BINDING_BAND_INDICES_SLOT: usize = 19;
const WARP_KEYFORM_BEGIN_INDICES_SLOT: usize = 20;
const WARP_KEYFORM_COUNTS_SLOT: usize = 21;
const WARP_VERTEX_COUNTS_SLOT: usize = 22;
const WARP_ROWS_SLOT: usize = 23;
const WARP_COLS_SLOT: usize = 24;
const ROTATION_KEYFORM_BINDING_BAND_INDICES_SLOT: usize = 25;
const ROTATION_KEYFORM_BEGIN_INDICES_SLOT: usize = 26;
const ROTATION_KEYFORM_COUNTS_SLOT: usize = 27;
const ROTATION_BASE_ANGLES_SLOT: usize = 28;
const PARAMETER_DEFAULT_VALUES_SLOT: usize = 53;
const WARP_KEYFORM_POSITION_BEGIN_INDICES_SLOT: usize = 60;
const ROTATION_KEYFORM_ANGLES_SLOT: usize = 62;
const ROTATION_KEYFORM_ORIGIN_XS_SLOT: usize = 63;
const ROTATION_KEYFORM_ORIGIN_YS_SLOT: usize = 64;
const ROTATION_KEYFORM_SCALES_SLOT: usize = 65;
const ROTATION_KEYFORM_REFLECT_XS_SLOT: usize = 66;
const ROTATION_KEYFORM_REFLECT_YS_SLOT: usize = 67;
const KEYFORM_POSITION_XYS_SLOT: usize = 71;
const KEYFORM_BINDING_INDICES_SLOT: usize = 72;
const KEYFORM_BINDING_BAND_BEGIN_INDICES_SLOT: usize = 73;
const KEYFORM_BINDING_BAND_COUNTS_SLOT: usize = 74;
const KEYFORM_BINDING_KEYS_BEGIN_INDICES_SLOT: usize = 75;
const KEYFORM_BINDING_KEYS_COUNTS_SLOT: usize = 76;
const KEY_VALUES_SLOT: usize = 77;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Moc3DeformerKind {
    Warp,
    Rotation,
}

impl Moc3DeformerKind {
    fn from_raw(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Warp),
            1 => Some(Self::Rotation),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Moc3KeyformBindings {
    parameter_default_values: Vec<f32>,
    keyform_binding_indices: Vec<i32>,
    band_begin_indices: Vec<i32>,
    band_counts: Vec<i32>,
    keys_begin_indices: Vec<i32>,
    keys_counts: Vec<i32>,
    key_values: Vec<f32>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct Moc3KeyformSlot {
    local_index: usize,
    weight: f32,
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct KeyformAxis {
    left_index: usize,
    t: f32,
    stride: usize,
    key_count: usize,
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct InterpolatedRotation {
    angle_degrees: f32,
    translation: Vector2,
    scale: f32,
    flip_x: bool,
    flip_y: bool,
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct DeformerBounds {
    min: Vector2,
    max: Vector2,
}

impl Moc3KeyformBindings {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let header = Moc3Header::parse(bytes)?;
        let offsets = Moc3SectionOffsets::parse(bytes)?;
        let counts = Moc3CountInfo::parse(bytes)?;
        let endianness = header.endianness();

        Ok(Self {
            parameter_default_values: read_f32_section(
                bytes,
                &offsets,
                PARAMETER_DEFAULT_VALUES_SLOT,
                to_usize(counts.parameters(), "parameter count")?,
                endianness,
            )?,
            keyform_binding_indices: read_i32_section(
                bytes,
                &offsets,
                KEYFORM_BINDING_INDICES_SLOT,
                to_usize(
                    counts.parameter_binding_indices(),
                    "keyform binding index count",
                )?,
                endianness,
            )?,
            band_begin_indices: read_i32_section(
                bytes,
                &offsets,
                KEYFORM_BINDING_BAND_BEGIN_INDICES_SLOT,
                to_usize(counts.keyform_bindings(), "keyform binding band count")?,
                endianness,
            )?,
            band_counts: read_i32_section(
                bytes,
                &offsets,
                KEYFORM_BINDING_BAND_COUNTS_SLOT,
                to_usize(counts.keyform_bindings(), "keyform binding band count")?,
                endianness,
            )?,
            keys_begin_indices: read_i32_section(
                bytes,
                &offsets,
                KEYFORM_BINDING_KEYS_BEGIN_INDICES_SLOT,
                to_usize(counts.parameter_bindings(), "keyform binding count")?,
                endianness,
            )?,
            keys_counts: read_i32_section(
                bytes,
                &offsets,
                KEYFORM_BINDING_KEYS_COUNTS_SLOT,
                to_usize(counts.parameter_bindings(), "keyform binding count")?,
                endianness,
            )?,
            key_values: read_f32_section(
                bytes,
                &offsets,
                KEY_VALUES_SLOT,
                to_usize(counts.keys(), "key count")?,
                endianness,
            )?,
        })
    }

    pub fn default_keyform_index(&self, band_index: i32, keyform_count: usize) -> Option<usize> {
        self.default_keyform_slots(band_index, keyform_count)?
            .into_iter()
            .max_by(|left, right| left.weight.total_cmp(&right.weight))
            .map(|slot| slot.local_index)
    }

    fn default_keyform_slots(
        &self,
        band_index: i32,
        keyform_count: usize,
    ) -> Option<Vec<Moc3KeyformSlot>> {
        if keyform_count == 0 {
            return None;
        }

        let bindings = self.band_keyform_bindings(band_index)?;
        if bindings.is_empty() {
            return Some(vec![Moc3KeyformSlot {
                local_index: 0,
                weight: 1.0,
            }]);
        }

        let mut axes = Vec::with_capacity(bindings.len());
        let mut stride = 1usize;
        for &binding_index in bindings {
            let binding_index = usize::try_from(binding_index).ok()?;
            let keys = self.binding_keys(binding_index)?;
            let (left_index, t) =
                keyform_axis_interval(keys, *self.parameter_default_values.get(binding_index)?)?;
            axes.push(KeyformAxis {
                left_index,
                t,
                stride,
                key_count: keys.len(),
            });
            stride = stride.checked_mul(keys.len())?;
        }

        expand_keyform_slots(&axes, keyform_count)
    }

    fn band_keyform_bindings(&self, band_index: i32) -> Option<&[i32]> {
        let band_index = usize::try_from(band_index).ok()?;
        let begin = usize::try_from(*self.band_begin_indices.get(band_index)?).ok()?;
        let len = usize::try_from(*self.band_counts.get(band_index)?).ok()?;
        self.keyform_binding_indices
            .get(begin..begin.checked_add(len)?)
    }

    fn binding_keys(&self, binding_index: usize) -> Option<&[f32]> {
        let begin = usize::try_from(*self.keys_begin_indices.get(binding_index)?).ok()?;
        let len = usize::try_from(*self.keys_counts.get(binding_index)?).ok()?;
        self.key_values.get(begin..begin.checked_add(len)?)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Moc3Deformers {
    parent_deformer_indices: Vec<i32>,
    deformer_kinds: Vec<Moc3DeformerKind>,
    specific_indices: Vec<i32>,
    warp_keyform_binding_band_indices: Vec<i32>,
    warp_keyform_begin_indices: Vec<i32>,
    warp_keyform_counts: Vec<i32>,
    warp_vertex_counts: Vec<i32>,
    warp_rows: Vec<i32>,
    warp_cols: Vec<i32>,
    rotation_keyform_binding_band_indices: Vec<i32>,
    rotation_keyform_begin_indices: Vec<i32>,
    rotation_keyform_counts: Vec<i32>,
    rotation_base_angles: Vec<f32>,
    warp_keyform_position_begin_indices: Vec<i32>,
    rotation_keyform_angles: Vec<f32>,
    rotation_keyform_origin_xs: Vec<f32>,
    rotation_keyform_origin_ys: Vec<f32>,
    rotation_keyform_scales: Vec<f32>,
    rotation_keyform_reflect_xs: Vec<bool>,
    rotation_keyform_reflect_ys: Vec<bool>,
    keyform_position_xys: Vec<f32>,
}

impl Moc3Deformers {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let header = Moc3Header::parse(bytes)?;
        let offsets = Moc3SectionOffsets::parse(bytes)?;
        let counts = Moc3CountInfo::parse(bytes)?;
        let endianness = header.endianness();
        let deformer_count = to_usize(counts.deformers(), "deformer count")?;
        let warp_count = to_usize(counts.warp_deformers(), "warp deformer count")?;
        let rotation_count = to_usize(counts.rotation_deformers(), "rotation deformer count")?;
        let warp_keyform_count = to_usize(
            counts.warp_deformer_keyforms(),
            "warp deformer keyform count",
        )?;
        let rotation_keyform_count = to_usize(
            counts.rotation_deformer_keyforms(),
            "rotation deformer keyform count",
        )?;

        let deformer_types = read_i32_section(
            bytes,
            &offsets,
            DEFORMER_TYPES_SLOT,
            deformer_count,
            endianness,
        )?;
        let deformer_kinds = deformer_types
            .iter()
            .copied()
            .map(|value| {
                Moc3DeformerKind::from_raw(value)
                    .ok_or_else(|| invalid_deformers(format!("unsupported deformer type {value}")))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            parent_deformer_indices: read_i32_section(
                bytes,
                &offsets,
                DEFORMER_PARENT_DEFORMER_INDICES_SLOT,
                deformer_count,
                endianness,
            )?,
            deformer_kinds,
            specific_indices: read_i32_section(
                bytes,
                &offsets,
                DEFORMER_SPECIFIC_INDICES_SLOT,
                deformer_count,
                endianness,
            )?,
            warp_keyform_binding_band_indices: read_i32_section(
                bytes,
                &offsets,
                WARP_KEYFORM_BINDING_BAND_INDICES_SLOT,
                warp_count,
                endianness,
            )?,
            warp_keyform_begin_indices: read_i32_section(
                bytes,
                &offsets,
                WARP_KEYFORM_BEGIN_INDICES_SLOT,
                warp_count,
                endianness,
            )?,
            warp_keyform_counts: read_i32_section(
                bytes,
                &offsets,
                WARP_KEYFORM_COUNTS_SLOT,
                warp_count,
                endianness,
            )?,
            warp_vertex_counts: read_i32_section(
                bytes,
                &offsets,
                WARP_VERTEX_COUNTS_SLOT,
                warp_count,
                endianness,
            )?,
            warp_rows: read_i32_section(bytes, &offsets, WARP_ROWS_SLOT, warp_count, endianness)?,
            warp_cols: read_i32_section(bytes, &offsets, WARP_COLS_SLOT, warp_count, endianness)?,
            rotation_keyform_binding_band_indices: read_i32_section(
                bytes,
                &offsets,
                ROTATION_KEYFORM_BINDING_BAND_INDICES_SLOT,
                rotation_count,
                endianness,
            )?,
            rotation_keyform_begin_indices: read_i32_section(
                bytes,
                &offsets,
                ROTATION_KEYFORM_BEGIN_INDICES_SLOT,
                rotation_count,
                endianness,
            )?,
            rotation_keyform_counts: read_i32_section(
                bytes,
                &offsets,
                ROTATION_KEYFORM_COUNTS_SLOT,
                rotation_count,
                endianness,
            )?,
            rotation_base_angles: read_f32_section(
                bytes,
                &offsets,
                ROTATION_BASE_ANGLES_SLOT,
                rotation_count,
                endianness,
            )?,
            warp_keyform_position_begin_indices: read_i32_section(
                bytes,
                &offsets,
                WARP_KEYFORM_POSITION_BEGIN_INDICES_SLOT,
                warp_keyform_count,
                endianness,
            )?,
            rotation_keyform_angles: read_f32_section(
                bytes,
                &offsets,
                ROTATION_KEYFORM_ANGLES_SLOT,
                rotation_keyform_count,
                endianness,
            )?,
            rotation_keyform_origin_xs: read_f32_section(
                bytes,
                &offsets,
                ROTATION_KEYFORM_ORIGIN_XS_SLOT,
                rotation_keyform_count,
                endianness,
            )?,
            rotation_keyform_origin_ys: read_f32_section(
                bytes,
                &offsets,
                ROTATION_KEYFORM_ORIGIN_YS_SLOT,
                rotation_keyform_count,
                endianness,
            )?,
            rotation_keyform_scales: read_f32_section(
                bytes,
                &offsets,
                ROTATION_KEYFORM_SCALES_SLOT,
                rotation_keyform_count,
                endianness,
            )?,
            rotation_keyform_reflect_xs: read_bool_section(
                bytes,
                &offsets,
                ROTATION_KEYFORM_REFLECT_XS_SLOT,
                rotation_keyform_count,
                endianness,
            )?,
            rotation_keyform_reflect_ys: read_bool_section(
                bytes,
                &offsets,
                ROTATION_KEYFORM_REFLECT_YS_SLOT,
                rotation_keyform_count,
                endianness,
            )?,
            keyform_position_xys: read_f32_section(
                bytes,
                &offsets,
                KEYFORM_POSITION_XYS_SLOT,
                to_usize(counts.keyform_positions(), "keyform position count")?,
                endianness,
            )?,
        })
    }

    fn transform_vertices(
        &self,
        parent_deformer_index: i32,
        vertices: &mut [Vector2],
        bindings: &Moc3KeyformBindings,
    ) -> Option<()> {
        let mut deformer_index = parent_deformer_index;
        let mut guard = 0usize;
        let mut rotation_anchor = Vector2::default();
        let mut has_rotation_anchor = false;

        while deformer_index >= 0 {
            let index = usize::try_from(deformer_index).ok()?;
            match *self.deformer_kinds.get(index)? {
                Moc3DeformerKind::Warp => {
                    let warp_index = usize::try_from(*self.specific_indices.get(index)?).ok()?;
                    self.transform_warp(
                        warp_index,
                        vertices,
                        bindings,
                        has_rotation_anchor.then_some(&mut rotation_anchor),
                    )?;
                }
                Moc3DeformerKind::Rotation => {
                    let rotation_index =
                        usize::try_from(*self.specific_indices.get(index)?).ok()?;
                    self.transform_rotation(rotation_index, vertices, bindings)?;
                    rotation_anchor = self.rotation_anchor(rotation_index, bindings)?;
                    has_rotation_anchor = true;
                }
            }

            deformer_index = *self.parent_deformer_indices.get(index)?;
            guard += 1;
            if guard > self.parent_deformer_indices.len() {
                return None;
            }
        }

        Some(())
    }

    fn transform_warp(
        &self,
        warp_index: usize,
        vertices: &mut [Vector2],
        bindings: &Moc3KeyformBindings,
        mut rotation_anchor: Option<&mut Vector2>,
    ) -> Option<()> {
        let grid = self.interpolated_warp_grid(warp_index, bindings)?;
        let cols = usize::try_from(*self.warp_cols.get(warp_index)?).ok()?;
        let rows = usize::try_from(*self.warp_rows.get(warp_index)?).ok()?;
        let bounds = if rotation_anchor.is_some() {
            Some(grid_bounds(&grid)?)
        } else {
            None
        };
        let transformed_anchor = match rotation_anchor.as_ref() {
            Some(anchor) => Some(warp_deformer_transform_target(
                **anchor,
                &grid,
                cols,
                rows,
                WarpInterpolation::Quad,
            )?),
            None => None,
        };

        for vertex in &mut *vertices {
            *vertex = warp_deformer_transform_target(
                *vertex,
                &grid,
                cols,
                rows,
                WarpInterpolation::Quad,
            )?;
        }

        if let Some(anchor) = rotation_anchor.as_mut() {
            let transformed_anchor = transformed_anchor?;
            correct_vertices_around_anchor(vertices, transformed_anchor, bounds?)?;
            **anchor = transformed_anchor;
        }

        Some(())
    }

    fn transform_rotation(
        &self,
        rotation_index: usize,
        vertices: &mut [Vector2],
        bindings: &Moc3KeyformBindings,
    ) -> Option<()> {
        let rotation = self.interpolated_rotation(rotation_index, bindings)?;

        for vertex in vertices {
            *vertex = rotation_deformer_transform_point(
                *vertex,
                rotation.angle_degrees,
                rotation.scale,
                rotation.translation,
                rotation.flip_x,
                rotation.flip_y,
            );
        }

        Some(())
    }

    fn rotation_anchor(
        &self,
        rotation_index: usize,
        bindings: &Moc3KeyformBindings,
    ) -> Option<Vector2> {
        let rotation = self.interpolated_rotation(rotation_index, bindings)?;
        Some(rotation_deformer_transform_point(
            Vector2::default(),
            rotation.angle_degrees,
            rotation.scale,
            rotation.translation,
            rotation.flip_x,
            rotation.flip_y,
        ))
    }

    fn warp_keyform_slots(
        &self,
        warp_index: usize,
        bindings: &Moc3KeyformBindings,
    ) -> Option<Vec<Moc3KeyformSlot>> {
        let keyform_count = usize::try_from(*self.warp_keyform_counts.get(warp_index)?).ok()?;
        bindings.default_keyform_slots(
            *self.warp_keyform_binding_band_indices.get(warp_index)?,
            keyform_count,
        )
    }

    fn rotation_keyform_slots(
        &self,
        rotation_index: usize,
        bindings: &Moc3KeyformBindings,
    ) -> Option<Vec<Moc3KeyformSlot>> {
        let keyform_count =
            usize::try_from(*self.rotation_keyform_counts.get(rotation_index)?).ok()?;
        bindings.default_keyform_slots(
            *self
                .rotation_keyform_binding_band_indices
                .get(rotation_index)?,
            keyform_count,
        )
    }

    fn interpolated_warp_grid(
        &self,
        warp_index: usize,
        bindings: &Moc3KeyformBindings,
    ) -> Option<Vec<Vector2>> {
        let slots = self.warp_keyform_slots(warp_index, bindings)?;
        let begin = usize::try_from(*self.warp_keyform_begin_indices.get(warp_index)?).ok()?;
        let vertex_count = usize::try_from(*self.warp_vertex_counts.get(warp_index)?).ok()?;
        let mut grid = vec![Vector2::default(); vertex_count];

        for slot in slots {
            let keyform_index = begin.checked_add(slot.local_index)?;
            let source = self.warp_grid(warp_index, keyform_index)?;
            if source.len() != grid.len() {
                return None;
            }
            for (target, source) in grid.iter_mut().zip(source) {
                *target = Vector2::new(
                    target.x() + source.x() * slot.weight,
                    target.y() + source.y() * slot.weight,
                );
            }
        }

        Some(grid)
    }

    fn interpolated_rotation(
        &self,
        rotation_index: usize,
        bindings: &Moc3KeyformBindings,
    ) -> Option<InterpolatedRotation> {
        let slots = self.rotation_keyform_slots(rotation_index, bindings)?;
        let begin =
            usize::try_from(*self.rotation_keyform_begin_indices.get(rotation_index)?).ok()?;
        let mut angle = 0.0f32;
        let mut translation = Vector2::default();
        let mut scale = 0.0f32;
        let mut flip_x = 0.0f32;
        let mut flip_y = 0.0f32;

        for slot in slots {
            let keyform_index = begin.checked_add(slot.local_index)?;
            angle += *self.rotation_keyform_angles.get(keyform_index)? * slot.weight;
            translation = Vector2::new(
                translation.x()
                    + *self.rotation_keyform_origin_xs.get(keyform_index)? * slot.weight,
                translation.y()
                    + *self.rotation_keyform_origin_ys.get(keyform_index)? * slot.weight,
            );
            scale += *self.rotation_keyform_scales.get(keyform_index)? * slot.weight;
            flip_x += u8::from(*self.rotation_keyform_reflect_xs.get(keyform_index)?) as f32
                * slot.weight;
            flip_y += u8::from(*self.rotation_keyform_reflect_ys.get(keyform_index)?) as f32
                * slot.weight;
        }

        Some(InterpolatedRotation {
            angle_degrees: *self.rotation_base_angles.get(rotation_index)? + angle,
            translation,
            scale,
            flip_x: interpolate_bool(flip_x),
            flip_y: interpolate_bool(flip_y),
        })
    }

    fn warp_grid(&self, warp_index: usize, keyform_index: usize) -> Option<Vec<Vector2>> {
        let start = usize::try_from(
            *self
                .warp_keyform_position_begin_indices
                .get(keyform_index)?,
        )
        .ok()?;
        let vertex_count = usize::try_from(*self.warp_vertex_counts.get(warp_index)?).ok()?;
        let len = vertex_count.checked_mul(2)?;
        let values = self
            .keyform_position_xys
            .get(start..start.checked_add(len)?)?;

        Some(
            values
                .chunks_exact(2)
                .map(|xy| Vector2::new(xy[0], xy[1]))
                .collect(),
        )
    }
}

pub fn build_moc3_drawable_meshes_for_default_pose(
    art_meshes: &Moc3ArtMeshes,
    art_mesh_keyforms: &Moc3ArtMeshKeyforms,
    deformers: &Moc3Deformers,
    bindings: &Moc3KeyformBindings,
) -> Option<Vec<Moc3DrawableMesh>> {
    let mut meshes = Vec::with_capacity(art_meshes.meshes().len());
    for art_mesh_index in 0..art_meshes.meshes().len() {
        meshes.push(build_moc3_drawable_mesh_for_default_pose(
            art_meshes,
            art_mesh_keyforms,
            deformers,
            bindings,
            art_mesh_index,
        )?);
    }

    Some(meshes)
}

fn build_moc3_drawable_mesh_for_default_pose(
    art_meshes: &Moc3ArtMeshes,
    art_mesh_keyforms: &Moc3ArtMeshKeyforms,
    deformers: &Moc3Deformers,
    bindings: &Moc3KeyformBindings,
    art_mesh_index: usize,
) -> Option<Moc3DrawableMesh> {
    let keyform_count = art_mesh_keyforms.art_mesh_keyforms(art_mesh_index)?.len();
    let slots = bindings.default_keyform_slots(
        art_meshes.art_mesh_keyform_binding_band_index(art_mesh_index)?,
        keyform_count,
    )?;
    let base_local_keyform_index = slots.first()?.local_index;
    let mesh = build_moc3_drawable_mesh(
        art_meshes,
        art_mesh_keyforms,
        art_mesh_index,
        base_local_keyform_index,
    )?;
    let opacity = interpolate_art_mesh_opacity(art_mesh_keyforms, art_mesh_index, &slots)?;
    let draw_order = interpolate_art_mesh_draw_order(art_mesh_keyforms, art_mesh_index, &slots)?;
    let mut positions = interpolate_art_mesh_positions(art_mesh_keyforms, art_mesh_index, &slots)?;

    deformers.transform_vertices(
        art_meshes.art_mesh_parent_deformer_index(art_mesh_index)?,
        &mut positions,
        bindings,
    )?;

    let vertices = mesh
        .vertices()
        .iter()
        .zip(positions)
        .map(|(vertex, position)| {
            Moc3DrawableVertex::new([position.x(), -position.y()], vertex.uv())
        })
        .collect();

    Some(Moc3DrawableMesh::from_parts_with_render_order(
        mesh.texture_index(),
        mesh.drawable_flags(),
        opacity,
        draw_order,
        mesh.render_order(),
        vertices,
        mesh.indices().to_vec(),
        mesh.masks().to_vec(),
    ))
}

fn interpolate_art_mesh_positions(
    keyforms: &Moc3ArtMeshKeyforms,
    art_mesh_index: usize,
    slots: &[Moc3KeyformSlot],
) -> Option<Vec<Vector2>> {
    let first = keyforms.art_mesh_keyform_positions(art_mesh_index, slots.first()?.local_index)?;
    let mut out = vec![Vector2::default(); first.len().checked_div(2)?];

    for slot in slots {
        let positions = keyforms.art_mesh_keyform_positions(art_mesh_index, slot.local_index)?;
        if positions.len() != first.len() || positions.len() % 2 != 0 {
            return None;
        }
        for (target, position) in out.iter_mut().zip(positions.chunks_exact(2)) {
            *target = Vector2::new(
                target.x() + position[0] * slot.weight,
                target.y() + position[1] * slot.weight,
            );
        }
    }

    Some(out)
}

fn interpolate_art_mesh_opacity(
    keyforms: &Moc3ArtMeshKeyforms,
    art_mesh_index: usize,
    slots: &[Moc3KeyformSlot],
) -> Option<f32> {
    interpolate_art_mesh_scalar(keyforms, art_mesh_index, slots, |keyform| keyform.opacity())
}

fn interpolate_art_mesh_draw_order(
    keyforms: &Moc3ArtMeshKeyforms,
    art_mesh_index: usize,
    slots: &[Moc3KeyformSlot],
) -> Option<f32> {
    interpolate_art_mesh_scalar(keyforms, art_mesh_index, slots, |keyform| {
        keyform.draw_order()
    })
}

fn interpolate_art_mesh_scalar(
    keyforms: &Moc3ArtMeshKeyforms,
    art_mesh_index: usize,
    slots: &[Moc3KeyformSlot],
    value: impl Fn(super::Moc3ArtMeshKeyformInfo) -> f32,
) -> Option<f32> {
    let keyforms = keyforms.art_mesh_keyforms(art_mesh_index)?;
    let mut out = 0.0f32;
    for slot in slots {
        out += value(*keyforms.get(slot.local_index)?) * slot.weight;
    }
    Some(out)
}

fn keyform_axis_interval(keys: &[f32], value: f32) -> Option<(usize, f32)> {
    if keys.is_empty() {
        return None;
    }

    if value <= keys[0] {
        return Some((0, 0.0));
    }

    let last_index = keys.len() - 1;
    if value >= keys[last_index] {
        return Some((last_index, 0.0));
    }

    for index in 0..keys.len().saturating_sub(1) {
        if keys[index] <= value && value <= keys[index + 1] {
            let width = keys[index + 1] - keys[index];
            if width.abs() <= f32::EPSILON {
                return Some((index, 0.0));
            }
            return Some((index, (value - keys[index]) / width));
        }
    }

    Some((last_index, 0.0))
}

fn expand_keyform_slots(
    axes: &[KeyformAxis],
    keyform_count: usize,
) -> Option<Vec<Moc3KeyformSlot>> {
    let active_count = axes.iter().filter(|axis| axis.t != 0.0).count();
    let slot_count = 1usize.checked_shl(u32::try_from(active_count).ok()?)?;
    let mut slots = Vec::with_capacity(slot_count);

    for mask in 0..slot_count {
        let mut local_index = 0usize;
        let mut weight = 1.0f32;
        let mut bit = 0usize;

        for axis in axes {
            let use_right = if axis.t == 0.0 {
                false
            } else {
                let use_right = ((mask >> bit) & 1) != 0;
                bit += 1;
                use_right
            };

            let axis_index = if use_right {
                axis.left_index.checked_add(1)?
            } else {
                axis.left_index
            };
            if axis_index >= axis.key_count {
                return None;
            }

            local_index = local_index.checked_add(axis_index.checked_mul(axis.stride)?)?;
            weight *= if axis.t == 0.0 {
                1.0
            } else if use_right {
                axis.t
            } else {
                1.0 - axis.t
            };
        }

        if local_index >= keyform_count {
            return None;
        }

        slots.push(Moc3KeyformSlot {
            local_index,
            weight,
        });
    }

    Some(slots)
}

fn interpolate_bool(value: f32) -> bool {
    (value + 0.001).trunc() != 0.0
}

fn grid_bounds(grid: &[Vector2]) -> Option<DeformerBounds> {
    let first = *grid.first()?;
    let mut bounds = DeformerBounds {
        min: first,
        max: first,
    };

    for &point in &grid[1..] {
        bounds.min = Vector2::new(bounds.min.x().min(point.x()), bounds.min.y().min(point.y()));
        bounds.max = Vector2::new(bounds.max.x().max(point.x()), bounds.max.y().max(point.y()));
    }

    Some(bounds)
}

fn correct_vertices_around_anchor(
    vertices: &mut [Vector2],
    anchor: Vector2,
    bounds: DeformerBounds,
) -> Option<()> {
    let width = bounds.max.x() - bounds.min.x();
    let height = bounds.max.y() - bounds.min.y();
    if width.abs() <= f32::EPSILON || height.abs() <= f32::EPSILON {
        return None;
    }

    for vertex in vertices {
        *vertex = Vector2::new(
            anchor.x() + (vertex.x() - anchor.x()) / width,
            anchor.y() + (vertex.y() - anchor.y()) / height,
        );
    }

    Some(())
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

fn read_bool_section(
    bytes: &[u8],
    offsets: &Moc3SectionOffsets,
    slot: usize,
    count: usize,
    endianness: Endianness,
) -> Result<Vec<bool>> {
    read_i32_section(bytes, offsets, slot, count, endianness)
        .map(|values| values.into_iter().map(|value| value == 1).collect())
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

    let offset = offsets
        .section_offset(slot)
        .ok_or_else(|| invalid_deformers(format!("section slot {slot} is outside offset table")))?;
    if offset == 0 {
        return Err(invalid_deformers(format!(
            "section slot {slot} has no offset"
        )));
    }

    let offset = usize::try_from(offset)
        .map_err(|_| invalid_deformers(format!("section slot {slot} offset is too large")))?;
    let byte_len = count
        .checked_mul(element_size)
        .ok_or_else(|| invalid_deformers(format!("section slot {slot} size overflows")))?;
    if bytes.len().saturating_sub(offset) < byte_len {
        return Err(invalid_deformers(format!(
            "section slot {slot} is incomplete"
        )));
    }

    let mut values = Vec::with_capacity(count);
    for index in 0..count {
        values.push(read(bytes, offset + index * element_size));
    }

    Ok(values)
}

fn to_usize(value: u32, name: &'static str) -> Result<usize> {
    usize::try_from(value).map_err(|_| invalid_deformers(format!("{name} is too large")))
}

fn invalid_deformers(message: impl Into<String>) -> Error {
    Error::InvalidMoc3 {
        message: message.into(),
    }
}