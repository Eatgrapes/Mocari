//! Mutable state for driving a loaded model.
//!
//! [`ModelRuntime`] is the type most applications update every frame. It stores
//! parameter values, part opacity overrides, pose state, and the current drawable
//! meshes. After changing parameters or applying a player from [`crate::motion`]
//! or [`crate::expression`], call [`ModelRuntime::update_meshes`] before drawing.

use std::collections::HashMap;

use crate::{
    core::{PhysicsOptions, PhysicsRuntime, clamp_parameter_value, draw_order_from_raw},
    json::{Model3, Physics3, Pose3, copy_pose_link_opacities, update_pose_group_opacities},
    moc3::{
        Moc3ArtMeshKeyforms, Moc3ArtMeshes, Moc3CanvasInfo, Moc3Deformers, Moc3DrawOrderGroups,
        Moc3DrawableMesh, Moc3Glues, Moc3Ids, Moc3KeyformBindings, Moc3MeshUpdateScratch,
        Moc3OffscreenInfo, Moc3Parts,
        build_moc3_drawable_meshes_with_parameters_offscreen_and_part_opacities,
        update_moc3_drawable_meshes_with_parameters_offscreen_and_part_opacities,
    },
};

#[derive(Debug, Clone)]
struct PoseGroup {
    members: Vec<usize>,
    links: Vec<Vec<usize>>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
/// A read-only view of one model parameter.
///
/// Values are reported in the model's native parameter range. Use
/// [`normalized_value`](Self::normalized_value) when UI code wants a stable
/// `0.0..=1.0` representation.
pub struct ParameterInfo<'a> {
    id: &'a str,
    minimum: f32,
    maximum: f32,
    default: f32,
    value: f32,
}

impl<'a> ParameterInfo<'a> {
    /// Returns the Cubism parameter id, such as `ParamAngleX`.
    pub fn id(&self) -> &'a str {
        self.id
    }

    /// Returns the minimum value declared by the model.
    pub fn minimum(&self) -> f32 {
        self.minimum
    }

    /// Returns the maximum value declared by the model.
    pub fn maximum(&self) -> f32 {
        self.maximum
    }

    /// Returns the default value declared by the model.
    pub fn default(&self) -> f32 {
        self.default
    }

    /// Returns the current runtime value.
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Returns the current value mapped into `0.0..=1.0`.
    ///
    /// If the model declares an invalid range where `maximum <= minimum`, this
    /// returns `0.0`.
    pub fn normalized_value(&self) -> f32 {
        normalized_parameter_value(self.value, self.minimum, self.maximum)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// A hit area that contains a tested model-space point.
///
/// Hit areas come from `HitAreas` in `.model3.json`. Their ids reference
/// drawable ids in the loaded `.moc3` file, and their names are the user-facing
/// labels commonly used to choose tap motions such as `TapBody`.
pub struct HitAreaInfo<'a> {
    id: &'a str,
    name: &'a str,
    drawable_index: usize,
}

impl<'a> HitAreaInfo<'a> {
    /// Returns the hit area's drawable id.
    pub fn id(&self) -> &'a str {
        self.id
    }

    /// Returns the user-facing hit area name, such as `Head` or `Body`.
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// Returns the model-order drawable index used for hit testing.
    pub fn drawable_index(&self) -> usize {
        self.drawable_index
    }
}

#[derive(Debug, Clone)]
/// Runtime state for a loaded Live2D/Cubism-compatible model.
///
/// The runtime owns the current parameter values and the generated drawable
/// meshes. A typical frame updates parameters, applies motion and expression
/// players, applies pose fading if needed, then calls [`update_meshes`](Self::update_meshes).
///
/// Applications usually create this through [`crate::assets::load_model_runtime`]
/// instead of calling [`new`](Self::new) directly.
pub struct ModelRuntime {
    model: Model3,
    canvas: Moc3CanvasInfo,
    art_meshes: Moc3ArtMeshes,
    art_mesh_keyforms: Moc3ArtMeshKeyforms,
    deformers: Moc3Deformers,
    bindings: Moc3KeyformBindings,
    ids: Moc3Ids,
    offscreen: Moc3OffscreenInfo,
    glues: Moc3Glues,
    parts: Moc3Parts,
    draw_order_groups: Option<Moc3DrawOrderGroups>,
    drawable_index: HashMap<String, usize>,
    parameter_index: HashMap<String, usize>,
    parameter_values: Vec<f32>,
    parameter_overrides: Vec<Option<f32>>,
    physics: Option<PhysicsRuntime>,
    part_index: HashMap<String, usize>,
    part_opacity_overrides: Vec<Option<f32>>,
    part_opacities: Vec<f32>,
    pose_groups: Vec<PoseGroup>,
    pose_fade_time: f32,
    pose_opacities: Vec<f32>,
    meshes: Vec<Moc3DrawableMesh>,
    mesh_update_scratch: Moc3MeshUpdateScratch,
}

