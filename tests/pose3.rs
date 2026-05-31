use rusty_live2d::json::Pose3;

#[test]
fn parses_pose3_groups_and_links() {
    let pose = Pose3::from_json_str(
        r#"{
            "Type": "Live2D Pose",
            "FadeInTime": 0.5,
            "Groups": [
                [
                    { "Id": "PartArmLA", "Link": ["PartArmLB"] },
                    { "Id": "PartArmRA" }
                ]
            ]
        }"#,
    )
    .unwrap();

    assert_eq!(pose.kind(), "Live2D Pose");
    assert_eq!(pose.fade_in_time(), Some(0.5));
    assert_eq!(pose.groups().len(), 1);
    assert_eq!(pose.groups()[0][0].id(), "PartArmLA");
    assert_eq!(pose.groups()[0][0].links(), ["PartArmLB"]);
    assert_eq!(pose.groups()[0][1].id(), "PartArmRA");
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
