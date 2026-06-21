use mocari::moc3::{Moc3DrawableMesh, Moc3DrawableVertex};
use mocari::render::common::{
    ClippingLayoutError, ClippingPlan, ClippingRect, DrawableInfo, MaskChannel, draw_order_indices,
};

fn infos(meshes: &[Moc3DrawableMesh]) -> Vec<DrawableInfo> {
    meshes.iter().map(DrawableInfo::from_mesh).collect()
}

#[test]
fn draw_order_indices_sort_by_quantized_order_then_render_rank() {
    let meshes = [
        test_mesh_with_draw_order(0, 30.0),
        test_mesh_with_draw_order(1, 10.0),
        test_mesh_with_draw_order(2, 10.0),
    ];

    assert_eq!(draw_order_indices(&infos(&meshes)), vec![1, 2, 0]);
}

#[test]
fn draw_order_indices_use_render_order_rank_to_break_ties() {
    let meshes = [
        test_mesh_with_render_order(0, 650.0, 70),
        test_mesh_with_render_order(1, 650.0, 49),
        test_mesh_with_render_order(2, 600.0, 90),
    ];

    assert_eq!(draw_order_indices(&infos(&meshes)), vec![2, 1, 0]);
}

#[test]
fn draw_order_indices_quantize_to_avoid_flicker() {
    let jittered = [
        test_mesh_with_render_order(0, 499.9998, 70),
        test_mesh_with_render_order(1, 500.0001, 49),
    ];
    let exact = [
        test_mesh_with_render_order(0, 500.0, 70),
        test_mesh_with_render_order(1, 500.0, 49),
    ];

    assert_eq!(draw_order_indices(&infos(&jittered)), vec![1, 0]);
    assert_eq!(
        draw_order_indices(&infos(&jittered)),
        draw_order_indices(&infos(&exact))
    );
}

#[test]
fn builds_clipping_plan_from_masked_drawables() {
    let meshes = [
        test_mesh_with_masks(0, 0.0, vec![1, 2]),
        test_mesh_with_draw_order(0, 1.0),
        test_mesh_with_masks(0, 2.0, vec![1, 2]),
        test_mesh_with_masks(0, 3.0, vec![3]),
    ];

    let plan = ClippingPlan::from_drawables(&infos(&meshes));

    assert_eq!(plan.unmasked_drawable_indices(), &[1]);
    assert_eq!(plan.contexts().len(), 2);
    assert_eq!(plan.contexts()[0].masks(), &[1, 2]);
    assert_eq!(plan.contexts()[0].drawable_indices(), &[0, 2]);
    assert_eq!(plan.contexts()[1].masks(), &[3]);
    assert_eq!(plan.contexts()[1].drawable_indices(), &[3]);
}

#[test]
fn merges_clipping_contexts_with_same_mask_set_regardless_of_order() {
    let meshes = [
        test_mesh_with_masks(0, 0.0, vec![1, 2]),
        test_mesh_with_masks(0, 1.0, vec![2, 1]),
    ];

    let plan = ClippingPlan::from_drawables(&infos(&meshes));

    assert_eq!(plan.contexts().len(), 1);
    assert_eq!(plan.contexts()[0].masks(), &[1, 2]);
    assert_eq!(plan.contexts()[0].drawable_indices(), &[0, 1]);
}

#[test]
fn splits_clipping_contexts_when_inverted_flag_differs() {
    let meshes = [
        test_mesh(0, 0, 0.0, vec![1, 2]),
        test_mesh(0, 1 << 3, 1.0, vec![1, 2]),
    ];

    let plan = ClippingPlan::from_drawables(&infos(&meshes));

    assert_eq!(plan.contexts().len(), 2);
    assert!(!plan.contexts()[0].inverted());
    assert_eq!(plan.contexts()[0].drawable_indices(), &[0]);
    assert!(plan.contexts()[1].inverted());
    assert_eq!(plan.contexts()[1].drawable_indices(), &[1]);
}

#[test]
fn assigns_single_texture_clipping_layouts_by_channel_and_cell() {
    let meshes = [
        test_mesh_with_masks(0, 0.0, vec![10]),
        test_mesh_with_masks(0, 1.0, vec![11]),
        test_mesh_with_masks(0, 2.0, vec![12]),
        test_mesh_with_masks(0, 3.0, vec![13]),
        test_mesh_with_masks(0, 4.0, vec![14]),
    ];
    let mut plan = ClippingPlan::from_drawables(&infos(&meshes));

    plan.assign_single_texture_layouts().unwrap();

    assert_eq!(
        plan.contexts()[0].layout().unwrap().channel(),
        MaskChannel::Red
    );
    assert_eq!(
        plan.contexts()[0].layout().unwrap().bounds(),
        ClippingRect::new(0.0, 0.0, 0.5, 1.0)
    );
    assert_eq!(
        plan.contexts()[1].layout().unwrap().channel(),
        MaskChannel::Red
    );
    assert_eq!(
        plan.contexts()[1].layout().unwrap().bounds(),
        ClippingRect::new(0.5, 0.0, 0.5, 1.0)
    );
    assert_eq!(
        plan.contexts()[2].layout().unwrap().channel(),
        MaskChannel::Green
    );
    assert_eq!(
        plan.contexts()[2].layout().unwrap().bounds(),
        ClippingRect::new(0.0, 0.0, 1.0, 1.0)
    );
    assert_eq!(
        plan.contexts()[4].layout().unwrap().channel_flag(),
        [0.0, 0.0, 0.0, 1.0]
    );
}

