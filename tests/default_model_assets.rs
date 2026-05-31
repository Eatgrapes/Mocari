use std::path::Path;

use rusty_live2d::assets::{DEFAULT_MODEL3_PATH, load_default_model};

#[test]
fn default_model_assets_are_project_local_and_renderable() {
    assert!(Path::new(DEFAULT_MODEL3_PATH).is_file());
    assert!(DEFAULT_MODEL3_PATH.starts_with("assets/models/"));

    let model = load_default_model().expect("load default model assets");

    assert!(model.model().moc().ends_with(".moc3"));
    assert!(!model.meshes().is_empty());
    assert!(!model.textures().is_empty());
    assert!(model.textures()[0].width() > 0);
    assert!(model.textures()[0].height() > 0);
    assert_eq!(
        model.textures()[0].rgba().len(),
        model.textures()[0].width() as usize * model.textures()[0].height() as usize * 4
    );
}
