use rusty_live2d::core::{
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
