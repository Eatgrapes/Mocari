mod ids {
    use mocari::{DrawableId, Error, Id, ParameterId, PartId};

    #[test]
    fn id_rejects_empty_strings() {
        let error = Id::new("").unwrap_err();
        assert!(matches!(error, Error::EmptyId));
    }

    #[test]
    fn id_trims_nothing_and_preserves_source_text() {
        let id = Id::new(" ParamAngleX ").unwrap();
        assert_eq!(id.as_str(), " ParamAngleX ");
        assert_eq!(id.to_string(), " ParamAngleX ");
    }

    #[test]
    fn typed_ids_expose_their_inner_text() {
        let parameter = ParameterId::new("ParamAngleX").unwrap();
        let part = PartId::new("PartSegmentA").unwrap();
        let drawable = DrawableId::new("DrawableBody").unwrap();

        assert_eq!(parameter.as_str(), "ParamAngleX");
        assert_eq!(part.as_str(), "PartSegmentA");
        assert_eq!(drawable.as_str(), "DrawableBody");
    }
}

mod core_parameters {
    use mocari::core::{clamp_parameter_value, core_repeat_fold, parameter_dirty};

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.00001,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn clamps_parameter_values_to_min_max() {
        assert_eq!(clamp_parameter_value(-2.0, -1.0, 1.0), -1.0);
        assert_eq!(clamp_parameter_value(2.0, -1.0, 1.0), 1.0);
        assert_eq!(clamp_parameter_value(0.25, -1.0, 1.0), 0.25);
    }

    #[test]
    fn repeat_fold_uses_core_floor_like_correction() {
        assert_close(core_repeat_fold(370.0, 0.0, 360.0), 10.0);
        assert_close(core_repeat_fold(-10.0, 0.0, 360.0), 350.0);
        assert_close(core_repeat_fold(-360.0, 0.0, 360.0), 0.0);
    }

    #[test]
    fn repeat_fold_keeps_large_q_path_truncated() {
        let value = 8_388_609.25_f32;

        assert_eq!(core_repeat_fold(value, 0.0, 1.0), 0.0);
    }

    #[test]
    fn dirty_flag_treats_nan_as_dirty() {
        assert!(!parameter_dirty(1.0, 1.0));
        assert!(parameter_dirty(1.0, 2.0));
        assert!(parameter_dirty(f32::NAN, 1.0));
        assert!(parameter_dirty(1.0, f32::NAN));
    }
}

mod math {
    use std::collections::BTreeMap;

    use mocari::core::{Matrix44, ModelMatrix};

    #[test]
    fn matrix44_transforms_scaled_and_translated_points() {
        let mut matrix = Matrix44::identity();
        matrix.scale(2.0, 3.0);
        matrix.translate(10.0, -4.0);

        assert_eq!(matrix.transform_x(5.0), 20.0);
        assert_eq!(matrix.transform_y(2.0), 2.0);
        assert_eq!(matrix.invert_transform_x(20.0), 5.0);
        assert_eq!(matrix.invert_transform_y(2.0), 2.0);
    }

    #[test]
    fn model_matrix_initializes_to_height_two() {
        let matrix = ModelMatrix::new(4.0, 8.0);

        assert_eq!(matrix.transform_x(4.0), 1.0);
        assert_eq!(matrix.transform_y(8.0), 2.0);
    }

    #[test]
    fn model_matrix_applies_layout_size_before_position() {
        let mut matrix = ModelMatrix::new(4.0, 8.0);
        let mut layout = BTreeMap::new();
        layout.insert("width".to_string(), 4.0);
        layout.insert("center_x".to_string(), 0.0);
        layout.insert("top".to_string(), 1.0);

        matrix.setup_from_layout(&layout);

        assert_eq!(matrix.transform_x(0.0), -2.0);
        assert_eq!(matrix.transform_x(4.0), 2.0);
        assert_eq!(matrix.transform_y(0.0), 1.0);
        assert_eq!(matrix.transform_y(8.0), 9.0);
    }
}

mod core_blend {
    use mocari::core::{
        BlendSlot, Rgb, blend_scalar_slots, blend_scalar_slots_clamped, multiply_rgb, screen_rgb,
    };

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.00001,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn blends_scalar_slots_with_single_and_pair_kinds() {
        let values = [10.0, 20.0, 30.0, 40.0];
        let slots = [
            BlendSlot::Skip,
            BlendSlot::Single {
                base: 0,
                index: 1,
                weight: 0.5,
                final_weight: 0.25,
            },
            BlendSlot::Pair {
                base: 0,
                index0: 2,
                weight0: 0.75,
                index1: 3,
                weight1: 0.25,
                final_weight: 0.5,
            },
        ];

        assert_close(blend_scalar_slots(&slots, &values, 1.0).unwrap(), 19.75);
    }

    #[test]
    fn clamps_scalar_blend_result() {
        let slots = [BlendSlot::Single {
            base: 0,
            index: 0,
            weight: 2.0,
            final_weight: 1.0,
        }];

        assert_eq!(
            blend_scalar_slots_clamped(&slots, &[0.75], 0.0, 0.0, 1.0).unwrap(),
            1.0
        );
    }

    #[test]
    fn blends_multiply_and_screen_rgb() {
        let local = Rgb::new(0.25, 0.5, 1.25);
        let parent = Rgb::new(0.5, 0.25, 0.5);

        assert_eq!(multiply_rgb(local, parent), Rgb::new(0.125, 0.125, 0.625));
        assert_eq!(screen_rgb(local, parent), Rgb::new(0.625, 0.625, 1.0));
    }
}

