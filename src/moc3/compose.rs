use crate::core::{
    Vector2, WarpInterpolation, rotation_deformer_transform_point, warp_deformer_transform_target,
};

pub(super) const ROTATION_DERIVATIVE_STEP: f32 = 0.1;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum ComposedDeformer {
    Warp(ComposedWarp),
    Rotation(ComposedRotation),
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ComposedWarp {
    pub(super) grid: Vec<Vector2>,
    pub(super) cols: usize,
    pub(super) rows: usize,
    pub(super) scale_accum: f32,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(super) struct ComposedRotation {
    pub(super) origin: Vector2,
    pub(super) angle_degrees: f32,
    pub(super) scale: f32,
    pub(super) flip_x: bool,
    pub(super) flip_y: bool,
    pub(super) scale_accum: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComposedDeformers {
    deformers: Vec<ComposedDeformer>,
}

impl ComposedDeformers {
    pub(super) fn new(deformers: Vec<ComposedDeformer>) -> Self {
        Self { deformers }
    }

    pub(super) fn transform_vertices(
        &self,
        parent_deformer_index: i32,
        vertices: &mut [Vector2],
    ) -> Option<()> {
        if parent_deformer_index < 0 {
            return Some(());
        }
        let index = usize::try_from(parent_deformer_index).ok()?;
        for vertex in vertices {
            *vertex = apply_one(self.deformers.get(index)?, *vertex)?;
        }
        Some(())
    }
}

pub(super) fn apply_one(deformer: &ComposedDeformer, point: Vector2) -> Option<Vector2> {
    match deformer {
        ComposedDeformer::Warp(warp) => warp_deformer_transform_target(
            point,
            &warp.grid,
            warp.cols,
            warp.rows,
            WarpInterpolation::Quad,
        ),
        ComposedDeformer::Rotation(rotation) => Some(rotation_deformer_transform_point(
            point,
            rotation.angle_degrees,
            rotation.scale,
            rotation.origin,
            rotation.flip_x,
            rotation.flip_y,
        )),
    }
}

pub(super) fn apply_composed_parent(
    composed: &[Option<ComposedDeformer>],
    parent_index: i32,
    point: Vector2,
) -> Option<Vector2> {
    if parent_index < 0 {
        return Some(point);
    }
    let index = usize::try_from(parent_index).ok()?;
    let parent = composed.get(index)?.as_ref()?;
    apply_one(parent, point)
}

pub(super) fn parent_scale_accum(composed: &[Option<ComposedDeformer>], parent_index: i32) -> f32 {
    if parent_index < 0 {
        return 1.0;
    }
    let index = match usize::try_from(parent_index) {
        Ok(value) => value,
        Err(_) => return 1.0,
    };
    match composed.get(index).and_then(|slot| slot.as_ref()) {
        Some(ComposedDeformer::Warp(warp)) => warp.scale_accum,
        Some(ComposedDeformer::Rotation(rotation)) => rotation.scale_accum,
        None => 1.0,
    }
}
