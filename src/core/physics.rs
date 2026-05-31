use super::math::{Vector2, direction_to_radian};

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

    if reflect { result } else { result * -1.0 }
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
    if reflect { value * -1.0 } else { value }
}

pub fn physics_output_translation_y(translation: Vector2, reflect: bool) -> f32 {
    let value = translation.y();
    if reflect { value * -1.0 } else { value }
}

pub fn physics_output_angle(translation: Vector2, parent_gravity: Vector2, reflect: bool) -> f32 {
    let value = direction_to_radian(parent_gravity, translation);
    if reflect { value * -1.0 } else { value }
}
