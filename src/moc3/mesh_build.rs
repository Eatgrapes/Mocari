use crate::core::Vector2;

use super::{
    Moc3ArtMeshKeyformInfo, Moc3ArtMeshKeyforms, Moc3ArtMeshes, Moc3Deformers, Moc3DrawableMesh,
    Moc3DrawableVertex, Moc3Ids, Moc3KeyformBindings, Moc3OffscreenInfo, build_moc3_drawable_mesh,
    compose::ComposedDeformers, deformers::Moc3KeyformSlot,
};

pub fn build_moc3_drawable_meshes_for_default_pose(
    art_meshes: &Moc3ArtMeshes,
    art_mesh_keyforms: &Moc3ArtMeshKeyforms,
    deformers: &Moc3Deformers,
    bindings: &Moc3KeyformBindings,
) -> Option<Vec<Moc3DrawableMesh>> {
    let composed = deformers.compose(bindings)?;
    let mut meshes = Vec::with_capacity(art_meshes.meshes().len());
    for art_mesh_index in 0..art_meshes.meshes().len() {
        meshes.push(build_moc3_drawable_mesh_for_default_pose(
            art_meshes,
            art_mesh_keyforms,
            &composed,
            bindings,
            art_mesh_index,
        )?);
    }

    Some(meshes)
}

pub fn build_moc3_drawable_meshes_for_default_pose_with_offscreen_state(
    art_meshes: &Moc3ArtMeshes,
    art_mesh_keyforms: &Moc3ArtMeshKeyforms,
    deformers: &Moc3Deformers,
    bindings: &Moc3KeyformBindings,
    ids: &Moc3Ids,
    offscreen: &Moc3OffscreenInfo,
) -> Option<Vec<Moc3DrawableMesh>> {
    let mut meshes = build_moc3_drawable_meshes_for_default_pose(
        art_meshes,
        art_mesh_keyforms,
        deformers,
        bindings,
    )?;

    for drawable_index in offscreen.effect_source_drawable_indices(ids) {
        meshes.get_mut(drawable_index)?.set_opacity(0.0);
    }

    Some(meshes)
}

fn build_moc3_drawable_mesh_for_default_pose(
    art_meshes: &Moc3ArtMeshes,
    art_mesh_keyforms: &Moc3ArtMeshKeyforms,
    composed: &ComposedDeformers,
    bindings: &Moc3KeyformBindings,
    art_mesh_index: usize,
) -> Option<Moc3DrawableMesh> {
    let keyform_count = art_mesh_keyforms.art_mesh_keyforms(art_mesh_index)?.len();
    let slots = bindings.default_keyform_slots(
        art_meshes.art_mesh_keyform_binding_band_index(art_mesh_index)?,
        keyform_count,
    )?;
    let base_local_keyform_index = slots.first()?.local_index;
    let mesh = build_moc3_drawable_mesh(
        art_meshes,
        art_mesh_keyforms,
        art_mesh_index,
        base_local_keyform_index,
    )?;
    let opacity = interpolate_art_mesh_opacity(art_mesh_keyforms, art_mesh_index, &slots)?;
    let draw_order = interpolate_art_mesh_draw_order(art_mesh_keyforms, art_mesh_index, &slots)?;
    let mut positions = interpolate_art_mesh_positions(art_mesh_keyforms, art_mesh_index, &slots)?;

    composed.transform_vertices(
        art_meshes.art_mesh_parent_deformer_index(art_mesh_index)?,
        &mut positions,
    )?;

    let vertices = mesh
        .vertices()
        .iter()
        .zip(positions)
        .map(|(vertex, position)| {
            Moc3DrawableVertex::new([position.x(), -position.y()], vertex.uv())
        })
        .collect();

    Some(Moc3DrawableMesh::from_parts_with_render_order(
        mesh.texture_index(),
        mesh.drawable_flags(),
        opacity,
        draw_order,
        mesh.render_order(),
        vertices,
        mesh.indices().to_vec(),
        mesh.masks().to_vec(),
    ))
}

fn interpolate_art_mesh_positions(
    keyforms: &Moc3ArtMeshKeyforms,
    art_mesh_index: usize,
    slots: &[Moc3KeyformSlot],
) -> Option<Vec<Vector2>> {
    let first = keyforms.art_mesh_keyform_positions(art_mesh_index, slots.first()?.local_index)?;
    let mut out = vec![Vector2::default(); first.len().checked_div(2)?];

    for slot in slots {
        let positions = keyforms.art_mesh_keyform_positions(art_mesh_index, slot.local_index)?;
        if positions.len() != first.len() || positions.len() % 2 != 0 {
            return None;
        }
        for (target, position) in out.iter_mut().zip(positions.chunks_exact(2)) {
            *target = Vector2::new(
                target.x() + position[0] * slot.weight,
                target.y() + position[1] * slot.weight,
            );
        }
    }

    Some(out)
}

fn interpolate_art_mesh_opacity(
    keyforms: &Moc3ArtMeshKeyforms,
    art_mesh_index: usize,
    slots: &[Moc3KeyformSlot],
) -> Option<f32> {
    interpolate_art_mesh_scalar(keyforms, art_mesh_index, slots, |keyform| keyform.opacity())
}

fn interpolate_art_mesh_draw_order(
    keyforms: &Moc3ArtMeshKeyforms,
    art_mesh_index: usize,
    slots: &[Moc3KeyformSlot],
) -> Option<f32> {
    interpolate_art_mesh_scalar(keyforms, art_mesh_index, slots, |keyform| {
        keyform.draw_order()
    })
}

fn interpolate_art_mesh_scalar(
    keyforms: &Moc3ArtMeshKeyforms,
    art_mesh_index: usize,
    slots: &[Moc3KeyformSlot],
    value: impl Fn(Moc3ArtMeshKeyformInfo) -> f32,
) -> Option<f32> {
    let keyforms = keyforms.art_mesh_keyforms(art_mesh_index)?;
    let mut out = 0.0f32;
    for slot in slots {
        out += value(*keyforms.get(slot.local_index)?) * slot.weight;
    }
    Some(out)
}
