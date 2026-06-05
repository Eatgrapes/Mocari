use rusty_live2d::{
    Error,
    moc3::{Moc3ArtMeshInfo, Moc3ArtMeshes},
};

#[test]
fn parses_moc3_art_mesh_render_sections() {
    let bytes = moc3_with_art_meshes();

    let art_meshes = Moc3ArtMeshes::parse(&bytes).unwrap();

    assert_eq!(art_meshes.meshes().len(), 2);
    assert_eq!(
        art_meshes.meshes()[0],
        Moc3ArtMeshInfo::new(1, 0b0000_0011, 6, 0, 0, 4, 0, 1)
    );
    assert_eq!(
        art_meshes.meshes()[1],
        Moc3ArtMeshInfo::new(0, 0b0000_0100, 3, 8, 6, 2, 1, 1)
    );
    assert_eq!(
        art_meshes.art_mesh_uvs(0).unwrap(),
        &[0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0]
    );
    assert_eq!(
        art_meshes.art_mesh_position_indices(0).unwrap(),
        &[0, 1, 2, 0, 2, 3]
    );
    assert_eq!(art_meshes.art_mesh_masks(0).unwrap(), &[1]);
    assert_eq!(
        art_meshes.art_mesh_uvs(1).unwrap(),
        &[0.25, 0.25, 0.75, 0.75]
    );
    assert_eq!(art_meshes.art_mesh_position_indices(1).unwrap(), &[0, 1, 0]);
    assert_eq!(art_meshes.art_mesh_masks(1).unwrap(), &[0]);
}

#[test]
fn rejects_incomplete_moc3_art_mesh_section() {
    let mut bytes = moc3_with_art_meshes();
    bytes.truncate(0xa90);

    let error = Moc3ArtMeshes::parse(&bytes).unwrap_err();

    assert!(matches!(error, Error::InvalidMoc3 { .. }));
}

fn moc3_with_art_meshes() -> Vec<u8> {
    let mut bytes = vec![0; 0xc00];
    bytes[0..4].copy_from_slice(b"MOC3");
    bytes[4] = 1;

    write_u32(&mut bytes, 0x40, 0x7c0);
    write_u32(&mut bytes, 0x44, 0x840);

    write_section_offset(&mut bytes, 41, 0x880);
    write_section_offset(&mut bytes, 42, 0x8c0);
    write_section_offset(&mut bytes, 43, 0x900);
    write_section_offset(&mut bytes, 44, 0x940);
    write_section_offset(&mut bytes, 45, 0x980);
    write_section_offset(&mut bytes, 46, 0x9c0);
    write_section_offset(&mut bytes, 47, 0xa00);
    write_section_offset(&mut bytes, 48, 0xa40);
    write_section_offset(&mut bytes, 78, 0xa80);
    write_section_offset(&mut bytes, 79, 0xb40);
    write_section_offset(&mut bytes, 80, 0xb80);

    write_u32(&mut bytes, 0x7d0, 2);
    write_u32(&mut bytes, 0x7fc, 12);
    write_u32(&mut bytes, 0x800, 9);
    write_u32(&mut bytes, 0x804, 2);

    write_i32_array(&mut bytes, 0x880, &[1, 0]);
    bytes[0x8c0..0x8c2].copy_from_slice(&[0b0000_0011, 0b0000_0100]);
    write_i32_array(&mut bytes, 0x900, &[4, 2]);
    write_i32_array(&mut bytes, 0x940, &[0, 8]);
    write_i32_array(&mut bytes, 0x980, &[0, 6]);
    write_i32_array(&mut bytes, 0x9c0, &[6, 3]);
    write_i32_array(&mut bytes, 0xa00, &[0, 1]);
    write_i32_array(&mut bytes, 0xa40, &[1, 1]);
    write_f32_array(
        &mut bytes,
        0xa80,
        &[
            0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.25, 0.25, 0.75, 0.75,
        ],
    );
    write_i16_array(&mut bytes, 0xb40, &[0, 1, 2, 0, 2, 3, 0, 1, 0]);
    write_i32_array(&mut bytes, 0xb80, &[1, 0]);

    bytes
}

