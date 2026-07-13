use mocari::{
    assets::{load_model, load_model_runtime},
    expression::{ExpressionManager, ExpressionPlayer, load_expression},
    json::{Expression3, Motion3},
    motion::MotionPlayer,
};

#[test]
fn runtime_default_pose_matches_default_model() {
    let runtime = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let default = load_model("assets/models/Haru/Haru.model3.json").unwrap();

    let runtime_meshes = runtime.runtime().meshes();
    let default_meshes = default.meshes();

    // Geometry must match; opacity may differ because the runtime applies the
    // pose3 part groups (hiding the redundant arm) while load_model does not.
    assert_eq!(runtime_meshes.len(), default_meshes.len());
    for (left, right) in runtime_meshes.iter().zip(default_meshes) {
        assert_eq!(left.vertices(), right.vertices());
    }
}

#[test]
fn loaded_textures_match_image_crate_rgba_output() {
    let model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let texture_paths = [
        "assets/models/Hiyori/Hiyori.2048/texture_00.png",
        "assets/models/Hiyori/Hiyori.2048/texture_01.png",
    ];

    assert_eq!(model.textures().len(), texture_paths.len());
    for (texture, path) in model.textures().iter().zip(texture_paths) {
        let expected = image::open(path).unwrap().to_rgba8();

        assert_eq!(texture.width(), expected.width());
        assert_eq!(texture.height(), expected.height());
        assert_eq!(texture.rgba(), expected.as_raw());
    }
}

#[test]
fn setting_a_parameter_changes_mesh_vertices() {
    let mut model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let before: Vec<_> = model
        .runtime()
        .meshes()
        .iter()
        .map(|mesh| mesh.vertices().to_vec())
        .collect();

    let angle_x = model
        .runtime()
        .parameter_ids()
        .iter()
        .find(|id| id.as_str() == "ParamAngleX")
        .cloned()
        .expect("Haru has ParamAngleX");
    model.runtime_mut().set_parameter(&angle_x, 30.0);
    model.runtime_mut().update_meshes().unwrap();

    let after: Vec<_> = model
        .runtime()
        .meshes()
        .iter()
        .map(|mesh| mesh.vertices().to_vec())
        .collect();

    assert_ne!(before, after, "moving ParamAngleX should deform the mesh");
}

#[test]
fn runtime_hit_tests_model3_hit_areas() {
    let model = load_model_runtime("assets/models/Mao/Mao.model3.json").unwrap();
    let runtime = model.runtime();
    let body = runtime
        .model()
        .hit_areas()
        .iter()
        .find(|hit_area| hit_area.name() == "Body")
        .expect("Mao declares Body hit area");
    let drawable_index = runtime
        .drawable_index(body.id())
        .expect("hit area id references a drawable");
    let (x, y) = drawable_center(&runtime.meshes()[drawable_index]);

    let hit = runtime.hit_test(x, y).expect("body center should hit");

    assert_eq!(hit.id(), body.id());
    assert_eq!(hit.name(), "Body");
    assert_eq!(hit.drawable_index(), drawable_index);
    assert!(runtime.hit_test(10_000.0, 10_000.0).is_none());
}

#[test]
fn updating_parameters_reuses_runtime_mesh_storage() {
    let mut model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let mesh_ptr = model.runtime().meshes().as_ptr();
    let vertex_ptrs: Vec<_> = model
        .runtime()
        .meshes()
        .iter()
        .map(|mesh| mesh.vertices().as_ptr())
        .collect();

    assert!(model.runtime_mut().set_parameter("ParamAngleX", 30.0));
    model.runtime_mut().update_meshes().unwrap();

    assert_eq!(model.runtime().meshes().as_ptr(), mesh_ptr);
    assert_eq!(
        model
            .runtime()
            .meshes()
            .iter()
            .map(|mesh| mesh.vertices().as_ptr())
            .collect::<Vec<_>>(),
        vertex_ptrs
    );
}

fn drawable_center(mesh: &mocari::moc3::Moc3DrawableMesh) -> (f32, f32) {
    let first = mesh.vertices().first().expect("hit drawable has vertices");
    let [mut min_x, mut min_y] = first.position();
    let mut max_x = min_x;
    let mut max_y = min_y;

    for vertex in mesh.vertices().iter().skip(1) {
        let [x, y] = vertex.position();
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }

    ((min_x + max_x) * 0.5, (min_y + max_y) * 0.5)
}

