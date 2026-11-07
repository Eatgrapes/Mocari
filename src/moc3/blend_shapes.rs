use crate::{Result, core::compute_keyform_axis_interval};

use super::{
    Moc3CountInfo, Moc3Header, Moc3SectionOffsets,
    parse::{invalid_moc3, read_f32_section, read_i32_section, to_usize},
};

const PARAMETER_TYPES_SLOT: usize = 114;
const BLEND_SHAPE_PARAMETER_BINDING_BEGIN_INDICES_SLOT: usize = 115;
const BLEND_SHAPE_PARAMETER_BINDING_COUNTS_SLOT: usize = 116;
const BLEND_SHAPE_PARAMETER_KEYS_BEGIN_INDICES_SLOT: usize = 117;
const BLEND_SHAPE_PARAMETER_KEYS_COUNTS_SLOT: usize = 118;
const BLEND_SHAPE_PARAMETER_BASE_KEY_INDICES_SLOT: usize = 119;
const BLEND_SHAPE_KEYFORM_PARAMETER_BINDING_INDICES_SLOT: usize = 120;
const BLEND_SHAPE_KEYFORM_INDICES_SLOT: usize = 121;
const BLEND_SHAPE_KEYFORM_COUNTS_SLOT: usize = 122;
const BLEND_SHAPE_KEYFORM_CONSTRAINT_BEGIN_INDICES_SLOT: usize = 123;
const BLEND_SHAPE_KEYFORM_CONSTRAINT_COUNTS_SLOT: usize = 124;
const BLEND_SHAPE_WARP_TARGET_INDICES_SLOT: usize = 125;
const BLEND_SHAPE_WARP_KEYFORM_BEGIN_INDICES_SLOT: usize = 126;
const BLEND_SHAPE_WARP_KEYFORM_COUNTS_SLOT: usize = 127;
const BLEND_SHAPE_ART_MESH_TARGET_INDICES_SLOT: usize = 128;
const BLEND_SHAPE_ART_MESH_KEYFORM_BEGIN_INDICES_SLOT: usize = 129;
const BLEND_SHAPE_ART_MESH_KEYFORM_COUNTS_SLOT: usize = 130;
const BLEND_SHAPE_CONSTRAINT_INDICES_SLOT: usize = 131;
const BLEND_SHAPE_CONSTRAINT_PARAMETER_INDICES_SLOT: usize = 132;
const BLEND_SHAPE_CONSTRAINT_VALUE_BEGIN_INDICES_SLOT: usize = 133;
const BLEND_SHAPE_CONSTRAINT_VALUE_COUNTS_SLOT: usize = 134;
const KEY_VALUES_SLOT: usize = 77;
const BLEND_SHAPE_CONSTRAINT_KEYS_SLOT: usize = 135;
const BLEND_SHAPE_CONSTRAINT_WEIGHTS_SLOT: usize = 136;
const BLEND_SHAPE_ROTATION_TARGET_INDICES_SLOT: usize = 146;
const BLEND_SHAPE_ROTATION_KEYFORM_BEGIN_INDICES_SLOT: usize = 147;
const BLEND_SHAPE_ROTATION_KEYFORM_COUNTS_SLOT: usize = 148;

#[derive(Debug, Default, Clone, PartialEq)]
pub(super) struct Moc3BlendShapes {
    binding_parameter_indices: Vec<usize>,
    parameter_keys_begin_indices: Vec<i32>,
    parameter_keys_counts: Vec<i32>,
    parameter_base_key_indices: Vec<i32>,
    parameter_keys: Vec<f32>,
    keyform_parameter_binding_indices: Vec<i32>,
    keyform_indices: Vec<i32>,
    keyform_counts: Vec<i32>,
    keyform_constraint_begin_indices: Vec<i32>,
    keyform_constraint_counts: Vec<i32>,
    warp_target_indices: Vec<i32>,
    warp_keyform_begin_indices: Vec<i32>,
    warp_keyform_counts: Vec<i32>,
    art_mesh_target_indices: Vec<i32>,
    art_mesh_keyform_begin_indices: Vec<i32>,
    art_mesh_keyform_counts: Vec<i32>,
    rotation_target_indices: Vec<i32>,
    rotation_keyform_begin_indices: Vec<i32>,
    rotation_keyform_counts: Vec<i32>,
    constraint_indices: Vec<i32>,
    constraint_parameter_indices: Vec<i32>,
    constraint_value_begin_indices: Vec<i32>,
    constraint_value_counts: Vec<i32>,
    constraint_keys: Vec<f32>,
    constraint_weights: Vec<f32>,
}

