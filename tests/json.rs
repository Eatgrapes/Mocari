mod model3 {
    use rusty_live2d::{Error, json::Model3};

    #[test]
    fn parses_minimal_model3_file_references() {
        let model = Model3::from_json_str(
            r#"{
                "Version": 3,
                "FileReferences": {
                    "Moc": "sample_model.moc3",
                    "Textures": ["sample_model.2048/texture_00.png"],
                    "Physics": "sample_model.physics3.json",
                    "DisplayInfo": "sample_model.cdi3.json"
                }
            }"#,
        )
        .unwrap();

        assert_eq!(model.version(), 3);
        assert_eq!(model.moc(), "sample_model.moc3");
        assert_eq!(model.textures(), ["sample_model.2048/texture_00.png"]);
        assert_eq!(model.physics(), Some("sample_model.physics3.json"));
        assert_eq!(model.display_info(), Some("sample_model.cdi3.json"));
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
                        "Name": "ParameterGroupA",
                        "Ids": ["ParamInputA", "ParamInputB"]
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
        assert_eq!(model.groups()[0].name(), "ParameterGroupA");
        assert_eq!(model.groups()[0].ids(), ["ParamInputA", "ParamInputB"]);

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
}

mod expression3 {
    use rusty_live2d::json::{
        Expression3, ExpressionBlend, apply_expression_blend, apply_expression_parameter,
    };

    #[test]
    fn parses_expression3_parameters() {
        let expression = Expression3::from_json_str(
            r#"{
                "Type": "Live2D Expression",
                "FadeInTime": 0.5,
                "FadeOutTime": 0.25,
                "Parameters": [
                    { "Id": "ParamInputA", "Value": 0.0, "Blend": "Overwrite" },
                    { "Id": "ParamMouthOpenY", "Value": 0.7, "Blend": "Add" },
                    { "Id": "ParamCheek", "Value": 0.5, "Blend": "Multiply" }
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(expression.kind(), "Live2D Expression");
        assert_eq!(expression.fade_in_time(), Some(0.5));
        assert_eq!(expression.fade_out_time(), Some(0.25));
        assert_eq!(expression.parameters()[0].id(), "ParamInputA");
        assert_eq!(expression.parameters()[0].value(), 0.0);
        assert_eq!(
            expression.parameters()[0].blend(),
            ExpressionBlend::Overwrite
        );
        assert_eq!(expression.parameters()[1].blend(), ExpressionBlend::Add);
        assert_eq!(
            expression.parameters()[2].blend(),
            ExpressionBlend::Multiply
        );
    }

    #[test]
    fn expression3_defaults_to_add_blend() {
        let expression = Expression3::from_json_str(
            r#"{
                "Type": "Live2D Expression",
                "Parameters": [
                    { "Id": "ParamInputA", "Value": 1.0 }
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(expression.parameters()[0].blend(), ExpressionBlend::Add);
    }

    #[test]
    fn expression3_unknown_blend_falls_back_to_add() {
        let expression = Expression3::from_json_str(
            r#"{
                "Type": "Live2D Expression",
                "Parameters": [
                    { "Id": "ParamInputA", "Value": 1.0, "Blend": "Screen" }
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(expression.parameters()[0].blend(), ExpressionBlend::Add);
    }

    #[test]
    fn applies_expression_blend_modes() {
        assert_eq!(
            apply_expression_blend(0.25, 0.5, ExpressionBlend::Add, 0.4),
            0.45
        );
        assert_eq!(
            apply_expression_blend(2.0, 1.5, ExpressionBlend::Multiply, 0.25),
            2.25
        );
        assert_eq!(
            apply_expression_blend(0.25, 0.75, ExpressionBlend::Overwrite, 0.5),
            0.5
        );
    }

    #[test]
    fn applies_expression_parameter() {
        let expression = Expression3::from_json_str(
            r#"{
                "Type": "Live2D Expression",
                "Parameters": [
                    { "Id": "ParamMouthOpenY", "Value": 0.7, "Blend": "Add" }
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(
            apply_expression_parameter(0.2, &expression.parameters()[0], 0.5),
            0.55
        );
    }
}

mod cdi3 {
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
}

mod pose3 {
    use rusty_live2d::json::{
        Pose3, copy_pose_link_opacities, resolved_pose_fade_in_time, update_pose_group_opacities,
    };

    #[test]
    fn parses_pose3_groups_and_links() {
        let pose = Pose3::from_json_str(
            r#"{
                "Type": "Live2D Pose",
                "FadeInTime": 0.5,
                "Groups": [
                    [
                        { "Id": "PartSegmentA", "Link": ["PartSegmentB"] },
                        { "Id": "PartSegmentC" }
                    ]
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(pose.kind(), "Live2D Pose");
        assert_eq!(pose.fade_in_time(), Some(0.5));
        assert_eq!(pose.groups().len(), 1);
        assert_eq!(pose.groups()[0][0].id(), "PartSegmentA");
        assert_eq!(pose.groups()[0][0].links(), ["PartSegmentB"]);
        assert_eq!(pose.groups()[0][1].id(), "PartSegmentC");
        assert!(pose.groups()[0][1].links().is_empty());
    }

    #[test]
    fn pose3_defaults_missing_groups_to_empty() {
        let pose = Pose3::from_json_str(
            r#"{
                "Type": "Live2D Pose"
            }"#,
        )
        .unwrap();

        assert!(pose.groups().is_empty());
    }

    #[test]
    fn resolves_pose_fade_time_like_framework() {
        let pose = Pose3::from_json_str(
            r#"{
                "Type": "Live2D Pose",
                "FadeInTime": -1.0
            }"#,
        )
        .unwrap();
        let missing = Pose3::from_json_str(r#"{ "Type": "Live2D Pose" }"#).unwrap();

        assert_eq!(resolved_pose_fade_in_time(pose.fade_in_time()), 0.5);
        assert_eq!(pose.resolved_fade_in_time(), 0.5);
        assert_eq!(missing.resolved_fade_in_time(), 0.5);
    }

    #[test]
    fn updates_pose_group_opacity_with_background_threshold() {
        let parameters = [0.0_f32, 1.0];
        let mut opacities = [1.0_f32, 0.0];

        update_pose_group_opacities(&parameters, &mut opacities, 0.1, 0.5).unwrap();

        assert!((opacities[0] - 0.8125).abs() < 0.00001);
        assert!((opacities[1] - 0.2).abs() < 0.00001);
    }

    #[test]
    fn pose_group_falls_back_to_first_part_when_no_parameter_visible() {
        let parameters = [0.0_f32, 0.0];
        let mut opacities = [0.25_f32, 0.5];

        update_pose_group_opacities(&parameters, &mut opacities, -0.2, 0.5).unwrap();

        assert_eq!(opacities, [1.0, 0.0]);
    }

    #[test]
    fn copies_pose_link_opacities() {
        let mut opacities = [0.35, 0.0, 1.0];

        copy_pose_link_opacities(&mut opacities, 0, &[1, 2]).unwrap();

        assert_eq!(opacities, [0.35, 0.35, 0.35]);
    }
}
