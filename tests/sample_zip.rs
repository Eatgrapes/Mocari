use std::env;
use std::fs::File;
use std::io::Read;

use mocari::{
    json::{Cdi3, Model3, Motion3, Physics3},
    moc3::{
        Moc3ArtMeshKeyforms, Moc3ArtMeshes, Moc3CanvasInfo, Moc3CountInfo, Moc3Header, Moc3Ids,
        Moc3SectionOffsets, build_moc3_drawable_mesh, build_moc3_drawable_meshes,
    },
};
use zip::ZipArchive;

#[test]
fn parses_model3_from_opt_in_sample_zip() {
    let Some(mut archive) = open_sample_archive() else {
        return;
    };

    let model_json = read_first_text_entry(&mut archive, ".model3.json")
        .expect("sample zip contains a model3 json");
    let model = Model3::from_json_str(&model_json).expect("parse model3 json from sample zip");

    assert_eq!(model.version(), 3);
    assert!(!model.moc().is_empty());
    assert!(!model.textures().is_empty());
}

#[test]
fn parses_runtime_assets_from_opt_in_sample_zip() {
    let Some(mut archive) = open_sample_archive() else {
        return;
    };

    let cdi =
        read_first_text_entry(&mut archive, ".cdi3.json").expect("sample zip contains a cdi3 json");
    assert!(!Cdi3::from_json_str(&cdi).unwrap().parameters().is_empty());

    let physics = read_first_text_entry(&mut archive, ".physics3.json")
        .expect("sample zip contains a physics3 json");
    assert!(
        !Physics3::from_json_str(&physics)
            .unwrap()
            .settings()
            .is_empty()
    );

    let motions = read_all_text_entries(&mut archive, ".motion3.json");
    assert!(!motions.is_empty());
    for motion in motions {
        assert!(!Motion3::from_json_str(&motion).unwrap().curves().is_empty());
    }

    let moc3 = read_first_binary_entry(&mut archive, ".moc3").expect("sample zip contains a moc3");
    Moc3Header::parse(&moc3).unwrap();
    Moc3SectionOffsets::parse(&moc3).unwrap();
    let counts = Moc3CountInfo::parse(&moc3).unwrap();
    assert!(counts.parameters() > 0);
    assert!(counts.art_meshes() > 0);

    let ids = Moc3Ids::parse(&moc3).unwrap();
    assert_eq!(ids.parameters().len(), counts.parameters() as usize);
    assert_eq!(ids.art_meshes().len(), counts.art_meshes() as usize);

    let art_meshes = Moc3ArtMeshes::parse(&moc3).unwrap();
    assert_eq!(art_meshes.meshes().len(), counts.art_meshes() as usize);
    assert!(!art_meshes.uv_xys().is_empty());
    assert!(!art_meshes.position_indices().is_empty());

    let art_mesh_keyforms = Moc3ArtMeshKeyforms::parse(&moc3).unwrap();
    assert_eq!(
        art_mesh_keyforms.keyforms().len(),
        counts.art_mesh_keyforms() as usize
    );
    assert!(!art_mesh_keyforms.position_xys().is_empty());
    assert!(art_mesh_keyforms.art_mesh_keyform_positions(0, 0).is_some());
    assert!(build_moc3_drawable_mesh(&art_meshes, &art_mesh_keyforms, 0, 0).is_some());
    assert_eq!(
        build_moc3_drawable_meshes(&art_meshes, &art_mesh_keyforms)
            .unwrap()
            .len(),
        counts.art_meshes() as usize
    );

    let canvas = Moc3CanvasInfo::parse(&moc3).unwrap();
    assert!(canvas.width() > 0.0);
    assert!(canvas.height() > 0.0);
    assert!(canvas.pixels_per_unit() > 0.0);
}

fn open_sample_archive() -> Option<ZipArchive<File>> {
    let Some(zip_path) = env::var_os("MODEL_FIXTURE_ARCHIVE") else {
        eprintln!("skipping sample zip test; MODEL_FIXTURE_ARCHIVE is not set");
        return None;
    };

    let file = File::open(&zip_path).expect("open sample zip");
    Some(ZipArchive::new(file).expect("read sample zip"))
}

fn read_first_text_entry(archive: &mut ZipArchive<File>, suffix: &str) -> Option<String> {
    read_all_text_entries(archive, suffix).into_iter().next()
}

fn read_all_text_entries(archive: &mut ZipArchive<File>, suffix: &str) -> Vec<String> {
    let mut entries = Vec::new();

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).expect("read zip entry");
        if !entry.name().ends_with(suffix) {
            continue;
        }

        let mut source = String::new();
        entry
            .read_to_string(&mut source)
            .expect("read text entry from sample zip");
        entries.push(source);
    }

    entries
}

fn read_first_binary_entry(archive: &mut ZipArchive<File>, suffix: &str) -> Option<Vec<u8>> {
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).expect("read zip entry");
        if !entry.name().ends_with(suffix) {
            continue;
        }

        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .expect("read binary entry from sample zip");
        return Some(bytes);
    }

    None
}
