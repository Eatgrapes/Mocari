mod buffers;
mod clipping;
mod draw;
mod pipeline;
mod texture;

pub use buffers::{
    WgpuDrawableBuffers, WgpuDrawableVertex, WgpuMeshBuffers, create_wgpu_drawable_buffers,
    encode_wgpu_indices, encode_wgpu_vertices, wgpu_vertex_from_drawable_vertex,
    wgpu_vertices_from_drawable,
};
pub use clipping::{
    WgpuClippingContext, WgpuClippingLayout, WgpuClippingLayoutError, WgpuClippingPlan,
    WgpuClippingRect, WgpuClippingResources, WgpuMaskChannel, WgpuMaskRenderTarget,
    WgpuPreparedClippingContext,
};
pub use draw::WgpuRenderError;
pub use pipeline::{
    live2d_blend_state, live2d_masked_wgsl_source, live2d_wgsl_source, mask_wgsl_source,
    wgpu_mask_blend_state,
};
pub use texture::{
    WgpuClipParams, WgpuMaskParams, WgpuTexture, WgpuTextureError, WgpuTransform,
    encode_wgpu_clip_params, encode_wgpu_mask_params, encode_wgpu_matrix,
};

#[derive(Debug)]
pub struct WgpuLive2dRenderer {
    normal_pipeline: wgpu::RenderPipeline,
    additive_pipeline: wgpu::RenderPipeline,
    multiplicative_pipeline: wgpu::RenderPipeline,
    mask_pipeline: wgpu::RenderPipeline,
    masked_normal_pipeline: wgpu::RenderPipeline,
    masked_additive_pipeline: wgpu::RenderPipeline,
    masked_multiplicative_pipeline: wgpu::RenderPipeline,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    transform_bind_group_layout: wgpu::BindGroupLayout,
    mask_params_bind_group_layout: wgpu::BindGroupLayout,
    clip_params_bind_group_layout: wgpu::BindGroupLayout,
    identity_transform: WgpuTransform,
    sampler: wgpu::Sampler,
}
