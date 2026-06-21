use mocari::core::{
    ArrayInterpolationGroup, InterpolationGroup, interpolate_float32, interpolate_float32_array,
    interpolate_float32_array_grouped, interpolate_float32_grouped, interpolate_int32,
};

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.00001,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn interpolates_float32_weighted_sum() {
    assert_close(
        interpolate_float32(&[10.0, 20.0], &[0.25, 0.75]).unwrap(),
        17.5,
    );
    assert!(interpolate_float32(&[10.0], &[0.5, 0.5]).is_none());
}

#[test]
fn interpolates_int32_with_core_truncation_bias() {
    assert_eq!(interpolate_int32(&[1.0, 5.0], &[0.5, 0.5]).unwrap(), 3);
    assert_eq!(interpolate_int32(&[-2.0, -1.0], &[0.5, 0.5]).unwrap(), -1);
}

#[test]
fn interpolates_float32_arrays() {
    let arrays: [&[f32]; 2] = [&[1.0, 2.0, 3.0], &[10.0, 20.0, 30.0]];
    let out = interpolate_float32_array(&arrays, &[0.25, 0.75], 3).unwrap();

    assert_close(out[0], 7.75);
    assert_close(out[1], 15.5);
    assert_close(out[2], 23.25);
    assert!(interpolate_float32_array(&arrays, &[1.0], 3).is_none());
}

#[test]
fn interpolates_float32_grouped_outputs() {
    let groups = [
        InterpolationGroup::new(0, 0, 2, 7),
        InterpolationGroup::new(1, 2, 2, 8),
    ];
    let out = interpolate_float32_grouped(
        &[1.0, 10.0, 100.0, 1000.0],
        &[0.5, 0.25, 0.1, 0.05],
        &groups,
        Some(&[true, false]),
    )
    .unwrap();

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].out_index(), 7);
    assert_close(out[0].value(), 3.0);
}

#[test]
fn interpolates_float32_arrays_grouped_outputs() {
    let arrays: [&[f32]; 3] = [
        &[1.0, 2.0, 3.0],
        &[10.0, 20.0, 30.0],
        &[100.0, 200.0, 300.0],
    ];
    let groups = [
        ArrayInterpolationGroup::new(0, 0, 2, 0, 3),
        ArrayInterpolationGroup::new(1, 1, 2, 1, 2),
    ];
    let mut outputs = [vec![9.0, 9.0, 9.0], vec![8.0, 8.0, 8.0]];

    interpolate_float32_array_grouped(
        &arrays,
        &[0.25, 0.75, 0.5],
        &groups,
        &mut outputs,
        Some(&[true, false]),
    )
    .unwrap();

    assert_close(outputs[0][0], 7.75);
    assert_close(outputs[0][1], 15.5);
    assert_close(outputs[0][2], 23.25);
    assert_eq!(outputs[1], vec![8.0, 8.0, 8.0]);

    interpolate_float32_array_grouped(&arrays, &[0.25, 0.75, 0.5], &groups, &mut outputs, None)
        .unwrap();

    assert_close(outputs[1][0], 57.5);
    assert_close(outputs[1][1], 115.0);
    assert_eq!(outputs[1][2], 8.0);
}
