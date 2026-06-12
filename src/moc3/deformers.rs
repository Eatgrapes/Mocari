use crate::{
    Result,
    core::{
        KeyformAxis, Vector2, WarpInterpolation, compute_keyform_axis_interval,
        expand_keyform_runtime_slots, rotation_deformer_transform_point,
        warp_deformer_transform_target,
    },
};

use super::{
    Moc3ArtMeshKeyforms, Moc3ArtMeshes, Moc3CountInfo, Moc3DrawableMesh, Moc3DrawableVertex,
    Moc3Header, Moc3Ids, Moc3OffscreenInfo, Moc3SectionOffsets, build_moc3_drawable_mesh,
    parse::{invalid_moc3, read_bool_section, read_f32_section, read_i32_section, to_usize},
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
struct InterpolatedRotation {
    angle_degrees: f32,
    translation: Vector2,
    scale: f32,
    flip_x: bool,
    flip_y: bool,
}

const ROTATION_DERIVATIVE_STEP: f32 = 0.1;

#[derive(Debug, Clone, PartialEq)]
enum ComposedDeformer {
    Warp(ComposedWarp),
    Rotation(ComposedRotation),
}

#[derive(Debug, Clone, PartialEq)]
struct ComposedWarp {
    grid: Vec<Vector2>,
    cols: usize,
    rows: usize,
    scale_accum: f32,
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct ComposedRotation {
    origin: Vector2,
    angle_degrees: f32,
    scale: f32,
    flip_x: bool,
    flip_y: bool,
    scale_accum: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComposedDeformers {
    deformers: Vec<ComposedDeformer>,
}

impl ComposedDeformers {
    fn transform_vertices(
        &self,
        parent_deformer_index: i32,
        vertices: &mut [Vector2],
    ) -> Option<()> {
        if parent_deformer_index < 0 {
            return Some(());
        }
        let index = usize::try_from(parent_deformer_index).ok()?;
        for vertex in vertices {
            *vertex = apply_one(self.deformers.get(index)?, *vertex)?;
        }
        Some(())
    }
}

fn apply_one(deformer: &ComposedDeformer, point: Vector2) -> Option<Vector2> {
    match deformer {
        ComposedDeformer::Warp(warp) => warp_deformer_transform_target(
            point,
            &warp.grid,
            warp.cols,
            warp.rows,
            WarpInterpolation::Quad,
        ),
        ComposedDeformer::Rotation(rotation) => Some(rotation_deformer_transform_point(
            point,
            rotation.angle_degrees,
            rotation.scale,
            rotation.origin,
            rotation.flip_x,
            rotation.flip_y,
        )),
    }
}

fn apply_composed_parent(
    composed: &[Option<ComposedDeformer>],
    parent_index: i32,
    point: Vector2,
) -> Option<Vector2> {
    if parent_index < 0 {
        return Some(point);
    }
    let index = usize::try_from(parent_index).ok()?;
    let parent = composed.get(index)?.as_ref()?;
    apply_one(parent, point)
}

fn parent_scale_accum(composed: &[Option<ComposedDeformer>], parent_index: i32) -> f32 {
    if parent_index < 0 {
        return 1.0;
    }
    let index = match usize::try_from(parent_index) {
        Ok(value) => value,
        Err(_) => return 1.0,
    };
    match composed.get(index).and_then(|slot| slot.as_ref()) {
        Some(ComposedDeformer::Warp(warp)) => warp.scale_accum,
        Some(ComposedDeformer::Rotation(rotation)) => rotation.scale_accum,
        None => 1.0,
    }
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

        if band_index < 0 {
            return Some(vec![Moc3KeyformSlot {
                local_index: 0,
                weight: 1.0,
            }]);
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
            let parameter_default = self
                .parameter_default_values
                .get(binding_index)
                .copied()
                .unwrap_or(0.0);
            let interval = compute_keyform_axis_interval(keys, parameter_default)?;
            let active_index = interval.left_index() + usize::from(interval.t() != 0.0);
            if active_index >= keys.len() {
                return None;
            }
            axes.push(KeyformAxis::new(
                interval.left_index(),
                interval.t(),
                stride,
            ));
            stride = stride.checked_mul(keys.len())?;
        }

        let slots = expand_keyform_runtime_slots(&axes)
            .into_iter()
            .map(|slot| {
                (slot.flat_index() < keyform_count).then_some(Moc3KeyformSlot {
                    local_index: slot.flat_index(),
                    weight: slot.weight(),
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(slots)
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
                    .ok_or_else(|| invalid_moc3(format!("unsupported deformer type {value}")))
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

    fn compose(&self, bindings: &Moc3KeyformBindings) -> Option<ComposedDeformers> {
        let count = self.deformer_kinds.len();
        let mut order: Vec<usize> = (0..count).collect();
        order.sort_by_key(|&idx| self.deformer_depth(idx));

        let mut composed: Vec<Option<ComposedDeformer>> = vec![None; count];
        for idx in order {
            let parent = *self.parent_deformer_indices.get(idx)?;
            let specific = usize::try_from(*self.specific_indices.get(idx)?).ok()?;
            let composed_deformer = match *self.deformer_kinds.get(idx)? {
                Moc3DeformerKind::Warp => {
                    let mut grid = self.interpolated_warp_grid(specific, bindings)?;
                    let cols = usize::try_from(*self.warp_cols.get(specific)?).ok()?;
                    let rows = usize::try_from(*self.warp_rows.get(specific)?).ok()?;
                    for point in &mut grid {
                        *point = apply_composed_parent(&composed, parent, *point)?;
                    }
                    let scale_accum = parent_scale_accum(&composed, parent);
                    ComposedDeformer::Warp(ComposedWarp {
                        grid,
                        cols,
                        rows,
                        scale_accum,
                    })
                }
                Moc3DeformerKind::Rotation => {
                    let rotation = self.interpolated_rotation(specific, bindings)?;
                    let origin = apply_composed_parent(&composed, parent, rotation.translation)?;
                    let stepped = apply_composed_parent(
                        &composed,
                        parent,
                        Vector2::new(
                            rotation.translation.x() + ROTATION_DERIVATIVE_STEP,
                            rotation.translation.y(),
                        ),
                    )?;
                    let parent_angle = (stepped.y() - origin.y()).atan2(stepped.x() - origin.x());
                    let scale_accum = parent_scale_accum(&composed, parent);
                    ComposedDeformer::Rotation(ComposedRotation {
                        origin,
                        angle_degrees: rotation.angle_degrees + parent_angle.to_degrees(),
                        scale: rotation.scale * scale_accum,
                        flip_x: rotation.flip_x,
                        flip_y: rotation.flip_y,
                        scale_accum: rotation.scale * scale_accum,
                    })
                }
            };
            *composed.get_mut(idx)? = Some(composed_deformer);
        }

        Some(ComposedDeformers {
            deformers: composed.into_iter().collect::<Option<Vec<_>>>()?,
        })
    }

    fn deformer_depth(&self, index: usize) -> usize {
        let mut depth = 0usize;
        let mut current = index;
        loop {
            let parent = self
                .parent_deformer_indices
                .get(current)
                .copied()
                .unwrap_or(-1);
            if parent < 0 {
                break;
            }
            current = match usize::try_from(parent) {
                Ok(value) => value,
                Err(_) => break,
            };
            depth += 1;
            if depth > self.parent_deformer_indices.len() {
                break;
            }
        }
        depth
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
    let composed = deformers.compose(bindings)?;
    let mut meshes = Vec::with_capacity(art_meshes.meshes().len());
    for art_mesh_index in 0..art_meshes.meshes().len() {
        meshes.push(build_moc3_drawable_mesh_for_default_pose(
            art_meshes,
            art_mesh_keyforms,
            &composed,
            bindings,
            art_mesh_index,
        )?);
    }

    Some(meshes)
}

pub fn build_moc3_drawable_meshes_for_default_pose_with_offscreen_state(
    art_meshes: &Moc3ArtMeshes,
    art_mesh_keyforms: &Moc3ArtMeshKeyforms,
    deformers: &Moc3Deformers,
    bindings: &Moc3KeyformBindings,
    ids: &Moc3Ids,
    offscreen: &Moc3OffscreenInfo,
) -> Option<Vec<Moc3DrawableMesh>> {
    let mut meshes = build_moc3_drawable_meshes_for_default_pose(
        art_meshes,
        art_mesh_keyforms,
        deformers,
        bindings,
    )?;

    for drawable_index in offscreen.effect_source_drawable_indices(ids) {
        meshes.get_mut(drawable_index)?.set_opacity(0.0);
    }

    Some(meshes)
}

fn build_moc3_drawable_mesh_for_default_pose(
    art_meshes: &Moc3ArtMeshes,
    art_mesh_keyforms: &Moc3ArtMeshKeyforms,
    composed: &ComposedDeformers,
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

    composed.transform_vertices(
        art_meshes.art_mesh_parent_deformer_index(art_mesh_index)?,
        &mut positions,
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

fn interpolate_bool(value: f32) -> bool {
    (value + 0.001).trunc() != 0.0
}
