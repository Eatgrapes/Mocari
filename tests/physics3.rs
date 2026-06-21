use mocari::{
    Error,
    json::{Physics3, PhysicsValueKind},
};

#[test]
fn parses_physics3_meta_and_setting_shape() {
    let physics = Physics3::from_json_str(
        r#"{
            "Version": 3,
            "Meta": {
                "PhysicsSettingCount": 1,
                "TotalInputCount": 1,
                "TotalOutputCount": 1,
                "VertexCount": 2,
                "EffectiveForces": {
                    "Gravity": { "X": 0.0, "Y": -1.0 },
                    "Wind": { "X": 0.0, "Y": 0.0 }
                },
                "PhysicsDictionary": [
                    { "Id": "PhysicsSetting1", "Name": "Hair" }
                ]
            },
            "PhysicsSettings": [
                {
                    "Id": "PhysicsSetting1",
                    "Input": [
                        {
                            "Source": { "Target": "Parameter", "Id": "ParamAngleX" },
                            "Weight": 60.0,
                            "Type": "X",
                            "Reflect": false
                        }
                    ],
                    "Output": [
                        {
                            "Destination": { "Target": "Parameter", "Id": "ParamHairFront" },
                            "VertexIndex": 1,
                            "Scale": 1.0,
                            "Weight": 100.0,
                            "Type": "Angle",
                            "Reflect": true
                        }
                    ],
                    "Vertices": [
                        {
                            "Mobility": 1.0,
                            "Delay": 0.2,
                            "Acceleration": 0.8,
                            "Radius": 10.0,
                            "Position": { "X": 0.0, "Y": 0.0 }
                        },
                        {
                            "Mobility": 0.8,
                            "Delay": 0.3,
                            "Acceleration": 0.7,
                            "Radius": 20.0,
                            "Position": { "X": 0.0, "Y": -20.0 }
                        }
                    ],
                    "Normalization": {
                        "Position": { "Minimum": -30.0, "Default": 0.0, "Maximum": 30.0 },
                        "Angle": { "Minimum": -45.0, "Default": 0.0, "Maximum": 45.0 }
                    }
                }
            ]
        }"#,
    )
    .unwrap();

    assert_eq!(physics.version(), 3);
    assert_eq!(physics.meta().physics_setting_count(), 1);
    assert_eq!(physics.meta().effective_forces().gravity().y(), -1.0);
    assert_eq!(physics.meta().physics_dictionary()[0].name(), "Hair");

    let setting = &physics.settings()[0];
    assert_eq!(setting.id(), "PhysicsSetting1");
    assert_eq!(setting.inputs()[0].source().id(), "ParamAngleX");
    assert_eq!(setting.inputs()[0].kind(), PhysicsValueKind::X);
    assert_eq!(setting.outputs()[0].destination().id(), "ParamHairFront");
    assert_eq!(setting.outputs()[0].kind(), PhysicsValueKind::Angle);
    assert!(setting.outputs()[0].reflect());
    assert_eq!(setting.vertices()[1].position().y(), -20.0);
    assert_eq!(setting.normalization().angle().maximum(), 45.0);
}

#[test]
fn rejects_unsupported_physics3_version() {
    let error = Physics3::from_json_str(
        r#"{
            "Version": 2,
            "Meta": {
                "PhysicsSettingCount": 0,
                "TotalInputCount": 0,
                "TotalOutputCount": 0,
                "VertexCount": 0,
                "EffectiveForces": {
                    "Gravity": { "X": 0.0, "Y": -1.0 },
                    "Wind": { "X": 0.0, "Y": 0.0 }
                },
                "PhysicsDictionary": []
            },
            "PhysicsSettings": []
        }"#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        Error::UnsupportedVersion {
            format: "physics3.json",
            version: 2
        }
    ));
}
