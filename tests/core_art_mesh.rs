use rusty_live2d::core::{
    Vector2, affect_art_mesh_pair, apply_art_mesh_blend_shape_delta, apply_parent_part_opacity,
    draw_order_from_raw, reverse_coordinate_y,
};

fn assert_vec_close(actual: Vector2, expected: Vector2) {
    assert!(
        (actual.x() - expected.x()).abs() < 0.00001,
        "expected x {}, got {}",
        expected.x(),
        actual.x()
    );
    assert!(
        (actual.y() - expected.y()).abs() < 0.00001,
        "expected y {}, got {}",
        expected.y(),
        actual.y()
    );
}

#[test]
fn glue_affects_art_mesh_pair_with_directional_weights() {
    let (a, b) = affect_art_mesh_pair(
        Vector2::new(0.0, 0.0),
        Vector2::new(10.0, 20.0),
        0.25,
        0.5,
        0.8,
    );

    assert_vec_close(a, Vector2::new(2.0, 4.0));
    assert_vec_close(b, Vector2::new(6.0, 12.0));
}

#[test]
fn reverse_coordinate_flips_y_only() {
    let mut vertices = [Vector2::new(1.0, 2.0), Vector2::new(-3.0, -4.0)];

    reverse_coordinate_y(&mut vertices);

    assert_eq!(vertices, [Vector2::new(1.0, -2.0), Vector2::new(-3.0, 4.0)]);
}

#[test]
fn draw_order_uses_core_int_bias_and_clamps() {
    assert_eq!(draw_order_from_raw(12.999), 13);
    assert_eq!(draw_order_from_raw(-1.0), 0);
    assert_eq!(draw_order_from_raw(1001.0), 1000);
}

#[test]
fn applies_art_mesh_blend_shape_delta() {
    let mut positions = [0.0, 1.0, 2.0, 3.0];

    apply_art_mesh_blend_shape_delta(&mut positions, &[10.0, -10.0, 4.0, -4.0], 0.25).unwrap();
    apply_art_mesh_blend_shape_delta(&mut positions, &[1.0, 1.0, 1.0, 1.0], 0.0).unwrap();

    assert_eq!(positions, [2.5, -1.5, 3.0, 2.0]);
    assert!(apply_art_mesh_blend_shape_delta(&mut positions, &[1.0], 1.0).is_none());
}

#[test]
fn parent_part_opacity_multiplies_art_mesh_opacity() {
    assert_eq!(apply_parent_part_opacity(0.75, 0.5), 0.375);
}