impl ModelRuntime {
    /// Builds a runtime from already parsed model components.
    ///
    /// This constructor is intended for custom loaders and tests. It returns
    /// `None` when the parsed parts cannot produce a valid initial mesh set.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        model: Model3,
        canvas: Moc3CanvasInfo,
        art_meshes: Moc3ArtMeshes,
        art_mesh_keyforms: Moc3ArtMeshKeyforms,
        deformers: Moc3Deformers,
        bindings: Moc3KeyformBindings,
        ids: Moc3Ids,
        offscreen: Moc3OffscreenInfo,
        glues: Moc3Glues,
        parts: Moc3Parts,
        draw_order_groups: Option<Moc3DrawOrderGroups>,
        pose: Option<Pose3>,
    ) -> Option<Self> {
        let parameter_values = bindings.parameter_default_values().to_vec();
        let parameter_overrides = vec![None; parameter_values.len()];
        let drawable_index = build_index(ids.art_meshes());
        let parameter_index = build_index(ids.parameters());
        let part_index = build_index(ids.parts());
        let part_count = parts.part_count();

        let pose_fade_time = pose
            .as_ref()
            .map(Pose3::resolved_fade_in_time)
            .unwrap_or_default();
        let pose_groups = pose
            .as_ref()
            .map(|pose| build_pose_groups(pose, &part_index))
            .unwrap_or_default();
        let pose_opacities = initial_pose_opacities(&pose_groups, part_count);

        let mut runtime = Self {
            model,
            canvas,
            art_meshes,
            art_mesh_keyforms,
            deformers,
            bindings,
            ids,
            offscreen,
            glues,
            parts,
            draw_order_groups,
            drawable_index,
            parameter_index,
            parameter_values,
            parameter_overrides,
            physics: None,
            part_index,
            part_opacity_overrides: vec![None; part_count],
            part_opacities: vec![1.0; part_count],
            pose_groups,
            pose_fade_time,
            pose_opacities,
            meshes: Vec::new(),
            mesh_update_scratch: Moc3MeshUpdateScratch::default(),
        };
        runtime.update_meshes()?;
        Some(runtime)
    }

    /// Returns the parsed `.model3.json` data associated with this runtime.
    pub fn model(&self) -> &Model3 {
        &self.model
    }

    /// Returns the model canvas information parsed from the `.moc3` file.
    pub fn canvas(&self) -> Moc3CanvasInfo {
        self.canvas
    }

    /// Returns all parameter ids in model order.
    pub fn parameter_ids(&self) -> &[String] {
        self.ids.parameters()
    }

    /// Returns all drawable ids in model order.
    pub fn drawable_ids(&self) -> &[String] {
        self.ids.art_meshes()
    }

    /// Returns the model-order drawable index for a drawable id.
    pub fn drawable_index(&self, id: &str) -> Option<usize> {
        self.drawable_index.get(id).copied()
    }

    /// Returns the model-order index for a parameter id.
    ///
    /// Cache this index in hot paths and use the `*_by_index` methods to avoid a
    /// string lookup each frame.
    pub fn parameter_index(&self, id: &str) -> Option<usize> {
        self.parameter_index.get(id).copied()
    }

    /// Returns the current value for a parameter id.
    pub fn parameter_value(&self, id: &str) -> Option<f32> {
        let index = self.parameter_index(id)?;
        self.parameter_values.get(index).copied()
    }

    /// Returns the current value for a parameter index.
    pub fn parameter_value_by_index(&self, index: usize) -> Option<f32> {
        self.parameter_values.get(index).copied()
    }

    /// Returns all current parameter values in model order.
    pub fn parameter_values(&self) -> &[f32] {
        &self.parameter_values
    }

    /// Returns metadata and the current value for a parameter id.
    pub fn parameter_info(&self, id: &str) -> Option<ParameterInfo<'_>> {
        let index = self.parameter_index(id)?;
        self.parameter_info_by_index(index)
    }

    /// Returns metadata and the current value for a parameter index.
    pub fn parameter_info_by_index(&self, index: usize) -> Option<ParameterInfo<'_>> {
        let (minimum, maximum) = self.parameter_range_by_index(index)?;
        Some(ParameterInfo {
            id: self.ids.parameters().get(index)?.as_str(),
            minimum,
            maximum,
            default: self.parameter_default_by_index(index)?,
            value: self.parameter_value_by_index(index)?,
        })
    }

    /// Iterates over all parameters with their ranges and current values.
    pub fn parameter_infos(&self) -> impl Iterator<Item = ParameterInfo<'_>> + '_ {
        (0..self.ids.parameters().len()).filter_map(|index| self.parameter_info_by_index(index))
    }

    /// Returns the declared minimum value for a parameter index.
    pub fn parameter_minimum_by_index(&self, index: usize) -> Option<f32> {
        self.bindings.parameter_min_values().get(index).copied()
    }

    /// Returns the declared maximum value for a parameter index.
    pub fn parameter_maximum_by_index(&self, index: usize) -> Option<f32> {
        self.bindings.parameter_max_values().get(index).copied()
    }

    /// Returns the declared default value for a parameter index.
    pub fn parameter_default_by_index(&self, index: usize) -> Option<f32> {
        self.bindings.parameter_default_values().get(index).copied()
    }

    /// Returns the current parameter value mapped into `0.0..=1.0`.
    pub fn parameter_normalized_value(&self, id: &str) -> Option<f32> {
        let index = self.parameter_index(id)?;
        self.parameter_normalized_value_by_index(index)
    }

    /// Returns the current parameter value for an index mapped into `0.0..=1.0`.
    pub fn parameter_normalized_value_by_index(&self, index: usize) -> Option<f32> {
        let minimum = self.parameter_minimum_by_index(index)?;
        let maximum = self.parameter_maximum_by_index(index)?;
        let value = self.parameter_value_by_index(index)?;
        Some(normalized_parameter_value(value, minimum, maximum))
    }

    /// Sets a parameter by id, clamping the value to the model's declared range.
    ///
    /// Returns `false` when the id is not present in the model.
    pub fn set_parameter(&mut self, id: &str, value: f32) -> bool {
        match self.parameter_index(id) {
            Some(index) => self.set_parameter_by_index(index, value),
            None => false,
        }
    }

    /// Sets a parameter by index, clamping the value to the model's declared range.
    ///
    /// Returns `false` when the index is out of range.
    pub fn set_parameter_by_index(&mut self, index: usize, value: f32) -> bool {
        let Some(slot) = self.parameter_values.get_mut(index) else {
            return false;
        };
        let (minimum, maximum) = parameter_clamp_range(&self.bindings, index);
        *slot = clamp_parameter_value(value, minimum, maximum);
        true
    }

    /// Sets a parameter with a normalized `0.0..=1.0` value.
    pub fn set_parameter_normalized(&mut self, id: &str, value: f32) -> bool {
        match self.parameter_index(id) {
            Some(index) => self.set_parameter_normalized_by_index(index, value),
            None => false,
        }
    }

    /// Sets a parameter by index with a normalized `0.0..=1.0` value.
    pub fn set_parameter_normalized_by_index(&mut self, index: usize, value: f32) -> bool {
        let Some(raw) = self.raw_parameter_value_from_normalized_index(index, value) else {
            return false;
        };
        self.set_parameter_by_index(index, raw)
    }

    /// Returns the pending override value for a parameter id.
    ///
    /// Overrides are separate from current parameter values until
    /// [`apply_parameter_overrides`](Self::apply_parameter_overrides) is called.
    pub fn parameter_override_value(&self, id: &str) -> Option<f32> {
        let index = self.parameter_index(id)?;
        self.parameter_override_value_by_index(index)
    }

    /// Returns the pending override value for a parameter index.
    pub fn parameter_override_value_by_index(&self, index: usize) -> Option<f32> {
        self.parameter_overrides.get(index).copied().flatten()
    }

    /// Returns the pending override value mapped into `0.0..=1.0`.
    pub fn parameter_override_normalized_value(&self, id: &str) -> Option<f32> {
        let index = self.parameter_index(id)?;
        self.parameter_override_normalized_value_by_index(index)
    }

    /// Returns the pending override value for an index mapped into `0.0..=1.0`.
    pub fn parameter_override_normalized_value_by_index(&self, index: usize) -> Option<f32> {
        let minimum = self.parameter_minimum_by_index(index)?;
        let maximum = self.parameter_maximum_by_index(index)?;
        let value = self.parameter_override_value_by_index(index)?;
        Some(normalized_parameter_value(value, minimum, maximum))
    }

    /// Stores a parameter override by id without immediately changing the value.
    pub fn set_parameter_override(&mut self, id: &str, value: f32) -> bool {
        match self.parameter_index(id) {
            Some(index) => self.set_parameter_override_by_index(index, value),
            None => false,
        }
    }

    /// Stores a parameter override by index without immediately changing the value.
    pub fn set_parameter_override_by_index(&mut self, index: usize, value: f32) -> bool {
        if index >= self.parameter_overrides.len() {
            return false;
        }
        let Some((minimum, maximum)) = self.parameter_range_by_index(index) else {
            return false;
        };
        self.parameter_overrides[index] = Some(clamp_parameter_value(value, minimum, maximum));
        true
    }

    /// Stores a normalized parameter override by id.
    pub fn set_parameter_override_normalized(&mut self, id: &str, value: f32) -> bool {
        match self.parameter_index(id) {
            Some(index) => self.set_parameter_override_normalized_by_index(index, value),
            None => false,
        }
    }

    /// Stores a normalized parameter override by index.
    pub fn set_parameter_override_normalized_by_index(&mut self, index: usize, value: f32) -> bool {
        let Some(raw) = self.raw_parameter_value_from_normalized_index(index, value) else {
            return false;
        };
        self.set_parameter_override_by_index(index, raw)
    }

    /// Clears a pending override for a parameter id.
    pub fn clear_parameter_override(&mut self, id: &str) -> bool {
        match self.parameter_index(id) {
            Some(index) => self.clear_parameter_override_by_index(index),
            None => false,
        }
    }

    /// Clears a pending override for a parameter index.
    pub fn clear_parameter_override_by_index(&mut self, index: usize) -> bool {
        let Some(slot) = self.parameter_overrides.get_mut(index) else {
            return false;
        };
        *slot = None;
        true
    }

    /// Clears all pending parameter overrides.
    pub fn clear_parameter_overrides(&mut self) {
        self.parameter_overrides.fill(None);
    }

    /// Applies all pending parameter overrides to the current parameter values.
    pub fn apply_parameter_overrides(&mut self) {
        for index in 0..self.parameter_overrides.len() {
            if let Some(value) = self.parameter_overrides[index] {
                self.set_parameter_by_index(index, value);
            }
        }
    }

    fn raw_parameter_value_from_normalized_index(&self, index: usize, value: f32) -> Option<f32> {
        let (minimum, maximum) = self.parameter_range_by_index(index)?;
        Some(raw_parameter_value_from_normalized_range(
            minimum, maximum, value,
        ))
    }

    /// Returns the first hit area containing a model-space point.
    ///
    /// Coordinates must be in the same model space as drawable vertices. UI code
    /// that starts from window pixels should first invert its render transform.
    pub fn hit_test(&self, x: f32, y: f32) -> Option<HitAreaInfo<'_>> {
        self.hit_test_all(x, y).next()
    }

    /// Returns all hit areas containing a model-space point.
    pub fn hit_test_all(&self, x: f32, y: f32) -> impl Iterator<Item = HitAreaInfo<'_>> + '_ {
        self.model.hit_areas().iter().filter_map(move |hit_area| {
            let drawable_index = self.drawable_index(hit_area.id())?;
            let mesh = self.meshes.get(drawable_index)?;
            drawable_contains_point(mesh, x, y).then_some(HitAreaInfo {
                id: hit_area.id(),
                name: hit_area.name(),
                drawable_index,
            })
        })
    }

    /// Resets current parameter values to the defaults declared by the model.
    pub fn reset_parameters(&mut self) {
        self.parameter_values
            .copy_from_slice(self.bindings.parameter_default_values());
    }

    pub fn set_physics(&mut self, physics: Physics3) {
        self.physics = Some(PhysicsRuntime::new(&physics, self.ids.parameters()));
    }

    pub fn clear_physics(&mut self) {
        self.physics = None;
    }

    pub fn physics(&self) -> Option<&PhysicsRuntime> {
        self.physics.as_ref()
    }

    pub fn physics_options(&self) -> Option<PhysicsOptions> {
        self.physics.as_ref().map(PhysicsRuntime::options)
    }

    pub fn set_physics_options(&mut self, options: PhysicsOptions) -> bool {
        let Some(physics) = &mut self.physics else {
            return false;
        };
        physics.set_options(options);
        true
    }

    pub fn reset_physics(&mut self) -> bool {
        let Some(physics) = &mut self.physics else {
            return false;
        };
        physics.reset();
        true
    }

    pub fn stabilize_physics(&mut self) -> bool {
        let Some(physics) = &mut self.physics else {
            return false;
        };
        physics.stabilize(
            &mut self.parameter_values,
            self.bindings.parameter_min_values(),
            self.bindings.parameter_max_values(),
            self.bindings.parameter_default_values(),
        );
        true
    }

    pub fn apply_physics(&mut self, delta_time_seconds: f32) -> bool {
        let Some(physics) = &mut self.physics else {
            return false;
        };
        physics.evaluate(
            &mut self.parameter_values,
            self.bindings.parameter_min_values(),
            self.bindings.parameter_max_values(),
            self.bindings.parameter_default_values(),
            delta_time_seconds,
        );
        true
    }

    /// Returns all part ids in model order.
    pub fn part_ids(&self) -> &[String] {
        self.ids.parts()
    }

    /// Returns the model-order index for a part id.
    pub fn part_index(&self, id: &str) -> Option<usize> {
        self.part_index.get(id).copied()
    }

    /// Overrides a part opacity by id.
    ///
    /// Values are clamped to `0.0..=1.0`. Pose fading can still affect the final
    /// drawable opacity after this override is applied.
    pub fn set_part_opacity(&mut self, id: &str, value: f32) -> bool {
        match self.part_index(id) {
            Some(index) => self.set_part_opacity_by_index(index, value),
            None => false,
        }
    }

    /// Overrides a part opacity by index.
    pub fn set_part_opacity_by_index(&mut self, index: usize, value: f32) -> bool {
        let Some(slot) = self.part_opacity_overrides.get_mut(index) else {
            return false;
        };
        *slot = Some(value.clamp(0.0, 1.0));
        true
    }

    /// Clears all part opacity overrides.
    pub fn reset_part_opacities(&mut self) {
        self.part_opacity_overrides
            .iter_mut()
            .for_each(|o| *o = None);
    }

    /// Advances pose fade state by `delta_seconds`.
    ///
    /// Call this once per frame for models that include a `pose3.json` file.
    pub fn apply_pose(&mut self, delta_seconds: f32) {
        for group in &self.pose_groups {
            let selection: Vec<f32> = group
                .members
                .iter()
                .map(|&part| self.part_selection_opacity(part))
                .collect();
            let mut faded: Vec<f32> = group
                .members
                .iter()
                .map(|&part| self.pose_opacities[part])
                .collect();

            if update_pose_group_opacities(
                &selection,
                &mut faded,
                delta_seconds,
                self.pose_fade_time,
            )
            .is_none()
            {
                continue;
            }

            for (opacity, &part) in faded.iter().zip(&group.members) {
                self.pose_opacities[part] = *opacity;
            }
            for (member_position, &part) in group.members.iter().enumerate() {
                let _ = copy_pose_link_opacities(
                    &mut self.pose_opacities,
                    part,
                    &group.links[member_position],
                );
            }
        }
    }

    fn part_selection_opacity(&self, part_index: usize) -> f32 {
        self.part_opacity_overrides[part_index].unwrap_or_else(|| {
            self.parts
                .interpolate_opacity(part_index, &self.bindings, &self.parameter_values)
                .unwrap_or(1.0)
        })
    }

    fn part_drawable_opacity(&self, part_index: usize) -> f32 {
        self.part_opacity_overrides[part_index].unwrap_or(1.0)
    }

    fn update_part_opacities(&mut self) {
        self.update_direct_part_opacities();
        self.apply_parent_part_opacities();
    }

    fn update_direct_part_opacities(&mut self) {
        for index in 0..self.part_opacities.len() {
            let base = self.part_drawable_opacity(index);
            self.part_opacities[index] = base * self.pose_opacities[index];
        }
    }

    fn apply_parent_part_opacities(&mut self) {
        for index in 0..self.part_opacities.len() {
            let mut opacity = self.part_opacities[index];
            let mut parent = self.parts.parent_part_index(index);
            while let Some(parent_index) = parent.and_then(|p| usize::try_from(p).ok()) {
                opacity *= self.part_opacities[parent_index];
                parent = self.parts.parent_part_index(parent_index);
            }
            self.part_opacities[index] = opacity;
        }
    }

    fn drawable_part_opacities(&self) -> Vec<f32> {
        (0..self.art_meshes.meshes().len())
            .map(|drawable_index| {
                self.offscreen
                    .drawable_parent_part_index(drawable_index)
                    .and_then(|p| usize::try_from(p).ok())
                    .and_then(|part_index| self.part_opacities.get(part_index).copied())
                    .unwrap_or(1.0)
            })
            .collect()
    }

    /// Rebuilds drawable meshes from the current runtime state.
    ///
    /// Call this after changing parameters, applying motion or expression
    /// players, changing part opacities, or advancing pose state. Returns `None`
    /// when the model data cannot produce a valid mesh update.
    pub fn update_meshes(&mut self) -> Option<()> {
        self.update_part_opacities();
        let drawable_part_opacities = self.drawable_part_opacities();
        self.rebuild_or_update_meshes(&drawable_part_opacities)?;
        self.apply_mesh_post_processing()
    }

    fn rebuild_or_update_meshes(&mut self, drawable_part_opacities: &[f32]) -> Option<()> {
        if self.meshes.len() == self.art_meshes.meshes().len() {
            update_moc3_drawable_meshes_with_parameters_offscreen_and_part_opacities(
                &mut self.meshes,
                &mut self.mesh_update_scratch,
                &self.art_meshes,
                &self.art_mesh_keyforms,
                &self.deformers,
                &self.bindings,
                &self.ids,
                &self.offscreen,
                &self.parameter_values,
                drawable_part_opacities,
            )?;
        } else {
            self.meshes = build_moc3_drawable_meshes_with_parameters_offscreen_and_part_opacities(
                &self.art_meshes,
                &self.art_mesh_keyforms,
                &self.deformers,
                &self.bindings,
                &self.ids,
                &self.offscreen,
                &self.parameter_values,
                drawable_part_opacities,
            )?;
        }
        Some(())
    }

    fn apply_mesh_post_processing(&mut self) -> Option<()> {
        self.glues
            .apply(&mut self.meshes, &self.bindings, &self.parameter_values)?;
        self.apply_group_render_orders();
        Some(())
    }

    fn apply_group_render_orders(&mut self) {
        let Some(groups) = self.draw_order_groups.as_ref() else {
            return;
        };
        let drawable_draw_orders: Vec<i32> = self
            .meshes
            .iter()
            .map(|mesh| draw_order_from_raw(mesh.draw_order()))
            .collect();

        let part_count = self.parts.part_count();
        let mut part_draw_orders = vec![0i32; part_count];
        let mut part_enable = vec![false; part_count];
        for index in 0..part_count {
            if let Some(raw) =
                self.parts
                    .interpolate_draw_order(index, &self.bindings, &self.parameter_values)
            {
                part_draw_orders[index] = draw_order_from_raw(raw);
                part_enable[index] = true;
            }
        }

        let Some(render_orders) = groups.render_orders(
            &drawable_draw_orders,
            &part_draw_orders,
            &part_enable,
            self.offscreen.part_offscreen_indices(),
            self.offscreen.offscreen_count(),
        ) else {
            return;
        };
        for (mesh, render_order) in self.meshes.iter_mut().zip(&render_orders) {
            mesh.set_render_order(*render_order);
        }
    }

    /// Returns the current drawable meshes in model order.
    ///
    /// Sort with [`crate::render::common::draw_order_indices`] or use a renderer
    /// backend before issuing draw calls.
    pub fn meshes(&self) -> &[Moc3DrawableMesh] {
        &self.meshes
    }

    fn parameter_range_by_index(&self, index: usize) -> Option<(f32, f32)> {
        Some((
            self.parameter_minimum_by_index(index)?,
            self.parameter_maximum_by_index(index)?,
        ))
    }
}

