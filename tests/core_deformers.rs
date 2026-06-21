use mocari::core::{
    DeformerTransform, Vector2, WarpInterpolation, rotation_deformer_transform_point,
    transform_art_mesh_vertices_by_deformers, warp_deformer_transform_inside,
    warp_deformer_transform_target,
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
fn rotation_deformer_applies_confirmed_forward_matrix() {
    let point = Vector2::new(1.0, 2.0);

    let transformed =
        rotation_deformer_transform_point(point, 90.0, 2.0, Vector2::new(10.0, 20.0), false, false);

    assert_vec_close(transformed, Vector2::new(6.0, 22.0));
}

#[test]
fn rotation_deformer_applies_flip_signs_to_axes() {
    let point = Vector2::new(1.0, 2.0);

    let transformed =
        rotation_deformer_transform_point(point, 90.0, 2.0, Vector2::new(10.0, 20.0), true, true);

    assert_vec_close(transformed, Vector2::new(14.0, 18.0));
}

#[test]
fn warp_deformer_inside_quad_uses_bilinear_grid_interpolation() {
    let grid = [
        Vector2::new(0.0, 0.0),
        Vector2::new(10.0, 0.0),
        Vector2::new(0.0, 20.0),
        Vector2::new(20.0, 20.0),
    ];

    let transformed = warp_deformer_transform_inside(
        Vector2::new(0.75, 0.75),
        &grid,
        1,
        1,
        WarpInterpolation::Quad,
    )
    .unwrap();

    assert_vec_close(transformed, Vector2::new(13.125, 15.0));
}

#[test]
fn warp_deformer_inside_triangle_uses_cell_diagonal() {
    let grid = [
        Vector2::new(0.0, 0.0),
        Vector2::new(10.0, 0.0),
        Vector2::new(0.0, 20.0),
        Vector2::new(20.0, 20.0),
    ];

    let lower = warp_deformer_transform_inside(
        Vector2::new(0.25, 0.5),
        &grid,
        1,
        1,
        WarpInterpolation::Triangle,
    )
    .unwrap();
    let upper = warp_deformer_transform_inside(
        Vector2::new(0.75, 0.75),
        &grid,
        1,
        1,
        WarpInterpolation::Triangle,
    )
    .unwrap();

    assert_vec_close(lower, Vector2::new(2.5, 10.0));
    assert_vec_close(upper, Vector2::new(12.5, 15.0));
}

#[test]
fn warp_deformer_inside_rejects_outside_or_incomplete_grid() {
    let grid = [
        Vector2::new(0.0, 0.0),
        Vector2::new(1.0, 0.0),
        Vector2::new(0.0, 1.0),
        Vector2::new(1.0, 1.0),
    ];

    assert!(
        warp_deformer_transform_inside(
            Vector2::new(1.0, 0.5),
            &grid,
            1,
            1,
            WarpInterpolation::Quad
        )
        .is_none()
    );
    assert!(
        warp_deformer_transform_inside(
            Vector2::new(0.5, 0.5),
            &grid[..3],
            1,
            1,
            WarpInterpolation::Quad
        )
        .is_none()
    );
}

#[test]
fn warp_deformer_target_extrapolates_points_outside_unit_grid() {
    let grid = [
        Vector2::new(0.0, 0.0),
        Vector2::new(10.0, 0.0),
        Vector2::new(0.0, 20.0),
        Vector2::new(10.0, 20.0),
    ];

    let transformed = warp_deformer_transform_target(
        Vector2::new(1.25, 0.5),
        &grid,
        1,
        1,
        WarpInterpolation::Quad,
    )
    .unwrap();

    assert_vec_close(transformed, Vector2::new(12.5, 10.0));
}

#[test]
fn applies_deformer_path_to_art_mesh_vertices() {
    let grid = [
        Vector2::new(10.0, 20.0),
        Vector2::new(12.0, 20.0),
        Vector2::new(10.0, 22.0),
        Vector2::new(12.0, 22.0),
    ];
    let vertices = [Vector2::new(0.25, 0.0), Vector2::new(0.0, -0.25)];
    let transforms = [
        DeformerTransform::Rotation {
            angle_degrees: 90.0,
            scale: 1.0,
            translation: Vector2::new(0.25, 0.25),
            flip_x: false,
            flip_y: false,
        },
        DeformerTransform::Warp {
            grid: &grid,
            cols: 1,
            rows: 1,
            interpolation: WarpInterpolation::Quad,
        },
    ];

    let out = transform_art_mesh_vertices_by_deformers(&vertices, &transforms).unwrap();

    assert_vec_close(out[0], Vector2::new(10.5, 21.0));
    assert_vec_close(out[1], Vector2::new(11.0, 20.5));
}
