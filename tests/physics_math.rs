use rusty_live2d::core::{
    PhysicsInputAccumulator, PhysicsRange, Vector2, direction_to_radian,
    normalize_physics_parameter, physics_output_angle, physics_output_translation_x,
    physics_output_translation_y, radian_to_direction,
};

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.00001,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn physics_parameter_normalization_uses_range_midpoint() {
    let parameter = PhysicsRange::new(0.0, 10.0, 2.0);
    let normalized = PhysicsRange::new(0.0, 1.0, 0.5);

    assert_close(
        normalize_physics_parameter(7.5, parameter, normalized, true),
        0.75,
    );
    assert_close(
        normalize_physics_parameter(7.5, parameter, normalized, false),
        -0.75,
    );
}

#[test]
fn physics_input_accumulator_applies_weighted_channels() {
    let parameter = PhysicsRange::new(-30.0, 30.0, 0.0);
    let position = PhysicsRange::new(-10.0, 10.0, 0.0);
    let angle = PhysicsRange::new(-30.0, 30.0, 0.0);
    let mut input = PhysicsInputAccumulator::default();

    input.add_translation_x(30.0, parameter, position, true, 50.0);
    input.add_translation_y(-30.0, parameter, position, false, 25.0);
    input.add_angle(15.0, parameter, angle, true, 100.0);

    assert_close(input.translation_x(), 5.0);
    assert_close(input.translation_y(), 2.5);
    assert_close(input.angle(), 15.0);
}

#[test]
fn physics_output_translation_reflects_axes() {
    let translation = Vector2::new(3.0, -4.0);

    assert_close(physics_output_translation_x(translation, false), 3.0);
    assert_close(physics_output_translation_x(translation, true), -3.0);
    assert_close(physics_output_translation_y(translation, false), -4.0);
    assert_close(physics_output_translation_y(translation, true), 4.0);
}

#[test]
fn physics_output_angle_uses_direction_delta() {
    let parent_gravity = Vector2::new(0.0, 1.0);
    let translation = Vector2::new(1.0, 0.0);

    assert_close(
        direction_to_radian(parent_gravity, translation),
        -std::f32::consts::FRAC_PI_2,
    );
    assert_eq!(radian_to_direction(0.0), parent_gravity);
    assert_close(
        physics_output_angle(translation, parent_gravity, false),
        -std::f32::consts::FRAC_PI_2,
    );
    assert_close(
        physics_output_angle(translation, parent_gravity, true),
        std::f32::consts::FRAC_PI_2,
    );
}
