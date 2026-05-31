use rusty_live2d::core::{
    Vector2, affect_art_mesh_pair, draw_order_from_raw, reverse_coordinate_y,
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