impl Moc3BlendShapes {
    pub(super) fn parse(bytes: &[u8]) -> Result<Self> {
        let header = Moc3Header::parse(bytes)?;
        let offsets = Moc3SectionOffsets::parse(bytes)?;
        if offsets.section_offset(PARAMETER_TYPES_SLOT) == Some(0) {
            return Ok(Self::default());
        }

        let counts = Moc3CountInfo::parse(bytes)?;
        let endianness = header.endianness();
        let read_i32 = |slot, count| read_i32_section(bytes, &offsets, slot, count, endianness);
        let read_f32 = |slot, count| read_f32_section(bytes, &offsets, slot, count, endianness);
        let parameter_count = to_usize(counts.parameters(), "parameter count")?;
        let parameter_binding_count = to_usize(
            counts.blend_shape_parameter_bindings(),
            "blend shape parameter binding count",
        )?;
        let keyform_binding_count = to_usize(
            counts.blend_shape_keyform_bindings(),
            "blend shape keyform binding count",
        )?;
        let constraint_count = to_usize(
            counts.blend_shape_constraints(),
            "blend shape constraint count",
        )?;
        let constraint_value_count = to_usize(
            counts.blend_shape_constraint_values(),
            "blend shape constraint value count",
        )?;
        let parameter_types = read_i32(PARAMETER_TYPES_SLOT, parameter_count)?;
        let parameter_binding_begin_indices = read_i32(
            BLEND_SHAPE_PARAMETER_BINDING_BEGIN_INDICES_SLOT,
            parameter_count,
        )?;
        let parameter_binding_counts =
            read_i32(BLEND_SHAPE_PARAMETER_BINDING_COUNTS_SLOT, parameter_count)?;
        let binding_parameter_indices = expand_binding_parameter_indices(
            &parameter_types,
            &parameter_binding_begin_indices,
            &parameter_binding_counts,
        )
        .ok_or_else(|| invalid_moc3("invalid blend shape parameter binding ranges"))?;
        if binding_parameter_indices.len() != parameter_binding_count {
            return Err(invalid_moc3(
                "blend shape parameter binding count does not match parameter ranges",
            ));
        }
        let parameter_keys_counts = read_i32(
            BLEND_SHAPE_PARAMETER_KEYS_COUNTS_SLOT,
            parameter_binding_count,
        )?;

        Ok(Self {
            binding_parameter_indices,
            parameter_keys_begin_indices: read_i32(
                BLEND_SHAPE_PARAMETER_KEYS_BEGIN_INDICES_SLOT,
                parameter_binding_count,
            )?,
            parameter_base_key_indices: read_i32(
                BLEND_SHAPE_PARAMETER_BASE_KEY_INDICES_SLOT,
                parameter_binding_count,
            )?,
            parameter_keys: read_f32(KEY_VALUES_SLOT, to_usize(counts.keys(), "key count")?)?,
            keyform_parameter_binding_indices: read_i32(
                BLEND_SHAPE_KEYFORM_PARAMETER_BINDING_INDICES_SLOT,
                keyform_binding_count,
            )?,
            keyform_indices: read_i32(BLEND_SHAPE_KEYFORM_INDICES_SLOT, keyform_binding_count)?,
            keyform_counts: read_i32(BLEND_SHAPE_KEYFORM_COUNTS_SLOT, keyform_binding_count)?,
            keyform_constraint_begin_indices: read_i32(
                BLEND_SHAPE_KEYFORM_CONSTRAINT_BEGIN_INDICES_SLOT,
                keyform_binding_count,
            )?,
            keyform_constraint_counts: read_i32(
                BLEND_SHAPE_KEYFORM_CONSTRAINT_COUNTS_SLOT,
                keyform_binding_count,
            )?,
            warp_target_indices: read_i32(
                BLEND_SHAPE_WARP_TARGET_INDICES_SLOT,
                to_usize(
                    counts.blend_shapes_warp_deformers(),
                    "blend shape warp deformer count",
                )?,
            )?,
            warp_keyform_begin_indices: read_i32(
                BLEND_SHAPE_WARP_KEYFORM_BEGIN_INDICES_SLOT,
                to_usize(
                    counts.blend_shapes_warp_deformers(),
                    "blend shape warp deformer count",
                )?,
            )?,
            warp_keyform_counts: read_i32(
                BLEND_SHAPE_WARP_KEYFORM_COUNTS_SLOT,
                to_usize(
                    counts.blend_shapes_warp_deformers(),
                    "blend shape warp deformer count",
                )?,
            )?,
            art_mesh_target_indices: read_i32(
                BLEND_SHAPE_ART_MESH_TARGET_INDICES_SLOT,
                to_usize(
                    counts.blend_shapes_art_meshes(),
                    "blend shape art mesh count",
                )?,
            )?,
            art_mesh_keyform_begin_indices: read_i32(
                BLEND_SHAPE_ART_MESH_KEYFORM_BEGIN_INDICES_SLOT,
                to_usize(
                    counts.blend_shapes_art_meshes(),
                    "blend shape art mesh count",
                )?,
            )?,
            art_mesh_keyform_counts: read_i32(
                BLEND_SHAPE_ART_MESH_KEYFORM_COUNTS_SLOT,
                to_usize(
                    counts.blend_shapes_art_meshes(),
                    "blend shape art mesh count",
                )?,
            )?,
            rotation_target_indices: read_i32(
                BLEND_SHAPE_ROTATION_TARGET_INDICES_SLOT,
                to_usize(
                    counts.blend_shapes_rotation_deformers(),
                    "blend shape rotation deformer count",
                )?,
            )?,
            rotation_keyform_begin_indices: read_i32(
                BLEND_SHAPE_ROTATION_KEYFORM_BEGIN_INDICES_SLOT,
                to_usize(
                    counts.blend_shapes_rotation_deformers(),
                    "blend shape rotation deformer count",
                )?,
            )?,
            rotation_keyform_counts: read_i32(
                BLEND_SHAPE_ROTATION_KEYFORM_COUNTS_SLOT,
                to_usize(
                    counts.blend_shapes_rotation_deformers(),
                    "blend shape rotation deformer count",
                )?,
            )?,
            constraint_indices: read_i32(
                BLEND_SHAPE_CONSTRAINT_INDICES_SLOT,
                to_usize(
                    counts.blend_shape_constraint_indices(),
                    "blend shape constraint index count",
                )?,
            )?,
            constraint_parameter_indices: read_i32(
                BLEND_SHAPE_CONSTRAINT_PARAMETER_INDICES_SLOT,
                constraint_count,
            )?,
            constraint_value_begin_indices: read_i32(
                BLEND_SHAPE_CONSTRAINT_VALUE_BEGIN_INDICES_SLOT,
                constraint_count,
            )?,
            constraint_value_counts: read_i32(
                BLEND_SHAPE_CONSTRAINT_VALUE_COUNTS_SLOT,
                constraint_count,
            )?,
            parameter_keys_counts,
            constraint_keys: read_f32(BLEND_SHAPE_CONSTRAINT_KEYS_SLOT, constraint_value_count)?,
            constraint_weights: read_f32(
                BLEND_SHAPE_CONSTRAINT_WEIGHTS_SLOT,
                constraint_value_count,
            )?,
        })
    }

