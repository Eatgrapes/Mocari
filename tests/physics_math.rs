use rusty_live2d::core::{
    PhysicsInputAccumulator, PhysicsParticle, PhysicsRange, Vector2, direction_to_radian,
    normalize_physics_parameter, parent_gravity_for_physics_output,
    physics_output_angle_with_parent_gravity, physics_output_translation_x,
    physics_output_translation_y, radian_to_direction, stabilize_physics_particles,
    update_physics_particles,
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
        physics_output_angle_with_parent_gravity(translation, parent_gravity, false),
        -std::f32::consts::FRAC_PI_2,
    );
    assert_close(
        physics_output_angle_with_parent_gravity(translation, parent_gravity, true),
        std::f32::consts::FRAC_PI_2,
    );
}

#[test]
fn physics_output_parent_gravity_matches_particle_index_branch() {
    let particles = [
        Vector2::new(1.0, 2.0),
        Vector2::new(4.0, 6.0),
        Vector2::new(10.0, 6.0),
    ];
    let parent_gravity = Vector2::new(0.0, -1.0);

    assert_eq!(
        parent_gravity_for_physics_output(&particles, 0, parent_gravity),
        Some(Vector2::new(0.0, 1.0))
    );
    assert_eq!(
        parent_gravity_for_physics_output(&particles, 1, parent_gravity),
        Some(Vector2::new(0.0, 1.0))
    );
    assert_eq!(
        parent_gravity_for_physics_output(&particles, 2, parent_gravity),
        Some(Vector2::new(3.0, 4.0))
    );
    assert_eq!(
        parent_gravity_for_physics_output(&particles, 3, parent_gravity),
        Some(Vector2::new(6.0, 0.0))
    );
    assert_eq!(
        parent_gravity_for_physics_output(&particles, 4, parent_gravity),
        None
    );
}

#[test]
fn physics_particles_update_positions_and_velocity() {
    let mut particles = [
        PhysicsParticle::new(
            Vector2::new(0.0, 0.0),
            Vector2::new(0.0, 0.0),
            Vector2::new(0.0, 0.0),
            Vector2::new(0.0, 0.0),
            Vector2::new(0.0, 1.0),
            1.0,
            1.0,
            0.0,
            0.0,
        ),
        PhysicsParticle::new(
            Vector2::new(0.0, 1.0),
            Vector2::new(0.0, 1.0),
            Vector2::new(1.0, 0.0),
            Vector2::new(0.0, 0.0),
            Vector2::new(0.0, 1.0),
            1.0,
            1.0,
            0.0,
            1.0,
        ),
    ];

    update_physics_particles(
        &mut particles,
        Vector2::new(0.0, 0.0),
        0.0,
        Vector2::new(0.0, 0.0),
        0.001,
        1.0 / 30.0,
        5.0,
    );

    assert_close(particles[1].position().x(), std::f32::consts::FRAC_1_SQRT_2);
    assert_close(particles[1].position().y(), std::f32::consts::FRAC_1_SQRT_2);
    assert_close(particles[1].velocity().x(), std::f32::consts::FRAC_1_SQRT_2);
    assert_close(
        particles[1].velocity().y(),
        std::f32::consts::FRAC_1_SQRT_2 - 1.0,
    );
}

#[test]
fn physics_particles_stabilize_along_force_direction() {
    let mut particles = [
        PhysicsParticle::new(
            Vector2::new(0.0, 0.0),
            Vector2::new(0.0, 0.0),
            Vector2::new(3.0, 4.0),
            Vector2::new(0.0, 0.0),
            Vector2::new(0.0, 1.0),
            1.0,
            1.0,
            0.0,
            0.0,
        ),
        PhysicsParticle::new(
            Vector2::new(10.0, 10.0),
            Vector2::new(10.0, 10.0),
            Vector2::new(3.0, 4.0),
            Vector2::new(0.0, 0.0),
            Vector2::new(0.0, 1.0),
            1.0,
            1.0,
            2.0,
            5.0,
        ),
    ];

    stabilize_physics_particles(
        &mut particles,
        Vector2::new(2.0, 3.0),
        0.0,
        Vector2::new(0.0, 0.0),
        0.001,
    );

    assert_eq!(particles[0].position(), Vector2::new(2.0, 3.0));
    assert_eq!(particles[1].position(), Vector2::new(2.0, 8.0));
    assert_eq!(particles[1].velocity(), Vector2::new(0.0, 0.0));
}
