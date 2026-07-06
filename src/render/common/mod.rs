mod clipping;
mod vertex;

#[cfg(feature = "wgpu")]
pub(crate) use clipping::draw_order_indices_from;
pub use clipping::{
    ClippingContext, ClippingLayout, ClippingLayoutError, ClippingPlan, ClippingRect, DrawableInfo,
    MaskChannel, draw_order_indices,
};
pub use vertex::{
    DrawableVertex, encode_indices, encode_vertices, encode_vertices_from_drawable,
    vertex_from_drawable_vertex, vertices_from_drawable,
};