fn write_section_offset(bytes: &mut [u8], slot: usize, offset: u32) {
    write_u32(bytes, 0x40 + slot * 4, offset);
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_i32_array(bytes: &mut [u8], offset: usize, values: &[i32]) {
    for (index, value) in values.iter().enumerate() {
        bytes[offset + index * 4..offset + index * 4 + 4].copy_from_slice(&value.to_le_bytes());
    }
}

fn write_i16_array(bytes: &mut [u8], offset: usize, values: &[i16]) {
    for (index, value) in values.iter().enumerate() {
        bytes[offset + index * 2..offset + index * 2 + 2].copy_from_slice(&value.to_le_bytes());
    }
}

fn write_f32_array(bytes: &mut [u8], offset: usize, values: &[f32]) {
    for (index, value) in values.iter().enumerate() {
        bytes[offset + index * 4..offset + index * 4 + 4].copy_from_slice(&value.to_le_bytes());
    }
}

mod moc3_art_mesh_keyforms {
    use rusty_live2d::{
        Error,
        moc3::{Moc3ArtMeshKeyformInfo, Moc3ArtMeshKeyforms},
    };

    #[test]
    fn parses_moc3_art_mesh_keyform_positions() {
        let bytes = moc3_with_art_mesh_keyforms();

        let keyforms = Moc3ArtMeshKeyforms::parse(&bytes).unwrap();

        assert_eq!(keyforms.keyforms().len(), 3);
        assert_eq!(
            keyforms.keyforms()[0],
            Moc3ArtMeshKeyformInfo::new(0.5, 100.0, 0)
        );
        assert_eq!(
            keyforms.keyforms()[2],
            Moc3ArtMeshKeyformInfo::new(0.75, 50.0, 14)
        );
        assert_eq!(
            keyforms.art_mesh_keyforms(0).unwrap(),
            &[
                Moc3ArtMeshKeyformInfo::new(0.5, 100.0, 0),
                Moc3ArtMeshKeyformInfo::new(1.0, 200.0, 8),
            ]
        );
        assert_eq!(
            keyforms.art_mesh_keyform_positions(0, 0).unwrap(),
            &[0.0, 0.0, 1.0, 0.0, 1.0, 1.0]
        );
        assert_eq!(
            keyforms.art_mesh_keyform_positions(0, 1).unwrap(),
            &[0.1, 0.1, 1.1, 0.1, 1.1, 1.1]
        );
        assert_eq!(
            keyforms.art_mesh_keyform_positions(1, 0).unwrap(),
            &[0.25, 0.25, 0.75, 0.75]
        );
    }

    #[test]
    fn rejects_out_of_range_moc3_art_mesh_keyform_positions() {
        let mut bytes = moc3_with_art_mesh_keyforms();
        write_i32_array(&mut bytes, 0x9c0, &[0, 8, 18]);

        let error = Moc3ArtMeshKeyforms::parse(&bytes).unwrap_err();

        assert!(matches!(error, Error::InvalidMoc3 { .. }));
    }

    fn moc3_with_art_mesh_keyforms() -> Vec<u8> {
        let mut bytes = vec![0; 0xb00];
        bytes[0..4].copy_from_slice(b"MOC3");
        bytes[4] = 1;

        write_u32(&mut bytes, 0x40, 0x7c0);
        write_u32(&mut bytes, 0x44, 0x840);

        write_section_offset(&mut bytes, 35, 0x880);
        write_section_offset(&mut bytes, 36, 0x8c0);
        write_section_offset(&mut bytes, 43, 0x900);
        write_section_offset(&mut bytes, 68, 0x940);
        write_section_offset(&mut bytes, 69, 0x980);
        write_section_offset(&mut bytes, 70, 0x9c0);
        write_section_offset(&mut bytes, 71, 0xa00);

        write_u32(&mut bytes, 0x7d0, 2);
        write_u32(&mut bytes, 0x7e4, 3);
        write_u32(&mut bytes, 0x7e8, 20);

        write_i32_array(&mut bytes, 0x880, &[0, 2]);
        write_i32_array(&mut bytes, 0x8c0, &[2, 1]);
        write_i32_array(&mut bytes, 0x900, &[3, 2]);
        write_f32_array(&mut bytes, 0x940, &[0.5, 1.0, 0.75]);
        write_f32_array(&mut bytes, 0x980, &[100.0, 200.0, 50.0]);
        write_i32_array(&mut bytes, 0x9c0, &[0, 8, 14]);
        write_f32_array(
            &mut bytes,
            0xa00,
            &[
                0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 9.0, 9.0, 0.1, 0.1, 1.1, 0.1, 1.1, 1.1, 0.25, 0.25,
                0.75, 0.75, 8.0, 8.0,
            ],
        );

        bytes
    }

    fn write_section_offset(bytes: &mut [u8], slot: usize, offset: u32) {
        write_u32(bytes, 0x40 + slot * 4, offset);
    }

    fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_i32_array(bytes: &mut [u8], offset: usize, values: &[i32]) {
        for (index, value) in values.iter().enumerate() {
            bytes[offset + index * 4..offset + index * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
    }

    fn write_f32_array(bytes: &mut [u8], offset: usize, values: &[f32]) {
        for (index, value) in values.iter().enumerate() {
            bytes[offset + index * 4..offset + index * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
    }
}

mod moc3_drawable_mesh {
    use std::fs;

    use rusty_live2d::moc3::{
        Moc3ArtMeshInfo, Moc3ArtMeshKeyformInfo, Moc3ArtMeshKeyforms, Moc3ArtMeshes, Moc3Deformers,
        Moc3DrawableBlendMode, Moc3Ids, Moc3KeyformBindings, Moc3OffscreenInfo,
        build_moc3_drawable_mesh, build_moc3_drawable_meshes,
        build_moc3_drawable_meshes_for_default_pose,
        build_moc3_drawable_meshes_for_default_pose_with_offscreen_state,
    };

    #[test]
    fn builds_moc3_drawable_mesh_from_art_mesh_sections() {
        let art_meshes = Moc3ArtMeshes::from_parts(
            vec![Moc3ArtMeshInfo::new(2, 0b0000_0100, 6, 0, 0, 4, 0, 1)],
            vec![0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0],
            vec![0, 1, 2, 0, 2, 3],
            vec![7],
        )
        .unwrap();
        let keyforms = Moc3ArtMeshKeyforms::from_parts(
            vec![0],
            vec![1],
            vec![4],
            vec![Moc3ArtMeshKeyformInfo::new(0.8, 500.0, 0)],
            vec![-1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0],
        )
        .unwrap();

        let mesh = build_moc3_drawable_mesh(&art_meshes, &keyforms, 0, 0).unwrap();

        assert_eq!(mesh.texture_index(), 2);
        assert_eq!(mesh.drawable_flags(), 0b0000_0100);
        assert_eq!(mesh.opacity(), 0.8);
        assert_eq!(mesh.draw_order(), 500.0);
        assert_eq!(mesh.masks(), &[7]);
        assert_eq!(mesh.indices(), &[0, 1, 2, 0, 2, 3]);
        assert_eq!(mesh.vertices().len(), 4);
        assert_eq!(mesh.vertices()[0].position(), [-1.0, -1.0]);
        assert_eq!(mesh.vertices()[0].uv(), [0.0, 0.0]);
        assert_eq!(mesh.vertices()[2].position(), [1.0, 1.0]);
        assert_eq!(mesh.vertices()[2].uv(), [1.0, 1.0]);
    }

    #[test]
    fn resolves_drawable_blend_mode_from_core_flags() {
        assert_eq!(
            Moc3DrawableBlendMode::from_flags(0),
            Moc3DrawableBlendMode::Normal
        );
        assert_eq!(
            Moc3DrawableBlendMode::from_flags(1 << 0),
            Moc3DrawableBlendMode::Additive
        );
        assert_eq!(
            Moc3DrawableBlendMode::from_flags(1 << 1),
            Moc3DrawableBlendMode::Multiplicative
        );
    }

    #[test]
    fn rejects_moc3_drawable_mesh_with_out_of_range_indices() {
        let art_meshes = Moc3ArtMeshes::from_parts(
            vec![Moc3ArtMeshInfo::new(0, 0, 3, 0, 0, 2, 0, 0)],
            vec![0.0, 0.0, 1.0, 1.0],
            vec![0, 1, 2],
            Vec::new(),
        )
        .unwrap();
        let keyforms = Moc3ArtMeshKeyforms::from_parts(
            vec![0],
            vec![1],
            vec![2],
            vec![Moc3ArtMeshKeyformInfo::new(1.0, 0.0, 0)],
            vec![0.0, 0.0, 1.0, 1.0],
        )
        .unwrap();

        assert!(build_moc3_drawable_mesh(&art_meshes, &keyforms, 0, 0).is_none());
    }

    #[test]
    fn builds_all_moc3_drawable_meshes_from_first_keyform() {
        let art_meshes = Moc3ArtMeshes::from_parts(
            vec![
                Moc3ArtMeshInfo::new(0, 0, 3, 0, 0, 3, 0, 0),
                Moc3ArtMeshInfo::new(1, 4, 3, 6, 3, 3, 0, 0),
            ],
            vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.5, 0.5, 1.0, 0.5, 0.5, 1.0],
            vec![0, 1, 2, 0, 1, 2],
            Vec::new(),
        )
        .unwrap();
        let keyforms = Moc3ArtMeshKeyforms::from_parts(
            vec![0, 1],
            vec![1, 1],
            vec![3, 3],
            vec![
                Moc3ArtMeshKeyformInfo::new(1.0, 10.0, 0),
                Moc3ArtMeshKeyformInfo::new(0.5, 20.0, 6),
            ],
            vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 2.0, 2.0, 3.0, 2.0, 2.0, 3.0],
        )
        .unwrap();

        let meshes = build_moc3_drawable_meshes(&art_meshes, &keyforms).unwrap();

        assert_eq!(meshes.len(), 2);
        assert_eq!(meshes[0].texture_index(), 0);
        assert_eq!(meshes[1].texture_index(), 1);
        assert_eq!(meshes[1].opacity(), 0.5);
        assert_eq!(meshes[1].vertices()[0].position(), [2.0, 2.0]);
    }

    #[test]
    fn builds_bundled_model_default_poses_from_bound_keyforms() {
        for name in [
            "Wanko", "Hiyori", "Haru", "Ren", "Mark", "Rice", "Natori", "Mao",
        ] {
            let bytes = fs::read(format!("assets/models/{name}/{name}.moc3")).unwrap();
            let art_meshes = Moc3ArtMeshes::parse(&bytes).unwrap();
            let keyforms = Moc3ArtMeshKeyforms::parse(&bytes).unwrap();
            let deformers = Moc3Deformers::parse(&bytes).unwrap();
            let bindings = Moc3KeyformBindings::parse(&bytes).unwrap();

            let meshes = build_moc3_drawable_meshes_for_default_pose(
                &art_meshes,
                &keyforms,
                &deformers,
                &bindings,
            )
            .unwrap_or_else(|| panic!("{name} default pose should build from bound keyforms"));

            assert_eq!(meshes.len(), art_meshes.meshes().len());
        }
    }

    #[test]
    fn parses_bundled_v53_offscreen_visibility_state() {
        let bytes = fs::read("assets/models/Ren/Ren.moc3").unwrap();

        let offscreen = Moc3OffscreenInfo::parse(&bytes).unwrap();

        assert_eq!(offscreen.offscreen_count(), 24);
        assert_eq!(offscreen.part_offscreen_index(17), Some(4));
        assert_eq!(offscreen.part_offscreen_index(41), Some(17));
        assert_eq!(offscreen.offscreen_owner_part_index(4), Some(17));
        assert_eq!(offscreen.offscreen_owner_part_index(17), Some(41));
        assert_eq!(offscreen.drawable_parent_part_index(91), Some(18));
        assert_eq!(offscreen.drawable_parent_part_index(105), Some(17));
    }

    #[test]
    fn bundled_v53_default_pose_keeps_effect_source_drawables_transparent() {
        let bytes = fs::read("assets/models/Ren/Ren.moc3").unwrap();
        let art_meshes = Moc3ArtMeshes::parse(&bytes).unwrap();
        let keyforms = Moc3ArtMeshKeyforms::parse(&bytes).unwrap();
        let deformers = Moc3Deformers::parse(&bytes).unwrap();
        let bindings = Moc3KeyformBindings::parse(&bytes).unwrap();
        let ids = Moc3Ids::parse(&bytes).unwrap();
        let offscreen = Moc3OffscreenInfo::parse(&bytes).unwrap();

        let meshes = build_moc3_drawable_meshes_for_default_pose_with_offscreen_state(
            &art_meshes,
            &keyforms,
            &deformers,
            &bindings,
            &ids,
            &offscreen,
        )
        .unwrap();

        for index in [
            91, 92, 93, 94, 95, 98, 99, 100, 101, 102, 103, 105, 196, 197,
        ] {
            assert_eq!(meshes[index].opacity(), 0.0, "drawable {index}");
        }
    }
}