    pub(super) fn for_each_warp_keyform(
        &self,
        warp_index: usize,
        parameter_values: &[f32],
        apply: impl FnMut(usize, f32) -> Option<()>,
    ) -> Option<()> {
        self.for_each_target_keyform(
            &self.warp_target_indices,
            &self.warp_keyform_begin_indices,
            &self.warp_keyform_counts,
            warp_index,
            parameter_values,
            apply,
        )
    }

    pub(super) fn for_each_art_mesh_keyform(
        &self,
        art_mesh_index: usize,
        parameter_values: &[f32],
        apply: impl FnMut(usize, f32) -> Option<()>,
    ) -> Option<()> {
        self.for_each_target_keyform(
            &self.art_mesh_target_indices,
            &self.art_mesh_keyform_begin_indices,
            &self.art_mesh_keyform_counts,
            art_mesh_index,
            parameter_values,
            apply,
        )
    }

    pub(super) fn for_each_rotation_keyform(
        &self,
        rotation_index: usize,
        parameter_values: &[f32],
        apply: impl FnMut(usize, f32) -> Option<()>,
    ) -> Option<()> {
        self.for_each_target_keyform(
            &self.rotation_target_indices,
            &self.rotation_keyform_begin_indices,
            &self.rotation_keyform_counts,
            rotation_index,
            parameter_values,
            apply,
        )
    }

