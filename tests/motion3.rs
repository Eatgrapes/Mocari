use rusty_live2d::{Error, json::Motion3};

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
