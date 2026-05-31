use rusty_live2d::core::{
    KeyformAxis, compute_keyform_axis_interval, expand_keyform_runtime_slots,
};

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.00001,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn computes_keyform_axis_interval() {
    let keys = [0.0, 10.0, 20.0];

    let below = compute_keyform_axis_interval(&keys, -1.0).unwrap();
    assert_eq!(below.left_index(), 0);
    assert_eq!(below.t(), 0.0);

    let middle = compute_keyform_axis_interval(&keys, 12.5).unwrap();
    assert_eq!(middle.left_index(), 1);
    assert_close(middle.t(), 0.25);

    let above = compute_keyform_axis_interval(&keys, 25.0).unwrap();
    assert_eq!(above.left_index(), 2);
    assert_eq!(above.t(), 0.0);

    assert!(compute_keyform_axis_interval(&[], 1.0).is_none());
}

#[test]
fn expands_runtime_slots_from_active_axes() {
    let axes = [
        KeyformAxis::new(1, 0.25, 1),
        KeyformAxis::new(2, 0.0, 4),
        KeyformAxis::new(0, 0.5, 12),
    ];

    let slots = expand_keyform_runtime_slots(&axes);

    assert_eq!(slots.len(), 4);
    assert_eq!(slots[0].flat_index(), 9);
    assert_close(slots[0].weight(), 0.375);
    assert_eq!(slots[1].flat_index(), 10);
    assert_close(slots[1].weight(), 0.125);
    assert_eq!(slots[2].flat_index(), 21);
    assert_close(slots[2].weight(), 0.375);
    assert_eq!(slots[3].flat_index(), 22);
    assert_close(slots[3].weight(), 0.125);
}
