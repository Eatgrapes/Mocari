use crate::render::common::{ClippingContext, ClippingLayoutError, ClippingPlan};

use super::{
    buffers::WgpuMeshBuffers,
    texture::{WgpuClipParams, WgpuMaskParams, WgpuTransform},
};

#[derive(Debug, Clone, PartialEq)]
pub struct WgpuClippingPlan {
    inner: ClippingPlan,
}

impl WgpuClippingPlan {
    pub fn from_mesh_buffers(mesh_buffers: &WgpuMeshBuffers) -> Self {
        Self {
            inner: ClippingPlan::from_drawables(mesh_buffers.iter_drawable_infos()),
        }
    }

    pub fn contexts(&self) -> &[ClippingContext] {
        self.inner.contexts()
    }

    pub fn unmasked_drawable_indices(&self) -> &[usize] {
        self.inner.unmasked_drawable_indices()
    }

    pub fn assign_single_texture_layouts(&mut self) -> Result<(), ClippingLayoutError> {
        self.inner.assign_single_texture_layouts()
    }

    pub fn prepare_single_texture_masks(
        &mut self,
        mesh_buffers: &WgpuMeshBuffers,
    ) -> Result<(), ClippingLayoutError> {
        self.inner
            .prepare_single_texture_masks_from_bounds(|drawable_index| {
                mesh_buffers.drawable_bounds(drawable_index)
            })
    }
}

#[derive(Debug)]
pub struct WgpuMaskRenderTarget {
    pub(super) texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) bind_group: wgpu::BindGroup,
    pub(super) width: u32,
    pub(super) height: u32,
}

impl WgpuMaskRenderTarget {
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}

#[derive(Debug)]
pub struct WgpuPreparedClippingContext {
    pub(super) mask_drawable_indices: Vec<usize>,
    pub(super) drawable_indices: Vec<usize>,
    pub(super) mask_transform: WgpuTransform,
    pub(super) mask_params: WgpuMaskParams,
    pub(super) clip_params: WgpuClipParams,
}

impl WgpuPreparedClippingContext {
    pub fn mask_drawable_indices(&self) -> &[usize] {
        &self.mask_drawable_indices
    }

    pub fn drawable_indices(&self) -> &[usize] {
        &self.drawable_indices
    }

    pub fn mask_transform(&self) -> &WgpuTransform {
        &self.mask_transform
    }

    pub fn mask_params(&self) -> &WgpuMaskParams {
        &self.mask_params
    }

    pub fn clip_params(&self) -> &WgpuClipParams {
        &self.clip_params
    }
}

#[derive(Debug)]
pub struct WgpuClippingResources {
    pub(super) contexts: Vec<WgpuPreparedClippingContext>,
    pub(super) drawable_context_indices: Vec<Option<usize>>,
}

impl WgpuClippingResources {
    pub fn contexts(&self) -> &[WgpuPreparedClippingContext] {
        &self.contexts
    }

    pub fn context_for_drawable(
        &self,
        drawable_index: usize,
    ) -> Option<&WgpuPreparedClippingContext> {
        self.drawable_context_indices
            .get(drawable_index)
            .and_then(|context_index| context_index.and_then(|index| self.contexts.get(index)))
    }
}