fn normalized_parameter_value(value: f32, minimum: f32, maximum: f32) -> f32 {
    if maximum <= minimum {
        0.0
    } else {
        ((value - minimum) / (maximum - minimum)).clamp(0.0, 1.0)
    }
}

fn raw_parameter_value_from_normalized_range(minimum: f32, maximum: f32, value: f32) -> f32 {
    let amount = value.clamp(0.0, 1.0);
    minimum + (maximum - minimum) * amount
}

fn parameter_clamp_range(bindings: &Moc3KeyformBindings, index: usize) -> (f32, f32) {
    let minimum = bindings
        .parameter_min_values()
        .get(index)
        .copied()
        .unwrap_or(f32::MIN);
    let maximum = bindings
        .parameter_max_values()
        .get(index)
        .copied()
        .unwrap_or(f32::MAX);
    (minimum, maximum)
}

fn build_index(ids: &[String]) -> HashMap<String, usize> {
    ids.iter()
        .enumerate()
        .map(|(index, id)| (id.clone(), index))
        .collect()
}

fn drawable_contains_point(mesh: &Moc3DrawableMesh, x: f32, y: f32) -> bool {
    let Some(first) = mesh.vertices().first() else {
        return false;
    };

    let [first_x, first_y] = first.position();
    let mut min_x = first_x;
    let mut min_y = first_y;
    let mut max_x = first_x;
    let mut max_y = first_y;

    for vertex in mesh.vertices().iter().skip(1) {
        let [vertex_x, vertex_y] = vertex.position();
        min_x = min_x.min(vertex_x);
        min_y = min_y.min(vertex_y);
        max_x = max_x.max(vertex_x);
        max_y = max_y.max(vertex_y);
    }

    (min_x..=max_x).contains(&x) && (min_y..=max_y).contains(&y)
}

