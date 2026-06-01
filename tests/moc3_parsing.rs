mod moc3_header {
    use rusty_live2d::{
        Error,
        moc3::{Endianness, Moc3Header, Moc3Version},
    };

    #[test]
    fn parses_moc3_header() {
        let header = Moc3Header::parse(&header_bytes(1, 0)).unwrap();

        assert_eq!(header.version(), Moc3Version::V3_0_0);
        assert_eq!(header.endianness(), Endianness::Little);
    }

    #[test]
    fn parses_big_endian_moc3_header_flag() {
        let header = Moc3Header::parse(&header_bytes(5, 1)).unwrap();

        assert_eq!(header.version(), Moc3Version::V5_0_0);
        assert_eq!(header.endianness(), Endianness::Big);
    }

    #[test]
    fn rejects_short_moc3_header() {
        let error = Moc3Header::parse(b"MOC3").unwrap_err();

        assert!(matches!(error, Error::InvalidMoc3 { .. }));
    }

    #[test]
    fn rejects_invalid_moc3_magic() {
        let mut bytes = header_bytes(1, 0);
        bytes[0..4].copy_from_slice(b"NOPE");

        let error = Moc3Header::parse(&bytes).unwrap_err();

        assert!(matches!(error, Error::InvalidMoc3 { .. }));
    }

    #[test]
    fn rejects_unknown_moc3_version() {
        let error = Moc3Header::parse(&header_bytes(99, 0)).unwrap_err();

        assert!(matches!(
            error,
            Error::UnsupportedVersion {
                format: "moc3",
                version: 99
            }
        ));
    }

    fn header_bytes(version: u8, big_endian: u8) -> [u8; 64] {
        let mut bytes = [0; 64];
        bytes[0..4].copy_from_slice(b"MOC3");
        bytes[4] = version;
        bytes[5] = big_endian;
        bytes
    }
}

mod moc3_canvas {
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
        let bytes = moc3_with_offsets(0x7c0, 0x860, 0x880);
        let error = Moc3CanvasInfo::parse(&bytes).unwrap_err();

        assert!(matches!(error, Error::InvalidMoc3 { .. }));
    }

    fn moc3_with_canvas_info(values: [f32; 5], flags: u8) -> Vec<u8> {
        let mut bytes = moc3_with_offsets(0x7c0, 0x840, 0x880);
        let mut cursor = 0x840;

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
}

mod moc3_counts {
    use rusty_live2d::{Error, moc3::Moc3CountInfo};

    #[test]
    fn parses_basic_moc3_count_info() {
        let bytes = moc3_with_counts(&[
            15, 49, 41, 8, 83, 29, 15, 242, 24, 414, 41472, 35, 29, 29, 85, 4006, 7624, 63, 21, 23,
            0, 0, 0,
        ]);

        let counts = Moc3CountInfo::parse(&bytes).unwrap();

        assert_eq!(counts.parts(), 15);
        assert_eq!(counts.deformers(), 49);
        assert_eq!(counts.art_meshes(), 83);
        assert_eq!(counts.parameters(), 29);
        assert_eq!(counts.uvs(), 4006);
        assert_eq!(counts.position_indices(), 7624);
    }

    #[test]
    fn rejects_incomplete_moc3_count_info() {
        let bytes = moc3_with_count_fixture(&[15, 49], 0x890, 0x880, 0x840);
        let error = Moc3CountInfo::parse(&bytes).unwrap_err();

        assert!(matches!(error, Error::InvalidMoc3 { .. }));
    }

    fn moc3_with_counts(counts: &[u32]) -> Vec<u8> {
        moc3_with_count_fixture(counts, 0x880, 0x7c0, 0x840)
    }

    fn moc3_with_count_fixture(
        counts: &[u32],
        len: usize,
        count_offset: u32,
        canvas_offset: u32,
    ) -> Vec<u8> {
        let mut bytes = vec![0; len];
        bytes[0..4].copy_from_slice(b"MOC3");
        bytes[4] = 1;
        bytes[0x40..0x44].copy_from_slice(&count_offset.to_le_bytes());
        bytes[0x44..0x48].copy_from_slice(&canvas_offset.to_le_bytes());

        let mut cursor = count_offset as usize;
        for count in counts {
            bytes[cursor..cursor + 4].copy_from_slice(&count.to_le_bytes());
            cursor += 4;
        }

        bytes
    }
}

mod moc3_ids {
    use rusty_live2d::{Error, moc3::Moc3Ids};

    #[test]
    fn parses_moc3_fixed_width_id_sections() {
        let bytes = moc3_with_ids();

        let ids = Moc3Ids::parse(&bytes).unwrap();

        assert_eq!(ids.parts(), &["PartSegmentA", "PartSegmentB"]);
        assert_eq!(ids.art_meshes(), &["ArtMesh00", "ArtMesh01"]);
        assert_eq!(ids.parameters(), &["ParamAngleX", "ParamInputA"]);
    }

