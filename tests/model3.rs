use rusty_live2d::{Error, json::Model3};

#[test]
fn parses_minimal_model3_file_references() {
    let model = Model3::from_json_str(
        r#"{
            "Version": 3,
            "FileReferences": {
                "Moc": "hiyori_free_t08.moc3",
                "Textures": ["hiyori_free_t08.2048/texture_00.png"],
                "Physics": "hiyori_free_t08.physics3.json",
                "DisplayInfo": "hiyori_free_t08.cdi3.json"
            }
        }"#,
    )
    .unwrap();

    assert_eq!(model.version(), 3);
    assert_eq!(model.moc(), "hiyori_free_t08.moc3");
    assert_eq!(model.textures(), ["hiyori_free_t08.2048/texture_00.png"]);
    assert_eq!(model.physics(), Some("hiyori_free_t08.physics3.json"));
    assert_eq!(model.display_info(), Some("hiyori_free_t08.cdi3.json"));
}

#[test]
fn parses_model3_motions_groups_and_hit_areas() {
    let model = Model3::from_json_str(
        r#"{
            "Version": 3,
            "FileReferences": {
                "Moc": "model.moc3",
                "Textures": ["texture_00.png"],
                "Motions": {
                    "Idle": [
                        { "File": "motion/idle.motion3.json" }
                    ]
                }
            },
            "Groups": [
                {
                    "Target": "Parameter",
                    "Name": "EyeBlink",
                    "Ids": ["ParamEyeLOpen", "ParamEyeROpen"]
                }
            ],
            "HitAreas": [
                { "Id": "HitArea", "Name": "Body" }
            ]
        }"#,
    )
    .unwrap();

    let idle = model.motions().get("Idle").unwrap();
    assert_eq!(idle[0].file(), "motion/idle.motion3.json");

    assert_eq!(model.groups()[0].target(), "Parameter");
    assert_eq!(model.groups()[0].name(), "EyeBlink");
    assert_eq!(model.groups()[0].ids(), ["ParamEyeLOpen", "ParamEyeROpen"]);

    assert_eq!(model.hit_areas()[0].id(), "HitArea");
    assert_eq!(model.hit_areas()[0].name(), "Body");
}

#[test]
fn rejects_unsupported_model3_version() {
    let error = Model3::from_json_str(
        r#"{
            "Version": 4,
            "FileReferences": {
                "Moc": "model.moc3",
                "Textures": ["texture_00.png"]
            }
        }"#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        Error::UnsupportedVersion {
            format: "model3.json",
            version: 4
        }
    ));
}
