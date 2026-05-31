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
            0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 9.0, 9.0, 0.1, 0.1, 1.1, 0.1, 1.1, 1.1, 0.25, 0.25, 0.75,
            0.75, 8.0, 8.0,
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