    fn for_each_target_keyform(
        &self,
        target_indices: &[i32],
        begin_indices: &[i32],
        counts: &[i32],
        target_index: usize,
        parameter_values: &[f32],
        mut apply: impl FnMut(usize, f32) -> Option<()>,
    ) -> Option<()> {
        for (shape, &target) in target_indices.iter().enumerate() {
            if usize::try_from(target).ok()? != target_index {
                continue;
            }
            let begin = usize::try_from(*begin_indices.get(shape)?).ok()?;
            let count = usize::try_from(*counts.get(shape)?).ok()?;
            for keyform_binding in begin..begin.checked_add(count)? {
                self.for_each_active_keyform(keyform_binding, parameter_values, &mut apply)?;
            }
        }
        Some(())
    }

    fn for_each_active_keyform(
        &self,
        keyform_binding: usize,
        parameter_values: &[f32],
        apply: &mut impl FnMut(usize, f32) -> Option<()>,
    ) -> Option<()> {
        let parameter_binding = usize::try_from(
            *self
                .keyform_parameter_binding_indices
                .get(keyform_binding)?,
        )
        .ok()?;
        let keyform_count = usize::try_from(*self.keyform_counts.get(keyform_binding)?).ok()?;
        let keys = self.parameter_keys(parameter_binding)?;
        let base =
            usize::try_from(*self.parameter_base_key_indices.get(parameter_binding)?).ok()?;
        if keyform_count != keys.len() || base >= keyform_count {
            return None;
        }

        let parameter_index = *self.binding_parameter_indices.get(parameter_binding)?;
        let interval =
            compute_keyform_axis_interval(keys, *parameter_values.get(parameter_index)?)?;
        let source = usize::try_from(*self.keyform_indices.get(keyform_binding)?).ok()?;
        let constraint = self.constraint_weight(keyform_binding, parameter_values)?;

        let left = interval.left_index();
        let left_weight = ((1.0 - interval.t()) * constraint).clamp(0.0, 1.0);
        if left_weight != 0.0 {
            apply(source.checked_add(left)?, left_weight)?;
        }
        if interval.t() != 0.0 {
            let right_weight = (interval.t() * constraint).clamp(0.0, 1.0);
            if right_weight != 0.0 {
                apply(source.checked_add(left + 1)?, right_weight)?;
            }
        }
        Some(())
    }

    fn parameter_keys(&self, parameter_binding: usize) -> Option<&[f32]> {
        let begin =
            usize::try_from(*self.parameter_keys_begin_indices.get(parameter_binding)?).ok()?;
        let count = usize::try_from(*self.parameter_keys_counts.get(parameter_binding)?).ok()?;
        self.parameter_keys.get(begin..begin.checked_add(count)?)
    }

    fn constraint_weight(&self, keyform_binding: usize, parameter_values: &[f32]) -> Option<f32> {
        let begin =
            usize::try_from(*self.keyform_constraint_begin_indices.get(keyform_binding)?).ok()?;
        let count = usize::try_from(*self.keyform_constraint_counts.get(keyform_binding)?).ok()?;
        let mut result = 1.0;
        for constraint_index in self
            .constraint_indices
            .get(begin..begin.checked_add(count)?)?
        {
            let constraint = usize::try_from(*constraint_index).ok()?;
            let parameter =
                usize::try_from(*self.constraint_parameter_indices.get(constraint)?).ok()?;
            let begin =
                usize::try_from(*self.constraint_value_begin_indices.get(constraint)?).ok()?;
            let count = usize::try_from(*self.constraint_value_counts.get(constraint)?).ok()?;
            let end = begin.checked_add(count)?;
            let keys = self.constraint_keys.get(begin..end)?;
            let weights = self.constraint_weights.get(begin..end)?;
            if keys.len() != weights.len() {
                return None;
            }
            let interval = compute_keyform_axis_interval(keys, *parameter_values.get(parameter)?)?;
            result *= interpolate(weights, interval.left_index(), interval.t())?;
        }
        Some(result)
    }
}

