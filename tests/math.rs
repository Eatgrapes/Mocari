use std::collections::BTreeMap;

use rusty_live2d::core::{Matrix44, ModelMatrix};

#[test]
fn matrix44_transforms_scaled_and_translated_points() {
    let mut matrix = Matrix44::identity();
    matrix.scale(2.0, 3.0);
    matrix.translate(10.0, -4.0);

    assert_eq!(matrix.transform_x(5.0), 20.0);
    assert_eq!(matrix.transform_y(2.0), 2.0);
    assert_eq!(matrix.invert_transform_x(20.0), 5.0);
    assert_eq!(matrix.invert_transform_y(2.0), 2.0);
}

#[test]
fn model_matrix_initializes_to_height_two() {
    let matrix = ModelMatrix::new(4.0, 8.0);

    assert_eq!(matrix.transform_x(4.0), 1.0);
    assert_eq!(matrix.transform_y(8.0), 2.0);
}

#[test]
fn model_matrix_applies_layout_size_before_position() {
    let mut matrix = ModelMatrix::new(4.0, 8.0);
    let mut layout = BTreeMap::new();
    layout.insert("width".to_string(), 4.0);
    layout.insert("center_x".to_string(), 0.0);
    layout.insert("top".to_string(), 1.0);

    matrix.setup_from_layout(&layout);

    assert_eq!(matrix.transform_x(0.0), -2.0);
    assert_eq!(matrix.transform_x(4.0), 2.0);
    assert_eq!(matrix.transform_y(0.0), 1.0);
    assert_eq!(matrix.transform_y(8.0), 9.0);
}
