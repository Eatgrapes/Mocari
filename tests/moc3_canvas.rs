use rusty_live2d::{Error, moc3::Moc3CanvasInfo};

#[test]
fn parses_moc3_canvas_info() {
    let bytes = moc3_with_canvas_info([2976.0, 1488.0, 2087.5, 2976.0, 4175.0], 1);

    let canvas = Moc3CanvasInfo::parse(&bytes).unwrap();

    assert_eq!(canvas.pixels_per_unit(), 2976.0);
    assert_eq!(canvas.origin_x(), 1488.0);
    assert_eq!(canvas.origin_y(), 2087.5);
    assert_eq!(canvas.width(), 2976.0);
    assert_eq!(canvas.height(), 4175.0);
    assert!(canvas.reverse_y_coordinate());
}

#[test]
fn rejects_incomplete_moc3_canvas_info() {
    let bytes = moc3_with_offsets(0x80, 0x100, 0x120);
    let error = Moc3CanvasInfo::parse(&bytes).unwrap_err();

    assert!(matches!(error, Error::InvalidMoc3 { .. }));
}

fn moc3_with_canvas_info(values: [f32; 5], flags: u8) -> Vec<u8> {
    let mut bytes = moc3_with_offsets(0x80, 0x100, 0x180);
    let mut cursor = 0x100;

    for value in values {
        bytes[cursor..cursor + 4].copy_from_slice(&value.to_le_bytes());
        cursor += 4;
    }

    bytes[cursor] = flags;
    bytes
}

fn moc3_with_offsets(count_info_offset: u32, canvas_info_offset: u32, len: usize) -> Vec<u8> {
    let mut bytes = vec![0; len];
    bytes[0..4].copy_from_slice(b"MOC3");
    bytes[4] = 1;
    bytes[0x40..0x44].copy_from_slice(&count_info_offset.to_le_bytes());
    bytes[0x44..0x48].copy_from_slice(&canvas_info_offset.to_le_bytes());
    bytes
}