fn expand_binding_parameter_indices(
    parameter_types: &[i32],
    begin_indices: &[i32],
    counts: &[i32],
) -> Option<Vec<usize>> {
    if parameter_types.len() != begin_indices.len() || parameter_types.len() != counts.len() {
        return None;
    }
    let binding_count = counts.iter().try_fold(0usize, |total, &count| {
        total.checked_add(usize::try_from(count).ok()?)
    })?;
    let mut sources = vec![None; binding_count];
    for (parameter_index, ((&parameter_type, &begin), &count)) in parameter_types
        .iter()
        .zip(begin_indices)
        .zip(counts)
        .enumerate()
    {
        match parameter_type {
            0 => continue,
            1 => {}
            _ => return None,
        }
        let count = usize::try_from(count).ok()?;
        if count == 0 {
            continue;
        }
        let begin = usize::try_from(begin).ok()?;
        for source in sources.get_mut(begin..begin.checked_add(count)?)? {
            *source = Some(parameter_index);
        }
    }
    sources.into_iter().collect()
}

fn interpolate(weights: &[f32], left: usize, t: f32) -> Option<f32> {
    let right = if t == 0.0 { left } else { left.checked_add(1)? };
    Some(weights.get(left)? * (1.0 - t) + weights.get(right)? * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_shapes() -> Moc3BlendShapes {
        Moc3BlendShapes {
            binding_parameter_indices: vec![0, 1],
            parameter_keys_begin_indices: vec![0, 3],
            parameter_keys_counts: vec![3, 3],
            parameter_base_key_indices: vec![1, 1],
            parameter_keys: vec![-1.0, 0.0, 1.0, -1.0, 0.0, 1.0],
            keyform_parameter_binding_indices: vec![0, 1],
            keyform_indices: vec![10, 20],
            keyform_counts: vec![3, 3],
            keyform_constraint_begin_indices: vec![0, 0],
            keyform_constraint_counts: vec![0, 0],
            art_mesh_target_indices: vec![2],
            art_mesh_keyform_begin_indices: vec![0],
            art_mesh_keyform_counts: vec![2],
            ..Moc3BlendShapes::default()
        }
    }

    fn active_keyforms(shapes: &Moc3BlendShapes, values: &[f32]) -> Option<Vec<(usize, f32)>> {
        let mut active = Vec::new();
        shapes.for_each_art_mesh_keyform(2, values, |keyform, weight| {
            active.push((keyform, weight));
            Some(())
        })?;
        Some(active)
    }

    #[test]
    fn expands_blend_shape_bindings_using_their_parameter_ranges() {
        let indices =
            expand_binding_parameter_indices(&[0, 1, 0, 1], &[-1, 0, -1, 1], &[0, 1, 0, 2])
                .unwrap();
        assert_eq!(indices, vec![1, 3, 3]);
    }

    #[test]
    fn interpolates_each_parameter_over_its_own_keyform_range() {
        let shapes = test_shapes();
        let active = active_keyforms(&shapes, &[0.5, -0.5]).unwrap();
        assert_eq!(active, vec![(11, 0.5), (12, 0.5), (20, 0.5), (21, 0.5)]);
    }

    #[test]
    fn applies_constraints_to_both_interpolation_weights() {
        let mut shapes = test_shapes();
        shapes.art_mesh_keyform_counts = vec![1];
        shapes.keyform_constraint_counts[0] = 1;
        shapes.constraint_indices = vec![0];
        shapes.constraint_parameter_indices = vec![1];
        shapes.constraint_value_begin_indices = vec![0];
        shapes.constraint_value_counts = vec![2];
        shapes.constraint_keys = vec![0.0, 1.0];
        shapes.constraint_weights = vec![0.0, 1.0];

        let active = active_keyforms(&shapes, &[0.5, 0.25]).unwrap();
        assert_eq!(active, vec![(11, 0.125), (12, 0.125)]);
        assert_eq!(active_keyforms(&shapes, &[0.5, 0.0]), Some(Vec::new()));
    }

    #[test]
    fn default_values_select_the_base_keyforms() {
        let shapes = test_shapes();
        assert_eq!(
            active_keyforms(&shapes, &[0.0, 0.0]),
            Some(vec![(11, 1.0), (21, 1.0)])
        );
    }
}