#[test]
fn rejects_more_than_single_texture_clipping_layout_capacity() {
    let meshes = (0..37)
        .map(|index| test_mesh_with_masks(0, index as f32, vec![index]))
        .collect::<Vec<_>>();
    let mut plan = ClippingPlan::from_drawables(&infos(&meshes));

    let error = plan.assign_single_texture_layouts().unwrap_err();

    assert_eq!(
        error,
        ClippingLayoutError::TooManyMasksForSingleTexture { mask_count: 37 }
    );
}

#[test]
fn prepares_clipping_bounds_and_matrices_from_clipped_drawables() {
    let clipped = Moc3DrawableMesh::from_parts(
        0,
        0,
        1.0,
        0.0,
        vec![
            Moc3DrawableVertex::new([-1.0, -2.0], [0.0, 0.0]),
            Moc3DrawableVertex::new([3.0, -2.0], [1.0, 0.0]),
            Moc3DrawableVertex::new([3.0, 4.0], [1.0, 1.0]),
        ],
        vec![0, 1, 2],
        vec![1],
    );
    let mask = test_mesh_with_draw_order(0, 1.0);
    let drawables = infos(&[clipped, mask]);
    let mut plan = ClippingPlan::from_drawables(&drawables);

    plan.prepare_single_texture_masks(&drawables).unwrap();

    let context = &plan.contexts()[0];
    assert_rect_close(
        context.all_clipped_draw_rect().unwrap(),
        ClippingRect::new(-1.2, -2.3, 4.4, 6.6),
    );

    let draw_matrix = context.matrix_for_draw().unwrap();
    assert_f32_close(draw_matrix.transform_x(-1.2), 0.0);
    assert_f32_close(draw_matrix.transform_x(3.2), 1.0);
    assert_f32_close(draw_matrix.transform_y(-2.3), 1.0);
    assert_f32_close(draw_matrix.transform_y(4.3), 0.0);

    let mask_matrix = context.matrix_for_mask().unwrap();
    assert_f32_close(mask_matrix.transform_x(-1.2), -1.0);
    assert_f32_close(mask_matrix.transform_x(3.2), 1.0);
    assert_f32_close(mask_matrix.transform_y(-2.3), -1.0);
    assert_f32_close(mask_matrix.transform_y(4.3), 1.0);
}

fn test_mesh_with_draw_order(texture_index: u8, draw_order: f32) -> Moc3DrawableMesh {
    test_mesh(texture_index, 0, draw_order, vec![])
}

fn test_mesh_with_masks(texture_index: u8, draw_order: f32, masks: Vec<i32>) -> Moc3DrawableMesh {
    test_mesh(texture_index, 0, draw_order, masks)
}

fn test_mesh_with_render_order(
    texture_index: u8,
    draw_order: f32,
    render_order: i32,
) -> Moc3DrawableMesh {
    Moc3DrawableMesh::from_parts_with_render_order(
        i32::from(texture_index),
        0,
        1.0,
        draw_order,
        render_order,
        vec![
            Moc3DrawableVertex::new([-0.5, -0.5], [0.0, 0.0]),
            Moc3DrawableVertex::new([0.5, -0.5], [1.0, 0.0]),
            Moc3DrawableVertex::new([0.0, 0.5], [0.5, 1.0]),
        ],
        vec![0, 1, 2],
        Vec::new(),
    )
}

fn test_mesh(
    texture_index: u8,
    drawable_flags: u8,
    draw_order: f32,
    masks: Vec<i32>,
) -> Moc3DrawableMesh {
    Moc3DrawableMesh::from_parts(
        i32::from(texture_index),
        drawable_flags,
        1.0,
        draw_order,
        vec![
            Moc3DrawableVertex::new([-0.5, -0.5], [0.0, 1.0]),
            Moc3DrawableVertex::new([0.5, -0.5], [1.0, 1.0]),
            Moc3DrawableVertex::new([0.0, 0.5], [0.5, 0.0]),
        ],
        vec![0, 1, 2],
        masks,
    )
}

fn assert_rect_close(actual: ClippingRect, expected: ClippingRect) {
    assert_f32_close(actual.x(), expected.x());
    assert_f32_close(actual.y(), expected.y());
    assert_f32_close(actual.width(), expected.width());
    assert_f32_close(actual.height(), expected.height());
}

fn assert_f32_close(actual: f32, expected: f32) {
    let difference = (actual - expected).abs();
    assert!(
        difference <= 0.00001,
        "expected {actual} to be within 0.00001 of {expected}, difference {difference}"
    );
}
