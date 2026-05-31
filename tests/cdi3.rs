use rusty_live2d::{Error, json::Cdi3};

#[test]
fn parses_cdi3_display_metadata() {
    let cdi = Cdi3::from_json_str(
        r#"{
            "Version": 3,
            "Parameters": [
                {
                    "Id": "ParamAngleX",
                    "GroupId": "ParamGroupFace",
                    "Name": "Angle X"
                }
            ],
            "ParameterGroups": [
                {
                    "Id": "ParamGroupFace",
                    "GroupId": "",
                    "Name": "Face"
                }
            ],
            "Parts": [
                {
                    "Id": "PartCore",
                    "Name": "Core"
                }
            ],
            "CombinedParameters": [
                ["ParamAngleX", "ParamAngleY"]
            ]
        }"#,
    )
    .unwrap();

    assert_eq!(cdi.version(), 3);
    assert_eq!(cdi.parameters()[0].id(), "ParamAngleX");
    assert_eq!(cdi.parameters()[0].group_id(), "ParamGroupFace");
    assert_eq!(cdi.parameters()[0].name(), "Angle X");

    assert_eq!(cdi.parameter_groups()[0].id(), "ParamGroupFace");
    assert_eq!(cdi.parameter_groups()[0].group_id(), "");
    assert_eq!(cdi.parameter_groups()[0].name(), "Face");

    assert_eq!(cdi.parts()[0].id(), "PartCore");
    assert_eq!(cdi.parts()[0].name(), "Core");

    assert_eq!(cdi.combined_parameters()[0], ["ParamAngleX", "ParamAngleY"]);
}

#[test]
fn cdi3_omitted_arrays_default_to_empty() {
    let cdi = Cdi3::from_json_str(
        r#"{
            "Version": 3
        }"#,
    )
    .unwrap();

    assert!(cdi.parameters().is_empty());
    assert!(cdi.parameter_groups().is_empty());
    assert!(cdi.parts().is_empty());
    assert!(cdi.combined_parameters().is_empty());
}

#[test]
fn rejects_unsupported_cdi3_version() {
    let error = Cdi3::from_json_str(
        r#"{
            "Version": 2
        }"#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        Error::UnsupportedVersion {
            format: "cdi3.json",
            version: 2
        }
    ));
}