#[test]
fn set_parameter_clamps_to_model_range() {
    let mut model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let id = "ParamAngleX";
    let index = model.runtime().parameter_index(id).unwrap();

    model.runtime_mut().set_parameter(id, 1_000_000.0);
    let high = model.runtime().parameter_value_by_index(index).unwrap();
    model.runtime_mut().set_parameter(id, -1_000_000.0);
    let low = model.runtime().parameter_value_by_index(index).unwrap();

    assert!(high < 1_000_000.0, "value must be clamped to the maximum");
    assert!(low > -1_000_000.0, "value must be clamped to the minimum");
    assert!(low < high);
}

#[test]
fn parameter_info_exposes_range_default_and_current_value() {
    let model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let index = model.runtime().parameter_index("ParamAngleX").unwrap();
    let info = model.runtime().parameter_info_by_index(index).unwrap();

    assert_eq!(info.id(), "ParamAngleX");
    assert!(info.minimum() < info.maximum());
    assert!(info.minimum() <= info.default());
    assert!(info.default() <= info.maximum());
    assert_eq!(info.default(), info.value());
    assert_eq!(
        model.runtime().parameter_infos().count(),
        model.runtime().parameter_ids().len()
    );
}

#[test]
fn set_parameter_normalized_maps_unit_range_to_model_range() {
    let mut model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let index = model.runtime().parameter_index("ParamAngleX").unwrap();
    let info = model.runtime().parameter_info_by_index(index).unwrap();
    let expected = info.minimum() + (info.maximum() - info.minimum()) * 0.75;

    assert!(
        model
            .runtime_mut()
            .set_parameter_normalized_by_index(index, 0.75)
    );

    assert_close(
        model.runtime().parameter_value_by_index(index).unwrap(),
        expected,
    );
    assert_close(
        model
            .runtime()
            .parameter_normalized_value_by_index(index)
            .unwrap(),
        0.75,
    );

    assert!(
        model
            .runtime_mut()
            .set_parameter_normalized("ParamAngleX", 2.0)
    );
    assert_close(
        model
            .runtime()
            .parameter_normalized_value_by_index(index)
            .unwrap(),
        1.0,
    );
}

#[test]
fn parameter_overrides_can_be_applied_after_parameter_reset() {
    let mut model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let index = model.runtime().parameter_index("ParamAngleX").unwrap();
    let info = model.runtime().parameter_info_by_index(index).unwrap();
    let maximum = info.maximum();
    let default = info.default();

    assert!(
        model
            .runtime_mut()
            .set_parameter_override_normalized_by_index(index, 1.0)
    );
    model.runtime_mut().reset_parameters();
    model.runtime_mut().apply_parameter_overrides();
    assert_close(
        model.runtime().parameter_value_by_index(index).unwrap(),
        maximum,
    );
    assert_close(
        model
            .runtime()
            .parameter_override_normalized_value_by_index(index)
            .unwrap(),
        1.0,
    );

    assert!(model.runtime_mut().clear_parameter_override_by_index(index));
    model.runtime_mut().reset_parameters();
    model.runtime_mut().apply_parameter_overrides();
    assert_close(
        model.runtime().parameter_value_by_index(index).unwrap(),
        default,
    );
    assert!(
        model
            .runtime()
            .parameter_override_value_by_index(index)
            .is_none()
    );
}

#[test]
fn motion_player_drives_a_parameter_over_time() {
    let mut model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let motion =
        mocari::motion::load_motion("assets/models/Haru/motions/haru_g_idle.motion3.json").unwrap();

    let target = motion
        .curves()
        .iter()
        .find(|curve| curve.target() == "Parameter")
        .map(|curve| curve.id().to_owned())
        .expect("idle motion has a parameter curve");
    let index = model
        .runtime()
        .parameter_index(&target)
        .expect("motion parameter exists on model");

    let mut player = MotionPlayer::new(motion);
    player.tick(0.5);
    player.apply(model.runtime_mut());
    let first = model.runtime().parameter_value_by_index(index).unwrap();

    player.tick(0.5);
    player.apply(model.runtime_mut());
    let second = model.runtime().parameter_value_by_index(index).unwrap();

    assert!(first.is_finite() && second.is_finite());
}

#[test]
fn non_looping_motion_finishes_after_duration() {
    let motion = Motion3::from_json_str(
        r#"{
            "Version": 3,
            "Meta": { "Duration": 1.0, "Fps": 30.0, "Loop": false },
            "Curves": [
                { "Target": "Parameter", "Id": "ParamAngleX", "Segments": [0.0, 0.0, 0, 1.0, 10.0] }
            ]
        }"#,
    )
    .unwrap();

    let mut player = MotionPlayer::new(motion);
    assert!(!player.is_finished());
    player.tick(0.5);
    assert!(!player.is_finished());
    player.tick(0.6);
    assert!(player.is_finished());
    assert_eq!(player.time(), 1.0);
}

