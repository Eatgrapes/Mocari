use super::{
    WgpuLive2dRenderer,
    buffers::WgpuMeshBuffers,
    clipping::{WgpuClippingResources, WgpuMaskRenderTarget},
    texture::{WgpuTexture, WgpuTransform},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WgpuRenderError {
    InvalidTextureIndex {
        texture_index: i32,
    },
    MissingTexture {
        texture_index: i32,
    },
    MissingDrawable {
        drawable_index: usize,
    },
    MissingClippingContext {
        drawable_index: usize,
    },
    UnsupportedClippingMasks {
        drawable_index: usize,
        mask_count: usize,
    },
}

impl std::fmt::Display for WgpuRenderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTextureIndex { texture_index } => {
                write!(formatter, "invalid texture index {texture_index}")
            }
            Self::MissingTexture { texture_index } => {
                write!(formatter, "missing texture bind group {texture_index}")
            }
            Self::MissingDrawable { drawable_index } => {
                write!(formatter, "missing drawable {drawable_index}")
            }
            Self::MissingClippingContext { drawable_index } => write!(
                formatter,
                "drawable {drawable_index} has clipping masks but no prepared clipping context"
            ),
            Self::UnsupportedClippingMasks {
                drawable_index,
                mask_count,
            } => write!(
                formatter,
                "drawable {drawable_index} uses {mask_count} clipping masks, but clipping is not implemented"
            ),
        }
    }
}

impl std::error::Error for WgpuRenderError {}

impl WgpuLive2dRenderer {
    pub fn draw(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh_buffers: &WgpuMeshBuffers,
        texture_bind_groups: &[wgpu::BindGroup],
    ) -> Result<u32, WgpuRenderError> {
        self.draw_with_bind_groups_and_transform(
            render_pass,
            mesh_buffers,
            texture_bind_groups,
            &self.identity_transform,
        )
    }

    pub fn draw_with_bind_groups_and_transform(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh_buffers: &WgpuMeshBuffers,
        texture_bind_groups: &[wgpu::BindGroup],
        transform: &WgpuTransform,
    ) -> Result<u32, WgpuRenderError> {
        self.draw_with_bind_group_provider(render_pass, mesh_buffers, transform, |texture_index| {
            texture_bind_group_at(texture_bind_groups, texture_index)
        })
    }

    pub fn draw_with_textures(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh_buffers: &WgpuMeshBuffers,
        textures: &[WgpuTexture],
    ) -> Result<u32, WgpuRenderError> {
        self.draw_with_textures_and_transform(
            render_pass,
            mesh_buffers,
            textures,
            &self.identity_transform,
        )
    }

    pub fn draw_with_textures_and_transform(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh_buffers: &WgpuMeshBuffers,
        textures: &[WgpuTexture],
        transform: &WgpuTransform,
    ) -> Result<u32, WgpuRenderError> {
        self.draw_with_bind_group_provider(render_pass, mesh_buffers, transform, |texture_index| {
            texture_bind_group_from_textures(textures, texture_index)
        })
    }

    pub fn draw_with_textures_and_clipping(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh_buffers: &WgpuMeshBuffers,
        textures: &[WgpuTexture],
        clipping_resources: &WgpuClippingResources,
        mask_target: &WgpuMaskRenderTarget,
    ) -> Result<u32, WgpuRenderError> {
        self.draw_with_textures_clipping_and_transform(
            render_pass,
            mesh_buffers,
            textures,
            clipping_resources,
            mask_target,
            &self.identity_transform,
        )
    }

    pub fn draw_with_textures_clipping_and_transform(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh_buffers: &WgpuMeshBuffers,
        textures: &[WgpuTexture],
        clipping_resources: &WgpuClippingResources,
        mask_target: &WgpuMaskRenderTarget,
        transform: &WgpuTransform,
    ) -> Result<u32, WgpuRenderError> {
        self.draw_with_clipping_bind_group_provider(
            render_pass,
            mesh_buffers,
            clipping_resources,
            mask_target.bind_group(),
            transform,
            |texture_index| texture_bind_group_from_textures(textures, texture_index),
        )
    }

    pub fn draw_masks(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh_buffers: &WgpuMeshBuffers,
        clipping_resources: &WgpuClippingResources,
        texture_bind_groups: &[wgpu::BindGroup],
    ) -> Result<u32, WgpuRenderError> {
        self.draw_masks_with_bind_group_provider(
            render_pass,
            mesh_buffers,
            clipping_resources,
            |texture_index| texture_bind_group_at(texture_bind_groups, texture_index),
        )
    }

    pub fn draw_masks_with_textures(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh_buffers: &WgpuMeshBuffers,
        clipping_resources: &WgpuClippingResources,
        textures: &[WgpuTexture],
    ) -> Result<u32, WgpuRenderError> {
        self.draw_masks_with_bind_group_provider(
            render_pass,
            mesh_buffers,
            clipping_resources,
            |texture_index| texture_bind_group_from_textures(textures, texture_index),
        )
    }