    #[test]
    fn rejects_incomplete_moc3_id_section() {
        let mut bytes = moc3_with_ids();
        bytes.truncate(0x920);

        let error = Moc3Ids::parse(&bytes).unwrap_err();

        assert!(matches!(error, Error::InvalidMoc3 { .. }));
    }

    fn moc3_with_ids() -> Vec<u8> {
        let mut bytes = vec![0; 0xa00];
        bytes[0..4].copy_from_slice(b"MOC3");
        bytes[4] = 1;

        write_u32(&mut bytes, 0x40, 0x7c0);
        write_u32(&mut bytes, 0x44, 0x840);
        write_u32(&mut bytes, 0x4c, 0x880);
        write_u32(&mut bytes, 0xc4, 0x900);
        write_u32(&mut bytes, 0x108, 0x980);

        write_u32(&mut bytes, 0x7c0, 2);
        write_u32(&mut bytes, 0x7d0, 2);
        write_u32(&mut bytes, 0x7d4, 2);

        write_str64(&mut bytes, 0x880, "PartSegmentA");
        write_str64(&mut bytes, 0x8c0, "PartSegmentB");
        write_str64(&mut bytes, 0x900, "ArtMesh00");
        write_str64(&mut bytes, 0x940, "ArtMesh01");
        write_str64(&mut bytes, 0x980, "ParamAngleX");
        write_str64(&mut bytes, 0x9c0, "ParamInputA");

        bytes
    }

    fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_str64(bytes: &mut [u8], offset: usize, value: &str) {
        bytes[offset..offset + value.len()].copy_from_slice(value.as_bytes());
    }
}

mod moc3_offsets {
    use rusty_live2d::{Error, moc3::Moc3SectionOffsets};

    #[test]
    fn parses_confirmed_moc3_section_offsets() {
        let bytes = moc3_with_offsets(0x7c0, 0x840, 0x900);
        let offsets = Moc3SectionOffsets::parse(&bytes).unwrap();

        assert_eq!(offsets.count_info_offset(), 0x7c0);
        assert_eq!(offsets.canvas_info_offset(), 0x840);
    }

    #[test]
    fn parses_full_moc3_section_offset_table() {
        let mut bytes = moc3_with_offsets(0x7c0, 0x840, 0x900);
        bytes[0x48..0x4c].copy_from_slice(&0x880u32.to_le_bytes());
        bytes[0x2bc..0x2c0].copy_from_slice(&0u32.to_le_bytes());

        let offsets = Moc3SectionOffsets::parse(&bytes).unwrap();

        assert_eq!(offsets.section_offsets().len(), 160);
        assert_eq!(offsets.section_offset(0), Some(0x7c0));
        assert_eq!(offsets.section_offset(1), Some(0x840));
        assert_eq!(offsets.section_offset(2), Some(0x880));
        assert_eq!(offsets.section_offset(160), None);
    }

    #[test]
    fn rejects_moc3_without_section_offset_table() {
        let bytes = header_only();
        let error = Moc3SectionOffsets::parse(&bytes).unwrap_err();

        assert!(matches!(error, Error::InvalidMoc3 { .. }));
    }

    #[test]
    fn rejects_out_of_range_moc3_section_offsets() {
        let bytes = moc3_with_offsets(0x7c0, 0x940, 0x900);
        let error = Moc3SectionOffsets::parse(&bytes).unwrap_err();

        assert!(matches!(error, Error::InvalidMoc3 { .. }));
    }

    #[test]
    fn rejects_moc3_section_offsets_that_point_into_header_or_table() {
        let bytes = moc3_with_offsets(0, 0x840, 0x900);
        let error = Moc3SectionOffsets::parse(&bytes).unwrap_err();

        assert!(matches!(error, Error::InvalidMoc3 { .. }));
    }

    #[test]
    fn rejects_moc3_section_offsets_that_point_into_full_offset_table() {
        let mut bytes = moc3_with_offsets(0x7c0, 0x840, 0x900);
        bytes[0x48..0x4c].copy_from_slice(&0x100u32.to_le_bytes());

        let error = Moc3SectionOffsets::parse(&bytes).unwrap_err();

        assert!(matches!(error, Error::InvalidMoc3 { .. }));
    }

    fn header_only() -> [u8; 64] {
        let mut bytes = [0; 64];
        bytes[0..4].copy_from_slice(b"MOC3");
        bytes[4] = 1;
        bytes
    }

    fn moc3_with_offsets(count_info_offset: u32, canvas_info_offset: u32, len: usize) -> Vec<u8> {
        let mut bytes = vec![0; len];
        bytes[0..64].copy_from_slice(&header_only());
        bytes[0x40..0x44].copy_from_slice(&count_info_offset.to_le_bytes());
        bytes[0x44..0x48].copy_from_slice(&canvas_info_offset.to_le_bytes());
        bytes
    }
}