#[test]
fn looping_motion_wraps_time() {
    let motion = Motion3::from_json_str(
        r#"{
            "Version": 3,
            "Meta": { "Duration": 1.0, "Fps": 30.0, "Loop": true },
            "Curves": [
                { "Target": "Parameter", "Id": "ParamAngleX", "Segments": [0.0, 0.0, 0, 1.0, 10.0] }
            ]
        }"#,
    )
    .unwrap();

    let mut player = MotionPlayer::new(motion);
    player.tick(1.5);
    assert!(!player.is_finished());
    assert!((player.time() - 0.5).abs() < 0.0001);
}

#[test]
fn one_shot_player_finishes_looping_motion() {
    let motion = Motion3::from_json_str(
        r#"{
            "Version": 3,
            "Meta": { "Duration": 1.0, "Fps": 30.0, "Loop": true },
            "Curves": [
                { "Target": "Parameter", "Id": "ParamAngleX", "Segments": [0.0, 0.0, 0, 1.0, 10.0] }
            ]
        }"#,
    )
    .unwrap();

    let mut player = MotionPlayer::new_once(motion);

    assert!(!player.is_looping());
    player.tick(1.5);
    assert!(player.is_finished());
    assert_eq!(player.time(), 1.0);
}

fn hiyori_mesh_snapshot(model: &mocari::assets::RuntimeModel) -> Vec<Vec<[f32; 2]>> {
    model
        .runtime()
        .meshes()
        .iter()
        .map(|mesh| mesh.vertices().iter().map(|v| v.position()).collect())
        .collect()
}

#[test]
fn hiyori_distinct_bindings_drive_distinct_parameters() {
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let baseline = hiyori_mesh_snapshot(&model);

    model.runtime_mut().set_parameter("ParamRibbon", 1.0);
    model.runtime_mut().update_meshes().unwrap();
    let ribbon = hiyori_mesh_snapshot(&model);
    assert_ne!(
        baseline, ribbon,
        "ParamRibbon should deform the Hiyori mesh"
    );

    model.runtime_mut().reset_parameters();
    model.runtime_mut().set_parameter("ParamSkirt2", 1.0);
    model.runtime_mut().update_meshes().unwrap();
    let skirt = hiyori_mesh_snapshot(&model);
    assert_ne!(baseline, skirt, "ParamSkirt2 should deform the Hiyori mesh");

    assert_ne!(
        ribbon, skirt,
        "distinct parameters must drive distinct deformations"
    );
}

#[test]
fn zeroing_a_part_hides_its_drawables() {
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let baseline_visible = model
        .runtime()
        .meshes()
        .iter()
        .filter(|mesh| mesh.opacity() > 0.0)
        .count();
    assert!(baseline_visible > 0);

    let part_ids: Vec<String> = model.runtime().part_ids().to_vec();
    let mut hid_some = false;
    for part_id in part_ids {
        model.runtime_mut().reset_part_opacities();
        model.runtime_mut().set_part_opacity(&part_id, 0.0);
        model.runtime_mut().update_meshes().unwrap();
        let visible = model
            .runtime()
            .meshes()
            .iter()
            .filter(|mesh| mesh.opacity() > 0.0)
            .count();
        if visible < baseline_visible {
            hid_some = true;
            assert!(visible > 0, "a single part should not hide the whole model");
            break;
        }
    }
    assert!(hid_some, "no part hid any drawable");
}

#[test]
fn mao_drawables_carry_non_identity_multiply_and_screen_colors() {
    let model = load_model_runtime("assets/models/Mao/Mao.model3.json").unwrap();
    let non_identity = model
        .runtime()
        .meshes()
        .iter()
        .filter(|mesh| {
            mesh.multiply_color() != [1.0, 1.0, 1.0] || mesh.screen_color() != [0.0, 0.0, 0.0]
        })
        .count();
    assert!(
        non_identity > 0,
        "Mao should expose per-drawable color keyforms"
    );
}

#[test]
fn expression_player_applies_faded_expression_parameters() {
    let mut model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let expression = Expression3::from_json_str(
        r#"{
            "Type": "Live2D Expression",
            "Parameters": [
                { "Id": "ParamAngleX", "Value": 10.0, "Blend": "Add" }
            ]
        }"#,
    )
    .unwrap();
    let index = model.runtime().parameter_index("ParamAngleX").unwrap();
    let default = model.runtime().parameter_value_by_index(index).unwrap();

    let mut player = ExpressionPlayer::new(expression);
    player.tick(0.5);
    player.apply(model.runtime_mut());

    let value = model.runtime().parameter_value_by_index(index).unwrap();
    assert_close(value, default + 5.0);
}

