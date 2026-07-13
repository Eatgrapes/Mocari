use std::collections::HashMap;

use crate::json::{Physics3, PhysicsValueKind};

use super::math::{Vector2, degrees_to_radian, direction_to_radian, radian_to_direction};

const MAXIMUM_WEIGHT: f32 = 100.0;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct PhysicsRange {
    minimum: f32,
    maximum: f32,
    default: f32,
}

impl PhysicsRange {
    pub fn new(minimum: f32, maximum: f32, default: f32) -> Self {
        Self {
            minimum,
            maximum,
            default,
        }
    }

    pub fn minimum(&self) -> f32 {
        self.minimum
    }

    pub fn maximum(&self) -> f32 {
        self.maximum
    }

    pub fn default(&self) -> f32 {
        self.default
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct PhysicsInputAccumulator {
    translation_x: f32,
    translation_y: f32,
    angle: f32,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct PhysicsParticle {
    position: Vector2,
    last_position: Vector2,
    velocity: Vector2,
    force: Vector2,
    last_gravity: Vector2,
    mobility: f32,
    delay: f32,
    acceleration: f32,
    radius: f32,
}

impl PhysicsParticle {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        position: Vector2,
        last_position: Vector2,
        velocity: Vector2,
        force: Vector2,
        last_gravity: Vector2,
        mobility: f32,
        delay: f32,
        acceleration: f32,
        radius: f32,
    ) -> Self {
        Self {
            position,
            last_position,
            velocity,
            force,
            last_gravity,
            mobility,
            delay,
            acceleration,
            radius,
        }
    }

    pub fn position(&self) -> Vector2 {
        self.position
    }

    pub fn last_position(&self) -> Vector2 {
        self.last_position
    }

    pub fn velocity(&self) -> Vector2 {
        self.velocity
    }

    pub fn force(&self) -> Vector2 {
        self.force
    }

    pub fn last_gravity(&self) -> Vector2 {
        self.last_gravity
    }
}

impl PhysicsInputAccumulator {
    pub fn add_translation_x(
        &mut self,
        value: f32,
        parameter: PhysicsRange,
        normalization: PhysicsRange,
        reflect: bool,
        weight_percent: f32,
    ) {
        self.translation_x +=
            weighted_normalized_value(value, parameter, normalization, reflect, weight_percent);
    }

    pub fn add_translation_y(
        &mut self,
        value: f32,
        parameter: PhysicsRange,
        normalization: PhysicsRange,
        reflect: bool,
        weight_percent: f32,
    ) {
        self.translation_y +=
            weighted_normalized_value(value, parameter, normalization, reflect, weight_percent);
    }

    pub fn add_angle(
        &mut self,
        value: f32,
        parameter: PhysicsRange,
        normalization: PhysicsRange,
        reflect: bool,
        weight_percent: f32,
    ) {
        self.angle +=
            weighted_normalized_value(value, parameter, normalization, reflect, weight_percent);
    }

    pub fn translation_x(&self) -> f32 {
        self.translation_x
    }

    pub fn translation_y(&self) -> f32 {
        self.translation_y
    }

    pub fn angle(&self) -> f32 {
        self.angle
    }
}

pub fn normalize_physics_parameter(
    value: f32,
    parameter: PhysicsRange,
    normalized: PhysicsRange,
    reflect: bool,
) -> f32 {
    let maximum = parameter.maximum.max(parameter.minimum);
    let minimum = parameter.maximum.min(parameter.minimum);
    let value = value.clamp(minimum, maximum);
    let normalized_minimum = normalized.minimum.min(normalized.maximum);
    let normalized_maximum = normalized.minimum.max(normalized.maximum);
    let normalized_middle = normalized.default;
    let middle = minimum + ((maximum - minimum).abs() / 2.0);
    let parameter_value = value - middle;

    let result = match parameter_value.total_cmp(&0.0) {
        std::cmp::Ordering::Greater => {
            let normalized_length = normalized_maximum - normalized_middle;
            let parameter_length = maximum - middle;
            if parameter_length == 0.0 {
                0.0
            } else {
                parameter_value * (normalized_length / parameter_length) + normalized_middle
            }
        }
        std::cmp::Ordering::Less => {
            let normalized_length = normalized_minimum - normalized_middle;
            let parameter_length = minimum - middle;
            if parameter_length == 0.0 {
                0.0
            } else {
                parameter_value * (normalized_length / parameter_length) + normalized_middle
            }
        }
        std::cmp::Ordering::Equal => normalized_middle,
    };

    if reflect { result } else { -result }
}

fn weighted_normalized_value(
    value: f32,
    parameter: PhysicsRange,
    normalization: PhysicsRange,
    reflect: bool,
    weight_percent: f32,
) -> f32 {
    normalize_physics_parameter(value, parameter, normalization, reflect)
        * (weight_percent / MAXIMUM_WEIGHT)
}

pub fn physics_output_translation_x(translation: Vector2, reflect: bool) -> f32 {
    let value = translation.x();
    if reflect { -value } else { value }
}

pub fn physics_output_translation_y(translation: Vector2, reflect: bool) -> f32 {
    let value = translation.y();
    if reflect { -value } else { value }
}

pub fn parent_gravity_for_physics_output(
    particles: &[Vector2],
    particle_index: usize,
    parent_gravity: Vector2,
) -> Option<Vector2> {
    if particle_index >= 2 {
        let current = particles.get(particle_index - 1)?;
        let previous = particles.get(particle_index - 2)?;
        return Some(Vector2::new(
            current.x() - previous.x(),
            current.y() - previous.y(),
        ));
    }

    Some(Vector2::new(-parent_gravity.x(), -parent_gravity.y()))
}

pub fn physics_output_angle_with_parent_gravity(
    translation: Vector2,
    parent_gravity: Vector2,
    reflect: bool,
) -> f32 {
    let value = direction_to_radian(parent_gravity, translation);
    if reflect { -value } else { value }
}

pub fn update_physics_particles(
    strand: &mut [PhysicsParticle],
    total_translation: Vector2,
    total_angle: f32,
    wind_direction: Vector2,
    threshold_value: f32,
    delta_time_seconds: f32,
    air_resistance: f32,
) {
    let Some((first, rest)) = strand.split_first_mut() else {
        return;
    };

    first.position = total_translation;
    let current_gravity = normalize(radian_to_direction(degrees_to_radian(total_angle)));
    let mut previous_position = first.position;

    for particle in rest {
        particle.force = add(mul(current_gravity, particle.acceleration), wind_direction);
        particle.last_position = particle.position;

        let delay = particle.delay * delta_time_seconds * 30.0;
        let mut direction = sub(particle.position, previous_position);
        let radian = direction_to_radian(particle.last_gravity, current_gravity) / air_resistance;

        let direction_x = radian.cos() * direction.x() - direction.y() * radian.sin();
        let direction_y = radian.sin() * direction_x + direction.y() * radian.cos();
        direction = Vector2::new(direction_x, direction_y);

        particle.position = add(previous_position, direction);
        let velocity = mul(particle.velocity, delay);
        let force = mul(particle.force, delay * delay);
        particle.position = add(add(particle.position, velocity), force);

        let new_direction = normalize(sub(particle.position, previous_position));
        particle.position = add(previous_position, mul(new_direction, particle.radius));

        if particle.position.x().abs() < threshold_value {
            particle.position = Vector2::new(0.0, particle.position.y());
        }

        if delay != 0.0 {
            particle.velocity = mul(
                div(sub(particle.position, particle.last_position), delay),
                particle.mobility,
            );
        }

        particle.force = Vector2::new(0.0, 0.0);
        particle.last_gravity = current_gravity;
        previous_position = particle.position;
    }
}

pub fn stabilize_physics_particles(
    strand: &mut [PhysicsParticle],
    total_translation: Vector2,
    total_angle: f32,
    wind_direction: Vector2,
    threshold_value: f32,
) {
    let Some((first, rest)) = strand.split_first_mut() else {
        return;
    };

    first.position = total_translation;
    let current_gravity = normalize(radian_to_direction(degrees_to_radian(total_angle)));
    let mut previous_position = first.position;

    for particle in rest {
        particle.force = add(mul(current_gravity, particle.acceleration), wind_direction);
        particle.last_position = particle.position;
        particle.velocity = Vector2::new(0.0, 0.0);

        let force = mul(normalize(particle.force), particle.radius);
        particle.position = add(previous_position, force);

        if particle.position.x().abs() < threshold_value {
            particle.position = Vector2::new(0.0, particle.position.y());
        }

        particle.force = Vector2::new(0.0, 0.0);
        particle.last_gravity = current_gravity;
        previous_position = particle.position;
    }
}

fn add(a: Vector2, b: Vector2) -> Vector2 {
    Vector2::new(a.x() + b.x(), a.y() + b.y())
}

fn sub(a: Vector2, b: Vector2) -> Vector2 {
    Vector2::new(a.x() - b.x(), a.y() - b.y())
}

fn mul(value: Vector2, factor: f32) -> Vector2 {
    Vector2::new(value.x() * factor, value.y() * factor)
}

fn div(value: Vector2, factor: f32) -> Vector2 {
    Vector2::new(value.x() / factor, value.y() / factor)
}

fn normalize(value: Vector2) -> Vector2 {
    let length = (value.x() * value.x() + value.y() * value.y()).sqrt();
    if length == 0.0 {
        value
    } else {
        div(value, length)
    }
}

const AIR_RESISTANCE: f32 = 5.0;
const MAXIMUM_DELTA_TIME: f32 = 5.0;
const MOVEMENT_THRESHOLD: f32 = 0.001;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct PhysicsOptions {
    gravity: Vector2,
    wind: Vector2,
}

impl PhysicsOptions {
    pub fn new(gravity: Vector2, wind: Vector2) -> Self {
        Self { gravity, wind }
    }

    pub fn gravity(&self) -> Vector2 {
        self.gravity
    }

    pub fn wind(&self) -> Vector2 {
        self.wind
    }
}

impl Default for PhysicsOptions {
    fn default() -> Self {
        Self::new(Vector2::new(0.0, -1.0), Vector2::new(0.0, 0.0))
    }
}

#[derive(Debug, Clone)]
struct RuntimePhysicsInput {
    parameter_index: Option<usize>,
    kind: PhysicsValueKind,
    reflect: bool,
    weight: f32,
}

#[derive(Debug, Clone)]
struct RuntimePhysicsOutput {
    parameter_index: Option<usize>,
    particle_index: usize,
    scale: f32,
    weight: f32,
    kind: PhysicsValueKind,
    reflect: bool,
}

#[derive(Debug, Clone)]
struct RuntimePhysicsSetting {
    inputs: Vec<RuntimePhysicsInput>,
    outputs: Vec<RuntimePhysicsOutput>,
    particles: Vec<PhysicsParticle>,
    position_normalization: PhysicsRange,
    angle_normalization: PhysicsRange,
    current_outputs: Vec<f32>,
    previous_outputs: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct PhysicsRuntime {
    settings: Vec<RuntimePhysicsSetting>,
    options: PhysicsOptions,
    current_remain_time: f32,
    parameter_cache: Vec<f32>,
    parameter_input_cache: Vec<f32>,
    fps: f32,
}

impl PhysicsRuntime {
    pub fn new(physics: &Physics3, parameter_ids: &[String]) -> Self {
        let parameter_indices = parameter_ids
            .iter()
            .enumerate()
            .map(|(index, id)| (id.as_str(), index))
            .collect::<HashMap<_, _>>();
        let settings = physics
            .settings()
            .iter()
            .map(|setting| {
                let inputs = setting
                    .inputs()
                    .iter()
                    .map(|input| RuntimePhysicsInput {
                        parameter_index: parameter_indices.get(input.source().id()).copied(),
                        kind: input.kind(),
                        reflect: input.reflect(),
                        weight: input.weight(),
                    })
                    .collect();
                let outputs = setting
                    .outputs()
                    .iter()
                    .map(|output| RuntimePhysicsOutput {
                        parameter_index: parameter_indices.get(output.destination().id()).copied(),
                        particle_index: output.vertex_index() as usize,
                        scale: output.scale(),
                        weight: output.weight(),
                        kind: output.kind(),
                        reflect: output.reflect(),
                    })
                    .collect::<Vec<_>>();
                let particles = particles_from_vertices(setting.vertices());

                RuntimePhysicsSetting {
                    inputs,
                    current_outputs: vec![0.0; outputs.len()],
                    previous_outputs: vec![0.0; outputs.len()],
                    outputs,
                    particles,
                    position_normalization: physics_range(setting.normalization().position()),
                    angle_normalization: physics_range(setting.normalization().angle()),
                }
            })
            .collect();

        Self {
            settings,
            options: PhysicsOptions::default(),
            current_remain_time: 0.0,
            parameter_cache: Vec::new(),
            parameter_input_cache: Vec::new(),
            fps: physics.meta().fps(),
        }
    }

    pub fn options(&self) -> PhysicsOptions {
        self.options
    }

    pub fn set_options(&mut self, options: PhysicsOptions) {
        self.options = options;
    }

    pub fn reset(&mut self) {
        self.options = PhysicsOptions::default();
        self.current_remain_time = 0.0;
        self.parameter_cache.clear();
        self.parameter_input_cache.clear();
        for setting in &mut self.settings {
            setting.particles = reset_particles(&setting.particles);
            setting.current_outputs.fill(0.0);
            setting.previous_outputs.fill(0.0);
        }
    }

    pub fn evaluate(
        &mut self,
        parameter_values: &mut [f32],
        parameter_minimum_values: &[f32],
        parameter_maximum_values: &[f32],
        parameter_default_values: &[f32],
        delta_time_seconds: f32,
    ) {
        if delta_time_seconds <= 0.0 || !delta_time_seconds.is_finite() {
            return;
        }

        let parameter_count = parameter_values.len();
        if parameter_minimum_values.len() != parameter_count
            || parameter_maximum_values.len() != parameter_count
            || parameter_default_values.len() != parameter_count
        {
            return;
        }

        self.current_remain_time += delta_time_seconds;
        if self.current_remain_time > MAXIMUM_DELTA_TIME {
            self.current_remain_time = 0.0;
        }

        if self.parameter_cache.len() != parameter_count {
            self.parameter_cache.resize(parameter_count, 0.0);
        }
        if self.parameter_input_cache.len() != parameter_count {
            self.parameter_input_cache.clear();
            self.parameter_input_cache
                .extend_from_slice(parameter_values);
        }

        let physics_delta_time = if self.fps > 0.0 && self.fps.is_finite() {
            1.0 / self.fps
        } else {
            delta_time_seconds
        };
        if !physics_delta_time.is_finite() || physics_delta_time <= 0.0 {
            return;
        }

        while self.current_remain_time >= physics_delta_time {
            let input_weight = physics_delta_time / self.current_remain_time;
            for (index, &parameter_value) in parameter_values.iter().enumerate() {
                self.parameter_cache[index] = self.parameter_input_cache[index]
                    * (1.0 - input_weight)
                    + parameter_value * input_weight;
                self.parameter_input_cache[index] = self.parameter_cache[index];
            }

            for setting in &mut self.settings {
                setting
                    .previous_outputs
                    .copy_from_slice(&setting.current_outputs);
                evaluate_setting(
                    setting,
                    &mut self.parameter_cache,
                    parameter_minimum_values,
                    parameter_maximum_values,
                    parameter_default_values,
                    self.options,
                    physics_delta_time,
                );
            }

            self.current_remain_time -= physics_delta_time;
        }

        let alpha = self.current_remain_time / physics_delta_time;
        for setting in &self.settings {
            for (output, (&previous, &current)) in setting.outputs.iter().zip(
                setting
                    .previous_outputs
                    .iter()
                    .zip(&setting.current_outputs),
            ) {
                let Some(parameter_index) = output.parameter_index else {
                    continue;
                };
                let Some(value) = parameter_values.get_mut(parameter_index) else {
                    continue;
                };
                let Some((&minimum, &maximum)) = parameter_minimum_values
                    .get(parameter_index)
                    .zip(parameter_maximum_values.get(parameter_index))
                else {
                    continue;
                };
                update_output_parameter(
                    value,
                    minimum,
                    maximum,
                    previous * (1.0 - alpha) + current * alpha,
                    output,
                );
            }
        }
    }

    pub fn stabilize(
        &mut self,
        parameter_values: &mut [f32],
        parameter_minimum_values: &[f32],
        parameter_maximum_values: &[f32],
        parameter_default_values: &[f32],
    ) {
        let parameter_count = parameter_values.len();
        if parameter_minimum_values.len() != parameter_count
            || parameter_maximum_values.len() != parameter_count
            || parameter_default_values.len() != parameter_count
        {
            return;
        }

        for setting in &mut self.settings {
            stabilize_setting(
                setting,
                parameter_values,
                parameter_minimum_values,
                parameter_maximum_values,
                parameter_default_values,
                self.options,
            );
        }
    }
}

fn evaluate_setting(
    setting: &mut RuntimePhysicsSetting,
    parameter_cache: &mut [f32],
    parameter_minimum_values: &[f32],
    parameter_maximum_values: &[f32],
    parameter_default_values: &[f32],
    options: PhysicsOptions,
    delta_time_seconds: f32,
) {
    let input = accumulate_inputs(
        setting,
        parameter_cache,
        parameter_minimum_values,
        parameter_maximum_values,
        parameter_default_values,
    );

    let radian = degrees_to_radian(-input.angle());
    let mut translation = Vector2::new(input.translation_x(), input.translation_y());
    let translation_x = translation.x() * radian.cos() - translation.y() * radian.sin();
    translation = Vector2::new(
        translation_x,
        translation_x * radian.sin() + translation.y() * radian.cos(),
    );
    update_physics_particles(
        &mut setting.particles,
        translation,
        input.angle(),
        options.wind(),
        MOVEMENT_THRESHOLD * setting.position_normalization.maximum(),
        delta_time_seconds,
        AIR_RESISTANCE,
    );

    for (index, output) in setting.outputs.iter().enumerate() {
        let Some((current, previous)) = setting.particles.get(output.particle_index).zip(
            output
                .particle_index
                .checked_sub(1)
                .and_then(|index| setting.particles.get(index)),
        ) else {
            continue;
        };
        let translation = sub(current.position(), previous.position());
        let value = match output.kind {
            PhysicsValueKind::X => physics_output_translation_x(translation, output.reflect),
            PhysicsValueKind::Y => physics_output_translation_y(translation, output.reflect),
            PhysicsValueKind::Angle => {
                let Some(parent_gravity) = parent_gravity_for_particles(
                    &setting.particles,
                    output.particle_index,
                    options.gravity(),
                ) else {
                    continue;
                };
                physics_output_angle_with_parent_gravity(
                    translation,
                    parent_gravity,
                    output.reflect,
                )
            }
        };
        setting.current_outputs[index] = value;

        let Some(parameter_index) = output.parameter_index else {
            continue;
        };
        let Some((value, (&minimum, &maximum))) = parameter_cache.get_mut(parameter_index).zip(
            parameter_minimum_values
                .get(parameter_index)
                .zip(parameter_maximum_values.get(parameter_index)),
        ) else {
            continue;
        };
        update_output_parameter(value, minimum, maximum, *value, output);
    }
}

fn stabilize_setting(
    setting: &mut RuntimePhysicsSetting,
    parameter_values: &mut [f32],
    parameter_minimum_values: &[f32],
    parameter_maximum_values: &[f32],
    parameter_default_values: &[f32],
    options: PhysicsOptions,
) {
    let input = accumulate_inputs(
        setting,
        parameter_values,
        parameter_minimum_values,
        parameter_maximum_values,
        parameter_default_values,
    );
    let radian = degrees_to_radian(-input.angle());
    let mut translation = Vector2::new(input.translation_x(), input.translation_y());
    let translation_x = translation.x() * radian.cos() - translation.y() * radian.sin();
    translation = Vector2::new(
        translation_x,
        translation_x * radian.sin() + translation.y() * radian.cos(),
    );
    stabilize_physics_particles(
        &mut setting.particles,
        translation,
        input.angle(),
        options.wind(),
        MOVEMENT_THRESHOLD * setting.position_normalization.maximum(),
    );

    for output in &setting.outputs {
        let Some((current, previous)) = setting.particles.get(output.particle_index).zip(
            output
                .particle_index
                .checked_sub(1)
                .and_then(|index| setting.particles.get(index)),
        ) else {
            continue;
        };
        let translation = sub(current.position(), previous.position());
        let value = match output.kind {
            PhysicsValueKind::X => physics_output_translation_x(translation, output.reflect),
            PhysicsValueKind::Y => physics_output_translation_y(translation, output.reflect),
            PhysicsValueKind::Angle => {
                let Some(parent_gravity) = parent_gravity_for_particles(
                    &setting.particles,
                    output.particle_index,
                    options.gravity(),
                ) else {
                    continue;
                };
                physics_output_angle_with_parent_gravity(
                    translation,
                    parent_gravity,
                    output.reflect,
                )
            }
        };
        let Some(parameter_index) = output.parameter_index else {
            continue;
        };
        let Some((parameter_value, (&minimum, &maximum))) =
            parameter_values.get_mut(parameter_index).zip(
                parameter_minimum_values
                    .get(parameter_index)
                    .zip(parameter_maximum_values.get(parameter_index)),
            )
        else {
            continue;
        };
        update_output_parameter(parameter_value, minimum, maximum, value, output);
    }
}

fn accumulate_inputs(
    setting: &RuntimePhysicsSetting,
    parameter_values: &[f32],
    parameter_minimum_values: &[f32],
    parameter_maximum_values: &[f32],
    parameter_default_values: &[f32],
) -> PhysicsInputAccumulator {
    let mut input = PhysicsInputAccumulator::default();
    for source in &setting.inputs {
        let Some(parameter_index) = source.parameter_index else {
            continue;
        };
        let Some(((&value, &minimum), (&maximum, &default))) = parameter_values
            .get(parameter_index)
            .zip(parameter_minimum_values.get(parameter_index))
            .zip(
                parameter_maximum_values
                    .get(parameter_index)
                    .zip(parameter_default_values.get(parameter_index)),
            )
        else {
            continue;
        };
        let parameter = PhysicsRange::new(minimum, maximum, default);
        match source.kind {
            PhysicsValueKind::X => input.add_translation_x(
                value,
                parameter,
                setting.position_normalization,
                source.reflect,
                source.weight,
            ),
            PhysicsValueKind::Y => input.add_translation_y(
                value,
                parameter,
                setting.position_normalization,
                source.reflect,
                source.weight,
            ),
            PhysicsValueKind::Angle => input.add_angle(
                value,
                parameter,
                setting.angle_normalization,
                source.reflect,
                source.weight,
            ),
        }
    }
    input
}

fn update_output_parameter(
    parameter_value: &mut f32,
    minimum: f32,
    maximum: f32,
    translation: f32,
    output: &RuntimePhysicsOutput,
) {
    let value = (translation * output.scale).clamp(minimum, maximum);
    let weight = output.weight / MAXIMUM_WEIGHT;
    *parameter_value = if weight >= 1.0 {
        value
    } else {
        *parameter_value * (1.0 - weight) + value * weight
    };
}

fn physics_range(value: &crate::json::PhysicsNormalizationValue) -> PhysicsRange {
    PhysicsRange::new(value.minimum(), value.maximum(), value.default())
}

fn particles_from_vertices(vertices: &[crate::json::PhysicsVertex]) -> Vec<PhysicsParticle> {
    let mut position = Vector2::new(0.0, 0.0);
    vertices
        .iter()
        .enumerate()
        .map(|(index, vertex)| {
            if index != 0 {
                position = add(position, Vector2::new(0.0, vertex.radius()));
            }
            PhysicsParticle::new(
                position,
                position,
                Vector2::new(0.0, 0.0),
                Vector2::new(0.0, 0.0),
                Vector2::new(0.0, 1.0),
                vertex.mobility(),
                vertex.delay(),
                vertex.acceleration(),
                vertex.radius(),
            )
        })
        .collect()
}

fn reset_particles(particles: &[PhysicsParticle]) -> Vec<PhysicsParticle> {
    let mut position = Vector2::new(0.0, 0.0);
    particles
        .iter()
        .enumerate()
        .map(|(index, particle)| {
            if index != 0 {
                position = add(position, Vector2::new(0.0, particle.radius));
            }
            PhysicsParticle::new(
                position,
                position,
                Vector2::new(0.0, 0.0),
                Vector2::new(0.0, 0.0),
                Vector2::new(0.0, 1.0),
                particle.mobility,
                particle.delay,
                particle.acceleration,
                particle.radius,
            )
        })
        .collect()
}

fn parent_gravity_for_particles(
    particles: &[PhysicsParticle],
    particle_index: usize,
    parent_gravity: Vector2,
) -> Option<Vector2> {
    if particle_index >= 2 {
        let current = particles.get(particle_index - 1)?;
        let previous = particles.get(particle_index - 2)?;
        Some(sub(current.position(), previous.position()))
    } else {
        Some(Vector2::new(-parent_gravity.x(), -parent_gravity.y()))
    }
}