    fn draw_with_bind_group_provider<'a>(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh_buffers: &WgpuMeshBuffers,
        transform: &WgpuTransform,
        mut bind_group_for_texture: impl FnMut(i32) -> Result<&'a wgpu::BindGroup, WgpuRenderError>,
    ) -> Result<u32, WgpuRenderError> {
        let mut drawn = 0;
        for drawable_index in mesh_buffers.draw_order_indices() {
            let drawable = mesh_buffers
                .drawables()
                .get(drawable_index)
                .ok_or(WgpuRenderError::MissingDrawable { drawable_index })?;
            if !drawable.masks().is_empty() {
                return Err(WgpuRenderError::UnsupportedClippingMasks {
                    drawable_index,
                    mask_count: drawable.masks().len(),
                });
            }
            let texture_bind_group = bind_group_for_texture(drawable.texture_index())?;

            render_pass.set_pipeline(self.pipeline_for_blend_mode(drawable.blend_mode()));
            render_pass.set_bind_group(0, texture_bind_group, &[]);
            render_pass.set_bind_group(1, transform.bind_group(), &[]);
            render_pass.set_vertex_buffer(0, drawable.vertex_buffer().slice(..));
            render_pass
                .set_index_buffer(drawable.index_buffer().slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..drawable.index_count(), 0, 0..1);
            drawn += 1;
        }

        Ok(drawn)
    }

    fn draw_masks_with_bind_group_provider<'a>(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh_buffers: &WgpuMeshBuffers,
        clipping_resources: &WgpuClippingResources,
        mut bind_group_for_texture: impl FnMut(i32) -> Result<&'a wgpu::BindGroup, WgpuRenderError>,
    ) -> Result<u32, WgpuRenderError> {
        let mut drawn = 0;
        for context in clipping_resources.contexts() {
            for &drawable_index in context.mask_drawable_indices() {
                let drawable = mesh_buffers
                    .drawables()
                    .get(drawable_index)
                    .ok_or(WgpuRenderError::MissingDrawable { drawable_index })?;
                let texture_bind_group = bind_group_for_texture(drawable.texture_index())?;

                render_pass.set_pipeline(&self.mask_pipeline);
                render_pass.set_bind_group(0, texture_bind_group, &[]);
                render_pass.set_bind_group(1, context.mask_transform().bind_group(), &[]);
                render_pass.set_bind_group(2, context.mask_params().bind_group(), &[]);
                render_pass.set_vertex_buffer(0, drawable.vertex_buffer().slice(..));
                render_pass
                    .set_index_buffer(drawable.index_buffer().slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..drawable.index_count(), 0, 0..1);
                drawn += 1;
            }
        }

        Ok(drawn)
    }

    fn draw_with_clipping_bind_group_provider<'a>(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh_buffers: &WgpuMeshBuffers,
        clipping_resources: &WgpuClippingResources,
        mask_bind_group: &wgpu::BindGroup,
        transform: &WgpuTransform,
        mut bind_group_for_texture: impl FnMut(i32) -> Result<&'a wgpu::BindGroup, WgpuRenderError>,
    ) -> Result<u32, WgpuRenderError> {
        let mut drawn = 0;
        for drawable_index in mesh_buffers.draw_order_indices() {
            let drawable = mesh_buffers
                .drawables()
                .get(drawable_index)
                .ok_or(WgpuRenderError::MissingDrawable { drawable_index })?;
            let texture_bind_group = bind_group_for_texture(drawable.texture_index())?;

            if drawable.masks().is_empty() {
                render_pass.set_pipeline(self.pipeline_for_blend_mode(drawable.blend_mode()));
                render_pass.set_bind_group(0, texture_bind_group, &[]);
                render_pass.set_bind_group(1, transform.bind_group(), &[]);
            } else {
                let context = clipping_resources
                    .context_for_drawable(drawable_index)
                    .ok_or(WgpuRenderError::MissingClippingContext { drawable_index })?;
                render_pass
                    .set_pipeline(self.masked_pipeline_for_blend_mode(drawable.blend_mode()));
                render_pass.set_bind_group(0, texture_bind_group, &[]);
                render_pass.set_bind_group(1, transform.bind_group(), &[]);
                render_pass.set_bind_group(2, mask_bind_group, &[]);
                render_pass.set_bind_group(3, context.clip_params().bind_group(), &[]);
            }

            render_pass.set_vertex_buffer(0, drawable.vertex_buffer().slice(..));
            render_pass
                .set_index_buffer(drawable.index_buffer().slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..drawable.index_count(), 0, 0..1);
            drawn += 1;
        }

        Ok(drawn)
    }
}

fn texture_bind_group_at(
    texture_bind_groups: &[wgpu::BindGroup],
    texture_index: i32,
) -> Result<&wgpu::BindGroup, WgpuRenderError> {
    let texture_index_usize = usize::try_from(texture_index)
        .map_err(|_| WgpuRenderError::InvalidTextureIndex { texture_index })?;
    texture_bind_groups
        .get(texture_index_usize)
        .ok_or(WgpuRenderError::MissingTexture { texture_index })
}

fn texture_bind_group_from_textures(
    textures: &[WgpuTexture],
    texture_index: i32,
) -> Result<&wgpu::BindGroup, WgpuRenderError> {
    let texture_index_usize = usize::try_from(texture_index)
        .map_err(|_| WgpuRenderError::InvalidTextureIndex { texture_index })?;
    textures
        .get(texture_index_usize)
        .map(WgpuTexture::bind_group)
        .ok_or(WgpuRenderError::MissingTexture { texture_index })
}
