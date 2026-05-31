use rusty_live2d::{
    Error,
    json::{Motion3, MotionPoint, MotionSegment},
    json::{
        apply_motion_fade, easing_sine, motion_fade_in_weight, motion_fade_out_weight,
        parameter_curve_fade_weight,
    },
};

#[test]
fn parses_motion3_meta_and_curve_identity() {
    let motion = Motion3::from_json_str(
        r#"{
            "Version": 3,
            "Meta": {
                "Duration": 1.0,
                "Fps": 30.0,
                "Loop": true,
                "AreBeziersRestricted": false,
                "CurveCount": 1,
                "TotalSegmentCount": 1,
                "TotalPointCount": 2,
                "UserDataCount": 0,
                "TotalUserDataSize": 0
            },
            "Curves": [
                {
                    "Target": "Parameter",
                    "Id": "ParamAngleX",
                    "Segments": [0.0, 0.0, 0, 1.0, 10.0]
                }
            ]
        }"#,
    )
    .unwrap();

    assert_eq!(motion.version(), 3);
    assert_eq!(motion.meta().duration(), 1.0);
    assert_eq!(motion.meta().fps(), 30.0);
    assert!(motion.meta().is_looping());
    assert_eq!(motion.curves()[0].target(), "Parameter");
    assert_eq!(motion.curves()[0].id(), "ParamAngleX");
}

#[test]
fn samples_linear_motion_segment() {
    let curve = Motion3::from_json_str(
        r#"{
            "Version": 3,
            "Meta": { "Duration": 1.0, "Fps": 30.0, "Loop": false },
            "Curves": [
                {
                    "Target": "Parameter",
                    "Id": "ParamAngleX",
                    "Segments": [0.0, 0.0, 0, 1.0, 10.0]
                }
            ]
        }"#,
    )
    .unwrap()
    .curves()[0]
        .clone();

    assert_eq!(curve.sample(0.0).unwrap(), 0.0);
    assert_eq!(curve.sample(0.5).unwrap(), 5.0);
    assert_eq!(curve.sample(1.0).unwrap(), 10.0);
}

#[test]
fn motion_segment_linear_matches_framework_extrapolation() {
    let segment = MotionSegment::Linear {
        start: MotionPoint {
            time: 0.0,
            value: 0.0,
        },
        end: MotionPoint {
            time: 1.0,
            value: 10.0,
        },
    };

    assert_eq!(segment.sample(2.0, false).unwrap(), 20.0);
}

#[test]
fn samples_stepped_motion_segment() {
    let curve = Motion3::from_json_str(
        r#"{
            "Version": 3,
            "Meta": { "Duration": 1.0, "Fps": 30.0, "Loop": false },
            "Curves": [
                {
                    "Target": "Parameter",
                    "Id": "ParamEyeLOpen",
                    "Segments": [0.0, 0.0, 2, 1.0, 1.0]
                }
            ]
        }"#,
    )
    .unwrap()
    .curves()[0]
        .clone();

    assert_eq!(curve.sample(0.0).unwrap(), 0.0);
    assert_eq!(curve.sample(0.5).unwrap(), 0.0);
    assert_eq!(curve.sample(1.0).unwrap(), 1.0);
}

#[test]
fn motion_segment_steps_match_framework_evaluators() {
    let start = MotionPoint {
        time: 0.0,
        value: 2.0,
    };
    let end = MotionPoint {
        time: 1.0,
        value: 7.0,
    };

    let stepped = MotionSegment::Stepped { start, end };
    let inverse_stepped = MotionSegment::InverseStepped { start, end };

    assert_eq!(stepped.sample(1.0, false).unwrap(), 2.0);
    assert_eq!(stepped.sample(2.0, false).unwrap(), 2.0);
    assert_eq!(inverse_stepped.sample(0.0, false).unwrap(), 7.0);
    assert_eq!(inverse_stepped.sample(0.5, false).unwrap(), 7.0);
}

#[test]
fn samples_restricted_bezier_motion_segment() {
    let curve = Motion3::from_json_str(
        r#"{
            "Version": 3,
            "Meta": {
                "Duration": 1.0,
                "Fps": 30.0,
                "Loop": false,
                "AreBeziersRestricted": true
            },
            "Curves": [
                {
                    "Target": "Parameter",
                    "Id": "ParamAngleX",
                    "Segments": [0.0, 0.0, 1, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]
                }
            ]
        }"#,
    )
    .unwrap()
    .curves()[0]
        .clone();

    assert_close(curve.sample(0.5).unwrap(), 0.5);
}

#[test]
fn samples_unrestricted_bezier_motion_segment_by_solving_time() {
    let curve = Motion3::from_json_str(
        r#"{
            "Version": 3,
            "Meta": {
                "Duration": 1.0,
                "Fps": 30.0,
                "Loop": false,
                "AreBeziersRestricted": false
            },
            "Curves": [
                {
                    "Target": "Parameter",
                    "Id": "ParamAngleX",
                    "Segments": [0.0, 0.0, 1, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]
                }
            ]
        }"#,
    )
    .unwrap()
    .curves()[0]
        .clone();

    assert_close(curve.sample(0.5).unwrap(), 0.889_881_6);
}

#[test]
fn motion_fade_uses_framework_easing_sine() {
    assert_eq!(easing_sine(-0.5), 0.0);
    assert_eq!(easing_sine(1.5), 1.0);
    assert_close(easing_sine(0.5), 0.5);

    assert_eq!(motion_fade_in_weight(0.5, 0.0, 0.0), 1.0);
    assert_close(motion_fade_in_weight(0.5, 0.0, 1.0), 0.5);
    assert_eq!(motion_fade_out_weight(2.0, -1.0, 1.0), 1.0);
    assert_close(motion_fade_out_weight(0.5, 1.0, 1.0), 0.5);
}

#[test]
fn parameter_curve_fade_matches_framework_override_rules() {
    assert_close(
        parameter_curve_fade_weight(0.3, 0.5, 0.75, None, None, 1.0, 0.0, 4.0),
        0.3,
    );

    assert_close(
        parameter_curve_fade_weight(0.8, 0.25, 0.75, Some(2.0), None, 1.0, 0.0, 4.0),
        0.3,
    );

    assert_close(
        parameter_curve_fade_weight(0.8, 0.25, 0.75, Some(0.0), Some(0.0), 1.0, 0.0, 4.0),
        0.8,
    );
}

#[test]
fn applies_motion_fade_to_source_and_target_values() {
    assert_eq!(apply_motion_fade(2.0, 10.0, 0.25), 4.0);
}

#[test]
fn rejects_unsupported_motion3_version() {
    let error = Motion3::from_json_str(
        r#"{
            "Version": 2,
            "Meta": { "Duration": 1.0, "Fps": 30.0, "Loop": false },
            "Curves": []
        }"#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        Error::UnsupportedVersion {
            format: "motion3.json",
            version: 2
        }
    ));
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.0001,
        "actual {actual}, expected {expected}"
    );
}