#[test]
fn expression_manager_fades_out_previous_expression_when_playing_next() {
    let mut model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let first = Expression3::from_json_str(
        r#"{
            "Type": "Live2D Expression",
            "Parameters": [
                { "Id": "ParamAngleX", "Value": 10.0, "Blend": "Add" }
            ]
        }"#,
    )
    .unwrap();
    let second = Expression3::from_json_str(
        r#"{
            "Type": "Live2D Expression",
            "Parameters": [
                { "Id": "ParamAngleX", "Value": -4.0, "Blend": "Add" }
            ]
        }"#,
    )
    .unwrap();
    let index = model.runtime().parameter_index("ParamAngleX").unwrap();
    let default = model.runtime().parameter_value_by_index(index).unwrap();

    let mut manager = ExpressionManager::new();
    manager.play(first);
    manager.tick(1.0);
    model.runtime_mut().reset_parameters();
    manager.apply(model.runtime_mut());
    assert_close(
        model.runtime().parameter_value_by_index(index).unwrap(),
        default + 10.0,
    );

    manager.play(second);
    manager.tick(0.5);
    model.runtime_mut().reset_parameters();
    manager.apply(model.runtime_mut());

    let value = model.runtime().parameter_value_by_index(index).unwrap();
    assert_close(value, default + (10.0 * 0.5) + (-4.0 * 0.5));
    assert_eq!(manager.active_expression_count(), 2);

    manager.tick(0.5);
    assert_eq!(manager.active_expression_count(), 1);
}

#[test]
fn load_expression_reads_exp3_asset() {
    let expression = load_expression("assets/models/Haru/expressions/F01.exp3.json").unwrap();

    assert_eq!(expression.kind(), "Live2D Expression");
    assert_eq!(expression.parameters()[0].id(), "ParamMouthForm");
}

#[test]
fn mao_drawable_colors_match_core_default_pose() {
    let model = load_model_runtime("assets/models/Mao/Mao.model3.json").unwrap();
    let meshes = model.runtime().meshes();

    assert_color_close(meshes[45].multiply_color(), [1.0, 1.0, 1.0]);
    assert_color_close(meshes[45].screen_color(), [0.0, 0.0, 0.0]);
    assert_color_close(meshes[138].multiply_color(), [1.0, 1.0, 1.0]);
    assert_color_close(meshes[138].screen_color(), [1.0, 0.454_901_96, 0.513_725_5]);
}

#[test]
fn legacy_model_without_color_keyforms_defaults_to_identity() {
    let model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    for mesh in model.runtime().meshes() {
        assert_eq!(mesh.multiply_color(), [1.0, 1.0, 1.0]);
        assert_eq!(mesh.screen_color(), [0.0, 0.0, 0.0]);
    }
}

#[test]
fn deformer_opacity_hides_rest_state_effects() {
    let model = load_model_runtime("assets/models/Rice/Rice.model3.json").unwrap();
    let meshes = model.runtime().meshes();
    for &index in &[151usize, 152, 153] {
        assert_eq!(
            meshes[index].opacity(),
            0.0,
            "magic-circle drawable {index} must be hidden by deformer opacity at rest"
        );
    }
}

#[test]
fn default_pose_hides_redundant_arm_via_pose_groups() {
    let model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let hidden = model
        .runtime()
        .meshes()
        .iter()
        .filter(|mesh| mesh.opacity() == 0.0)
        .count();
    let visible = model
        .runtime()
        .meshes()
        .iter()
        .filter(|mesh| mesh.opacity() > 0.0)
        .count();

    assert!(
        hidden > 0,
        "pose group should hide the redundant arm at rest"
    );
    assert!(visible > 0, "the selected arm and body must stay visible");
}

#[test]
fn loaded_physics_updates_parameters_and_can_be_reset() {
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let runtime = model.runtime_mut();
    let initial_output = runtime.parameter_value("ParamHairFront").unwrap();

    assert!(runtime.physics().is_some());
    assert!(runtime.set_parameter("ParamAngleX", 30.0));
    for _ in 0..3 {
        assert!(runtime.apply_physics(1.0 / 30.0));
    }

    let output = runtime.parameter_value("ParamHairFront").unwrap();
    assert!(output.is_finite());
    assert_ne!(output, initial_output);
    assert!(runtime.reset_physics());
    assert!(runtime.stabilize_physics());
    runtime.clear_physics();
    assert!(runtime.physics().is_none());
    assert!(!runtime.apply_physics(1.0 / 30.0));
}

fn assert_color_close(actual: [f32; 3], expected: [f32; 3]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!((actual - expected).abs() < 0.0001);
    }
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.0001,
        "actual {actual}, expected {expected}"
    );
}
