use std::ops::Range;

use crate::{
    Result,
    core::{Vector2, compute_keyform_axis_interval},
};

use super::{
    Moc3CountInfo, Moc3Header, Moc3SectionOffsets,
    keyform_bindings::Moc3KeyformSlot,
    parse::{invalid_moc3, read_f32_section, read_i32_section, to_usize},
};

const PARAMETER_TYPES: usize = 114;
const PARAMETER_BINDING_BEGINS: usize = 115;
const PARAMETER_BINDING_COUNTS: usize = 116;
const PARAMETER_KEY_BEGINS: usize = 117;
const PARAMETER_KEY_COUNTS: usize = 118;
const PARAMETER_BASE_KEYS: usize = 119;
const KEYFORM_PARAMETERS: usize = 120;
const KEYFORM_BEGINS: usize = 121;
const KEYFORM_COUNTS: usize = 122;
const KEYFORM_CONSTRAINT_BEGINS: usize = 123;
const KEYFORM_CONSTRAINT_COUNTS: usize = 124;
const WARP_TARGETS: [usize; 3] = [125, 126, 127];
const ART_MESH_TARGETS: [usize; 3] = [128, 129, 130];
const CONSTRAINT_INDICES: usize = 131;
const CONSTRAINT_PARAMETERS: usize = 132;
const CONSTRAINT_VALUE_BEGINS: usize = 133;
const CONSTRAINT_VALUE_COUNTS: usize = 134;
const CONSTRAINT_KEYS: usize = 135;
const CONSTRAINT_WEIGHTS: usize = 136;
const ROTATION_TARGETS: [usize; 3] = [146, 147, 148];