fn build_pose_groups(pose: &Pose3, part_index: &HashMap<String, usize>) -> Vec<PoseGroup> {
    pose.groups()
        .iter()
        .filter_map(|group| {
            let mut members = Vec::new();
            let mut links = Vec::new();
            for part in group {
                let Some(&part_idx) = part_index.get(part.id()) else {
                    continue;
                };
                members.push(part_idx);
                links.push(
                    part.links()
                        .iter()
                        .filter_map(|link| part_index.get(link).copied())
                        .collect(),
                );
            }
            (members.len() >= 2).then_some(PoseGroup { members, links })
        })
        .collect()
}

fn initial_pose_opacities(groups: &[PoseGroup], part_count: usize) -> Vec<f32> {
    let mut opacities = vec![1.0; part_count];
    for group in groups {
        for (position, &part) in group.members.iter().enumerate() {
            let opacity = if position == 0 { 1.0 } else { 0.0 };
            opacities[part] = opacity;
            for &link in &group.links[position] {
                opacities[link] = opacity;
            }
        }
    }
    opacities
}

#[cfg(test)]
mod tests {
    use crate::{
        assets::load_model_runtime,
        moc3::{Moc3ArtMeshInfo, Moc3ArtMeshes, Moc3OffscreenInfo, Moc3Parts},
    };

    #[test]
    fn part_keyform_opacity_does_not_drive_drawable_visibility() {
        let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
        let runtime = model.runtime_mut();
        runtime.art_meshes = Moc3ArtMeshes::from_parts(
            vec![Moc3ArtMeshInfo::new(0, 0, 3, 0, 0, 3, 0, 0)],
            vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
            vec![0, 1, 2],
            Vec::new(),
        )
        .unwrap();
        runtime.offscreen = Moc3OffscreenInfo::from_parts(vec![-1], vec![0], vec![-1], Vec::new());
        runtime.parts =
            Moc3Parts::from_parts(vec![-1], vec![-1], vec![0], vec![1], vec![0.0], vec![0.0]);
        runtime.part_opacity_overrides = vec![None];
        runtime.part_opacities = vec![1.0];
        runtime.pose_opacities = vec![1.0];

        runtime.update_part_opacities();

        assert_eq!(runtime.drawable_part_opacities(), vec![1.0]);
    }
}
