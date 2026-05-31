use rusty_live2d::core::{clamp_parameter_value, core_repeat_fold, parameter_dirty};

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.00001,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn clamps_parameter_values_to_min_max() {
    assert_eq!(clamp_parameter_value(-2.0, -1.0, 1.0), -1.0);
    assert_eq!(clamp_parameter_value(2.0, -1.0, 1.0), 1.0);
    assert_eq!(clamp_parameter_value(0.25, -1.0, 1.0), 0.25);
}

#[test]
fn repeat_fold_uses_core_floor_like_correction() {
    assert_close(core_repeat_fold(370.0, 0.0, 360.0), 10.0);
    assert_close(core_repeat_fold(-10.0, 0.0, 360.0), 350.0);
    assert_close(core_repeat_fold(-360.0, 0.0, 360.0), 0.0);
}

#[test]
fn repeat_fold_keeps_large_q_path_truncated() {
    let value = 8_388_609.25_f32;

    assert_eq!(core_repeat_fold(value, 0.0, 1.0), 0.0);
}

#[test]
fn dirty_flag_treats_nan_as_dirty() {
    assert!(!parameter_dirty(1.0, 1.0));
    assert!(parameter_dirty(1.0, 2.0));
    assert!(parameter_dirty(f32::NAN, 1.0));
    assert!(parameter_dirty(1.0, f32::NAN));
}