mod core_keyforms {
    use mocari::core::{
        KeyformAxis, compute_keyform_axis_interval, expand_keyform_runtime_slots,
    };

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.00001,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn computes_keyform_axis_interval() {
        let keys = [0.0, 10.0, 20.0];

        let below = compute_keyform_axis_interval(&keys, -1.0).unwrap();
        assert_eq!(below.left_index(), 0);
        assert_eq!(below.t(), 0.0);

        let middle = compute_keyform_axis_interval(&keys, 12.5).unwrap();
        assert_eq!(middle.left_index(), 1);
        assert_close(middle.t(), 0.25);

        let above = compute_keyform_axis_interval(&keys, 25.0).unwrap();
        assert_eq!(above.left_index(), 2);
        assert_eq!(above.t(), 0.0);

        assert!(compute_keyform_axis_interval(&[], 1.0).is_none());
    }

    #[test]
    fn expands_runtime_slots_from_active_axes() {
        let axes = [
            KeyformAxis::new(1, 0.25, 1),
            KeyformAxis::new(2, 0.0, 4),
            KeyformAxis::new(0, 0.5, 12),
        ];

        let slots = expand_keyform_runtime_slots(&axes);

        assert_eq!(slots.len(), 4);
        assert_eq!(slots[0].flat_index(), 9);
        assert_close(slots[0].weight(), 0.375);
        assert_eq!(slots[1].flat_index(), 10);
        assert_close(slots[1].weight(), 0.125);
        assert_eq!(slots[2].flat_index(), 21);
        assert_close(slots[2].weight(), 0.375);
        assert_eq!(slots[3].flat_index(), 22);
        assert_close(slots[3].weight(), 0.125);
    }
}

mod core_update_order {
    use mocari::core::{
        ModelUpdateStep, semantic_model_update_order, should_affect_glues, should_blend_glues,
        should_run_offscreen_stage,
    };

    #[test]
    fn exposes_confirmed_semantic_model_update_order() {
        let order = semantic_model_update_order();

        assert_eq!(order.first(), Some(&ModelUpdateStep::PreUpdateDynamicFlags));
        assert_eq!(
            order.windows(4).position(|steps| {
                steps
                    == [
                        ModelUpdateStep::UpdateParameters,
                        ModelUpdateStep::UpdateParameterBindings,
                        ModelUpdateStep::UpdateBlendShapeParameterBindings,
                        ModelUpdateStep::UpdateKeyformBindings,
                    ]
            }),
            Some(1)
        );
        assert!(
            order
                .iter()
                .position(|step| *step == ModelUpdateStep::BlendArtMeshes)
                .unwrap()
                < order
                    .iter()
                    .position(|step| *step == ModelUpdateStep::TransformDeformers)
                    .unwrap()
        );
        assert_eq!(order.last(), Some(&ModelUpdateStep::PostUpdateDynamicFlags));
    }

    #[test]
    fn applies_confirmed_glue_and_offscreen_guards() {
        assert!(!should_affect_glues(0));
        assert!(should_affect_glues(1));
        assert!(!should_blend_glues(4));
        assert!(should_blend_glues(5));
        assert!(!should_run_offscreen_stage(5));
        assert!(should_run_offscreen_stage(6));
    }
}

mod core_art_mesh {
    use mocari::core::{
        Vector2, affect_art_mesh_pair, apply_art_mesh_blend_shape_delta, apply_parent_part_opacity,
        draw_order_from_raw, reverse_coordinate_y,
    };

    fn assert_vec_close(actual: Vector2, expected: Vector2) {
        assert!(
            (actual.x() - expected.x()).abs() < 0.00001,
            "expected x {}, got {}",
            expected.x(),
            actual.x()
        );
        assert!(
            (actual.y() - expected.y()).abs() < 0.00001,
            "expected y {}, got {}",
            expected.y(),
            actual.y()
        );
    }

    #[test]
    fn glue_affects_art_mesh_pair_with_directional_weights() {
        let (a, b) = affect_art_mesh_pair(
            Vector2::new(0.0, 0.0),
            Vector2::new(10.0, 20.0),
            0.25,
            0.5,
            0.8,
        );

        assert_vec_close(a, Vector2::new(2.0, 4.0));
        assert_vec_close(b, Vector2::new(6.0, 12.0));
    }

    #[test]
    fn reverse_coordinate_flips_y_only() {
        let mut vertices = [Vector2::new(1.0, 2.0), Vector2::new(-3.0, -4.0)];

        reverse_coordinate_y(&mut vertices);

        assert_eq!(vertices, [Vector2::new(1.0, -2.0), Vector2::new(-3.0, 4.0)]);
    }

    #[test]
    fn draw_order_uses_core_int_bias_and_clamps() {
        assert_eq!(draw_order_from_raw(12.999), 13);
        assert_eq!(draw_order_from_raw(-1.0), 0);
        assert_eq!(draw_order_from_raw(1001.0), 1000);
    }

    #[test]
    fn applies_art_mesh_blend_shape_delta() {
        let mut positions = [0.0, 1.0, 2.0, 3.0];

        apply_art_mesh_blend_shape_delta(&mut positions, &[10.0, -10.0, 4.0, -4.0], 0.25).unwrap();
        apply_art_mesh_blend_shape_delta(&mut positions, &[1.0, 1.0, 1.0, 1.0], 0.0).unwrap();

        assert_eq!(positions, [2.5, -1.5, 3.0, 2.0]);
        assert!(apply_art_mesh_blend_shape_delta(&mut positions, &[1.0], 1.0).is_none());
    }

    #[test]
    fn parent_part_opacity_multiplies_art_mesh_opacity() {
        assert_eq!(apply_parent_part_opacity(0.75, 0.5), 0.375);
    }
}