#[derive(Debug, Clone, PartialEq)]
struct ParameterBinding {
    parameter: usize,
    keys: Range<usize>,
    base_key: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct KeyformBinding {
    parameter_binding: usize,
    keyforms: Range<usize>,
    constraints: Range<usize>,
}

#[derive(Debug, Clone, PartialEq)]
struct Constraint {
    parameter: Option<usize>,
    values: Range<usize>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub(super) struct Moc3BlendShapes {
    parameters: Vec<ParameterBinding>,
    keyforms: Vec<KeyformBinding>,
    constraints: Vec<Constraint>,
    constraint_indices: Vec<usize>,
    constraint_keys: Vec<f32>,
    constraint_weights: Vec<f32>,
    warp_targets: Vec<Vec<usize>>,
    art_mesh_targets: Vec<Vec<usize>>,
    rotation_targets: Vec<Vec<usize>>,
}

#[derive(Debug, Default, Clone)]
pub(super) struct BlendShapeScratch {
    parameters: Vec<[Moc3KeyformSlot; 2]>,
    constraints: Vec<f32>,
    keyforms: Vec<[Moc3KeyformSlot; 2]>,
}

pub(super) struct EvaluatedBlendShapes<'a> {
    shapes: &'a Moc3BlendShapes,
    keyforms: &'a [[Moc3KeyformSlot; 2]],
}

impl Moc3BlendShapes {
    pub(super) fn parse(
        bytes: &[u8],
        header: Moc3Header,
        offsets: &Moc3SectionOffsets,
        counts: &Moc3CountInfo,
        keys: &[f32],
    ) -> Result<Self> {
        let parameter_count = to_usize(counts.parameters(), "parameter count")?;
        let binding_count = to_usize(
            counts.blend_shape_parameter_bindings(),
            "blend parameter count",
        )?;
        let keyform_count = to_usize(counts.blend_shape_keyform_bindings(), "blend keyform count")?;
        if binding_count == 0 && keyform_count == 0 {
            return Ok(Self::default());
        }
        let constraint_count =
            to_usize(counts.blend_shape_constraints(), "blend constraint count")?;
        let value_count = to_usize(
            counts.blend_shape_constraint_values(),
            "blend constraint value count",
        )?;
        let read = |slot, count| read_i32_section(bytes, offsets, slot, count, header.endianness());
        let types = read(PARAMETER_TYPES, parameter_count)?;
        let begins = read(PARAMETER_BINDING_BEGINS, parameter_count)?;
        let lengths = read(PARAMETER_BINDING_COUNTS, parameter_count)?;
        let bases = read(PARAMETER_BASE_KEYS, binding_count)?;
        let mut owners = vec![None; binding_count];
        for parameter in 0..parameter_count {
            if lengths[parameter] == 0 {
                continue;
            }
            if types[parameter] != 1 {
                return Err(invalid_moc3(
                    "blend binding belongs to a non-blend parameter",
                ));
            }
            let range = checked_range(begins[parameter], lengths[parameter], binding_count)?;
            for owner in &mut owners[range] {
                if owner.replace(parameter).is_some() {
                    return Err(invalid_moc3("overlapping blend parameter binding ranges"));
                }
            }
        }
        let begins = read(PARAMETER_KEY_BEGINS, binding_count)?;
        let lengths = read(PARAMETER_KEY_COUNTS, binding_count)?;
        let parameters = owners
            .into_iter()
            .enumerate()
            .map(|(i, owner)| {
                let parameter =
                    owner.ok_or_else(|| invalid_moc3("unbound blend parameter binding"))?;
                let range = checked_range(begins[i], lengths[i], keys.len())?;
                validate_keys(&keys[range.clone()])?;
                let base_key = checked_index(bases[i], range.len())?;
                Ok(ParameterBinding {
                    parameter,
                    keys: range,
                    base_key,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let constraint_keys = read_f32_section(
            bytes,
            offsets,
            CONSTRAINT_KEYS,
            value_count,
            header.endianness(),
        )?;
        let constraint_weights = read_f32_section(
            bytes,
            offsets,
            CONSTRAINT_WEIGHTS,
            value_count,
            header.endianness(),
        )?;
        if constraint_weights.iter().any(|weight| !weight.is_finite()) {
            return Err(invalid_moc3("non-finite blend constraint weight"));
        }
        let sources = read(CONSTRAINT_PARAMETERS, constraint_count)?;
        let begins = read(CONSTRAINT_VALUE_BEGINS, constraint_count)?;
        let lengths = read(CONSTRAINT_VALUE_COUNTS, constraint_count)?;
        let constraints = (0..constraint_count)
            .map(|i| {
                let parameter = if sources[i] == -1 {
                    None
                } else {
                    Some(checked_index(sources[i], parameter_count)?)
                };
                let values = checked_range(begins[i], lengths[i], value_count)?;
                if !values.is_empty() {
                    validate_keys(&constraint_keys[values.clone()])?;
                }
                Ok(Constraint { parameter, values })
            })
            .collect::<Result<Vec<_>>>()?;
        let constraint_indices = read(
            CONSTRAINT_INDICES,
            to_usize(
                counts.blend_shape_constraint_indices(),
                "blend constraint index count",
            )?,
        )?
        .into_iter()
        .map(|index| checked_index(index, constraint_count))
        .collect::<Result<Vec<_>>>()?;

        let sources = read(KEYFORM_PARAMETERS, keyform_count)?;
        let begins = read(KEYFORM_BEGINS, keyform_count)?;
        let lengths = read(KEYFORM_COUNTS, keyform_count)?;
        let constraint_begins = read(KEYFORM_CONSTRAINT_BEGINS, keyform_count)?;
        let constraint_lengths = read(KEYFORM_CONSTRAINT_COUNTS, keyform_count)?;
        let keyforms = (0..keyform_count)
            .map(|i| {
                let parameter_binding = checked_index(sources[i], binding_count)?;
                let keyforms = checked_range(begins[i], lengths[i], usize::MAX)?;
                if keyforms.len() != parameters[parameter_binding].keys.len() {
                    return Err(invalid_moc3(
                        "blend keyform and parameter key counts differ",
                    ));
                }
                let constraints = checked_range(
                    constraint_begins[i],
                    constraint_lengths[i],
                    constraint_indices.len(),
                )?;
                Ok(KeyformBinding {
                    parameter_binding,
                    keyforms,
                    constraints,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let targets = |slots: [usize; 3],
                       shape_count: u32,
                       target_count: u32,
                       limit: u32|
         -> Result<Vec<Vec<usize>>> {
            let shape_count = to_usize(shape_count, "blend shape count")?;
            if shape_count == 0 {
                return Ok(Vec::new());
            }
            let target_count = to_usize(target_count, "blend target count")?;
            if target_count > bytes.len() / 4 {
                return Err(invalid_moc3("blend target count exceeds file size"));
            }
            let indices = read(slots[0], shape_count)?;
            let begins = read(slots[1], shape_count)?;
            let lengths = read(slots[2], shape_count)?;
            let mut targets = vec![Vec::new(); target_count];
            for i in 0..shape_count {
                let target = checked_index(indices[i], targets.len())?;
                let range = checked_range(begins[i], lengths[i], keyforms.len())?;
                for binding in range {
                    if keyforms[binding].keyforms.end > limit as usize {
                        return Err(invalid_moc3(
                            "blend keyforms are outside their target table",
                        ));
                    }
                    targets[target].push(binding);
                }
            }
            Ok(targets)
        };
        let warp_targets = targets(
            WARP_TARGETS,
            counts.blend_shapes_warp_deformers(),
            counts.warp_deformers(),
            counts.warp_deformer_keyforms(),
        )?;
        let art_mesh_targets = targets(
            ART_MESH_TARGETS,
            counts.blend_shapes_art_meshes(),
            counts.art_meshes(),
            counts.art_mesh_keyforms(),
        )?;
        let rotation_targets = targets(
            ROTATION_TARGETS,
            counts.blend_shapes_rotation_deformers(),
            counts.rotation_deformers(),
            counts.rotation_deformer_keyforms(),
        )?;
        Ok(Self {
            parameters,
            keyforms,
            constraints,
            constraint_indices,
            constraint_keys,
            constraint_weights,
            warp_targets,
            art_mesh_targets,
            rotation_targets,
        })
    }

    pub(super) fn evaluate<'a>(
        &'a self,
        keys: &[f32],
        values: &[f32],
        scratch: &'a mut BlendShapeScratch,
    ) -> Option<EvaluatedBlendShapes<'a>> {
        scratch.parameters.clear();
        for binding in &self.parameters {
            let interval = compute_keyform_axis_interval(
                &keys[binding.keys.clone()],
                *values.get(binding.parameter)?,
            )?;
            let left = interval.left_index();
            let t = interval.t();
            scratch.parameters.push([
                Moc3KeyformSlot {
                    local_index: left,
                    weight: if left == binding.base_key {
                        0.0
                    } else {
                        1.0 - t
                    },
                },
                Moc3KeyformSlot {
                    local_index: left + usize::from(t != 0.0),
                    weight: if left + 1 == binding.base_key { 0.0 } else { t },
                },
            ]);
        }
        scratch.constraints.clear();
        for constraint in &self.constraints {
            let weight = match constraint.parameter {
                Some(parameter) if !constraint.values.is_empty() => {
                    let keys = &self.constraint_keys[constraint.values.clone()];
                    let weights = &self.constraint_weights[constraint.values.clone()];
                    let interval = compute_keyform_axis_interval(keys, *values.get(parameter)?)?;
                    let left = interval.left_index();
                    let t = interval.t();
                    weights[left] * (1.0 - t) + weights[left + usize::from(t != 0.0)] * t
                }
                _ => 1.0,
            };
            scratch.constraints.push(weight);
        }
        scratch.keyforms.clear();
        for binding in &self.keyforms {
            let constraint = self.constraint_indices[binding.constraints.clone()]
                .iter()
                .fold(1.0_f32, |weight, &index| {
                    weight.min(scratch.constraints[index])
                });
            scratch
                .keyforms
                .push(
                    scratch.parameters[binding.parameter_binding].map(|slot| Moc3KeyformSlot {
                        local_index: binding.keyforms.start + slot.local_index,
                        weight: slot.weight * constraint,
                    }),
                );
        }
        Some(EvaluatedBlendShapes {
            shapes: self,
            keyforms: &scratch.keyforms,
        })
    }
}

impl EvaluatedBlendShapes<'_> {
    pub(super) fn for_each_warp_keyform(
        &self,
        target: usize,
        apply: impl FnMut(usize, f32) -> Option<()>,
    ) -> Option<()> {
        self.for_each(&self.shapes.warp_targets, target, apply)
    }

    pub(super) fn for_each_art_mesh_keyform(
        &self,
        target: usize,
        apply: impl FnMut(usize, f32) -> Option<()>,
    ) -> Option<()> {
        self.for_each(&self.shapes.art_mesh_targets, target, apply)
    }

    pub(super) fn for_each_rotation_keyform(
        &self,
        target: usize,
        apply: impl FnMut(usize, f32) -> Option<()>,
    ) -> Option<()> {
        self.for_each(&self.shapes.rotation_targets, target, apply)
    }

    fn for_each(
        &self,
        targets: &[Vec<usize>],
        target: usize,
        mut apply: impl FnMut(usize, f32) -> Option<()>,
    ) -> Option<()> {
        for &binding in targets.get(target).into_iter().flatten() {
            for slot in &self.keyforms[binding] {
                if slot.weight != 0.0 {
                    apply(slot.local_index, slot.weight)?;
                }
            }
        }
        Some(())
    }
}

fn checked_range(begin: i32, count: i32, limit: usize) -> Result<Range<usize>> {
    if count == 0 {
        return Ok(0..0);
    }
    let begin = usize::try_from(begin).map_err(|_| invalid_moc3("negative blend range begin"))?;
    let count = usize::try_from(count).map_err(|_| invalid_moc3("negative blend range count"))?;
    let end = begin
        .checked_add(count)
        .filter(|&end| end <= limit)
        .ok_or_else(|| invalid_moc3("blend range is outside its table"))?;
    Ok(begin..end)
}

pub(super) fn add_weighted_positions(
    positions: &mut [Vector2],
    source: &[f32],
    weight: f32,
) -> Option<()> {
    if source.len() != positions.len().checked_mul(2)? {
        return None;
    }
    for (position, xy) in positions.iter_mut().zip(source.as_chunks::<2>().0) {
        *position = Vector2::new(position.x() + xy[0] * weight, position.y() + xy[1] * weight);
    }
    Some(())
}

fn checked_index(index: i32, limit: usize) -> Result<usize> {
    usize::try_from(index)
        .ok()
        .filter(|&index| index < limit)
        .ok_or_else(|| invalid_moc3("blend index is outside its table"))
}

fn validate_keys(keys: &[f32]) -> Result<()> {
    if keys.is_empty()
        || keys.iter().any(|key| !key.is_finite())
        || keys.windows(2).any(|pair| pair[0] > pair[1])
    {
        return Err(invalid_moc3(
            "blend keys must be finite, ordered and nonempty",
        ));
    }
    Ok(())
}
