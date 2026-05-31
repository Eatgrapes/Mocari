use rusty_live2d::core::{PhysicsInputAccumulator, PhysicsRange, normalize_physics_parameter};

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
