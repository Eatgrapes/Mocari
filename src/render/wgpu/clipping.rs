use crate::core::Matrix44;

use super::{
    buffers::WgpuMeshBuffers,
    texture::{WgpuClipParams, WgpuMaskParams, WgpuTransform},
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WgpuMaskChannel {
    Red,
    Green,
    Blue,
    Alpha,
}

impl WgpuMaskChannel {
    pub fn index(self) -> usize {
        match self {
            Self::Red => 0,
            Self::Green => 1,
            Self::Blue => 2,
            Self::Alpha => 3,
        }
    }

    pub fn flag(self) -> [f32; 4] {
        match self {
            Self::Red => [1.0, 0.0, 0.0, 0.0],
            Self::Green => [0.0, 1.0, 0.0, 0.0],
            Self::Blue => [0.0, 0.0, 1.0, 0.0],
            Self::Alpha => [0.0, 0.0, 0.0, 1.0],
        }
    }

    fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::Red),
            1 => Some(Self::Green),
            2 => Some(Self::Blue),
            3 => Some(Self::Alpha),
            _ => None,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct WgpuClippingRect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl WgpuClippingRect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn x(&self) -> f32 {
        self.x
    }

    pub fn y(&self) -> f32 {
        self.y
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    fn expanded(self, margin_ratio: f32) -> Self {
        let margin_x = self.width * margin_ratio;
        let margin_y = self.height * margin_ratio;
        Self::new(
            self.x - margin_x,
            self.y - margin_y,
            self.width + margin_x * 2.0,
            self.height + margin_y * 2.0,
        )
    }

    fn union(self, other: Self) -> Self {
        let min_x = self.x.min(other.x);
        let min_y = self.y.min(other.y);
        let max_x = (self.x + self.width).max(other.x + other.width);
        let max_y = (self.y + self.height).max(other.y + other.height);
        Self::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct WgpuClippingLayout {
    channel: WgpuMaskChannel,
    bounds: WgpuClippingRect,
}

impl WgpuClippingLayout {
    pub fn new(channel: WgpuMaskChannel, bounds: WgpuClippingRect) -> Self {
        Self { channel, bounds }
    }

    pub fn channel(&self) -> WgpuMaskChannel {
        self.channel
    }

    pub fn channel_flag(&self) -> [f32; 4] {
        self.channel.flag()
    }

    pub fn bounds(&self) -> WgpuClippingRect {
        self.bounds
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WgpuClippingLayoutError {
    TooManyMasksForSingleTexture { mask_count: usize },
    MissingDrawableBounds { drawable_index: usize },
    MissingLayout { context_index: usize },
    MissingMaskMatrix { context_index: usize },
    MissingDrawMatrix { context_index: usize },
    InvalidMaskDrawableIndex { drawable_index: i32 },
    DegenerateClippedBounds { context_index: usize },
}

impl std::fmt::Display for WgpuClippingLayoutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyMasksForSingleTexture { mask_count } => write!(
                formatter,
                "single mask texture supports at most 36 clipping contexts, got {mask_count}"
            ),
            Self::MissingDrawableBounds { drawable_index } => {
                write!(
                    formatter,
                    "drawable {drawable_index} has no clipping bounds"
                )
            }
            Self::MissingLayout { context_index } => {
                write!(formatter, "clipping context {context_index} has no layout")
            }
            Self::MissingMaskMatrix { context_index } => write!(
                formatter,
                "clipping context {context_index} has no mask matrix"
            ),
            Self::MissingDrawMatrix { context_index } => write!(
                formatter,
                "clipping context {context_index} has no draw matrix"
            ),
            Self::InvalidMaskDrawableIndex { drawable_index } => {
                write!(formatter, "invalid mask drawable index {drawable_index}")
            }
            Self::DegenerateClippedBounds { context_index } => write!(
                formatter,
                "clipping context {context_index} has degenerate clipped bounds"
            ),
        }
    }
}

impl std::error::Error for WgpuClippingLayoutError {}

#[derive(Debug, Clone, PartialEq)]
pub struct WgpuClippingContext {
    masks: Vec<i32>,
    drawable_indices: Vec<usize>,
    layout: Option<WgpuClippingLayout>,
    all_clipped_draw_rect: Option<WgpuClippingRect>,
    matrix_for_mask: Option<Matrix44>,
    matrix_for_draw: Option<Matrix44>,
}

impl WgpuClippingContext {
    pub fn masks(&self) -> &[i32] {
        &self.masks
    }

    pub fn drawable_indices(&self) -> &[usize] {
        &self.drawable_indices
    }

    pub fn layout(&self) -> Option<WgpuClippingLayout> {
        self.layout
    }

    pub fn all_clipped_draw_rect(&self) -> Option<WgpuClippingRect> {
        self.all_clipped_draw_rect
    }

    pub fn matrix_for_mask(&self) -> Option<Matrix44> {
        self.matrix_for_mask
    }

    pub fn matrix_for_draw(&self) -> Option<Matrix44> {
        self.matrix_for_draw
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WgpuClippingPlan {
    contexts: Vec<WgpuClippingContext>,
    unmasked_drawable_indices: Vec<usize>,
}

impl WgpuClippingPlan {
    pub fn from_mesh_buffers(mesh_buffers: &WgpuMeshBuffers) -> Self {
        let mut contexts = Vec::<WgpuClippingContext>::new();
        let mut unmasked_drawable_indices = Vec::new();

        for (drawable_index, drawable) in mesh_buffers.drawables().iter().enumerate() {
            if drawable.masks().is_empty() {
                unmasked_drawable_indices.push(drawable_index);
                continue;
            }

            if let Some(context) = contexts
                .iter_mut()
                .find(|context| same_mask_set(&context.masks, drawable.masks()))
            {
                context.drawable_indices.push(drawable_index);
            } else {
                contexts.push(WgpuClippingContext {
                    masks: drawable.masks().to_vec(),
                    drawable_indices: vec![drawable_index],
                    layout: None,
                    all_clipped_draw_rect: None,
                    matrix_for_mask: None,
                    matrix_for_draw: None,
                });
            }
        }

        Self {
            contexts,
            unmasked_drawable_indices,
        }
    }

    pub fn contexts(&self) -> &[WgpuClippingContext] {
        &self.contexts
    }

    pub fn unmasked_drawable_indices(&self) -> &[usize] {
        &self.unmasked_drawable_indices
    }

    pub fn assign_single_texture_layouts(&mut self) -> Result<(), WgpuClippingLayoutError> {
        let using_clip_count = self.contexts.len();
        if using_clip_count > 36 {
            return Err(WgpuClippingLayoutError::TooManyMasksForSingleTexture {
                mask_count: using_clip_count,
            });
        }

        let div = using_clip_count / 4;
        let rem = using_clip_count % 4;
        let mut context_index = 0;

        for channel_index in 0..4 {
            let layout_count = div + usize::from(channel_index < rem);
            let channel = WgpuMaskChannel::from_index(channel_index).expect("valid RGBA channel");

            for layout_index in 0..layout_count {
                self.contexts[context_index].layout = Some(WgpuClippingLayout::new(
                    channel,
                    clipping_layout_bounds(layout_index, layout_count),
                ));
                context_index += 1;
            }
        }

        Ok(())
    }

    pub fn prepare_single_texture_masks(
        &mut self,
        mesh_buffers: &WgpuMeshBuffers,
    ) -> Result<(), WgpuClippingLayoutError> {
        self.assign_single_texture_layouts()?;

        for context_index in 0..self.contexts.len() {
            let layout = self.contexts[context_index]
                .layout
                .ok_or(WgpuClippingLayoutError::MissingLayout { context_index })?;
            let bounds = clipped_draw_total_bounds(
                mesh_buffers,
                self.contexts[context_index].drawable_indices(),
            )?
            .ok_or(WgpuClippingLayoutError::DegenerateClippedBounds { context_index })?
            .expanded(0.05);
            let (matrix_for_mask, matrix_for_draw) = clipping_matrices(bounds, layout.bounds())
                .ok_or(WgpuClippingLayoutError::DegenerateClippedBounds { context_index })?;

            self.contexts[context_index].all_clipped_draw_rect = Some(bounds);
            self.contexts[context_index].matrix_for_mask = Some(matrix_for_mask);
            self.contexts[context_index].matrix_for_draw = Some(matrix_for_draw);
        }

        Ok(())
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
}

impl WgpuClippingResources {
    pub fn contexts(&self) -> &[WgpuPreparedClippingContext] {
        &self.contexts
    }

    pub fn context_for_drawable(
        &self,
        drawable_index: usize,
    ) -> Option<&WgpuPreparedClippingContext> {
        self.contexts
            .iter()
            .find(|context| context.drawable_indices.contains(&drawable_index))
    }
}

fn same_mask_set(left: &[i32], right: &[i32]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    let mut sorted_left = left.to_vec();
    let mut sorted_right = right.to_vec();
    sorted_left.sort_unstable();
    sorted_right.sort_unstable();

    sorted_left == sorted_right
}

fn clipping_layout_bounds(layout_index: usize, layout_count: usize) -> WgpuClippingRect {
    match layout_count {
        0 => WgpuClippingRect::new(0.0, 0.0, 0.0, 0.0),
        1 => WgpuClippingRect::new(0.0, 0.0, 1.0, 1.0),
        2 => WgpuClippingRect::new(layout_index as f32 * 0.5, 0.0, 0.5, 1.0),
        3 | 4 => {
            let xpos = layout_index % 2;
            let ypos = layout_index / 2;
            WgpuClippingRect::new(xpos as f32 * 0.5, ypos as f32 * 0.5, 0.5, 0.5)
        }
        5..=9 => {
            let xpos = layout_index % 3;
            let ypos = layout_index / 3;
            WgpuClippingRect::new(xpos as f32 / 3.0, ypos as f32 / 3.0, 1.0 / 3.0, 1.0 / 3.0)
        }
        _ => unreachable!("single texture channel layouts are capped at 9 cells"),
    }
}

fn clipped_draw_total_bounds(
    mesh_buffers: &WgpuMeshBuffers,
    drawable_indices: &[usize],
) -> Result<Option<WgpuClippingRect>, WgpuClippingLayoutError> {
    let mut bounds: Option<WgpuClippingRect> = None;

    for &drawable_index in drawable_indices {
        let drawable_bounds = mesh_buffers
            .drawables()
            .get(drawable_index)
            .ok_or(WgpuClippingLayoutError::MissingDrawableBounds { drawable_index })?
            .bounds()
            .ok_or(WgpuClippingLayoutError::MissingDrawableBounds { drawable_index })?;

        bounds = Some(match bounds {
            Some(bounds) => bounds.union(drawable_bounds),
            None => drawable_bounds,
        });
    }

    Ok(bounds)
}

fn clipping_matrices(
    bounds: WgpuClippingRect,
    layout: WgpuClippingRect,
) -> Option<(Matrix44, Matrix44)> {
    if bounds.width <= 0.0 || bounds.height <= 0.0 {
        return None;
    }

    let scale_x = layout.width / bounds.width;
    let scale_y = layout.height / bounds.height;
    let draw_translate_x = -bounds.x * scale_x + layout.x;
    let normalized_translate_y = -bounds.y * scale_y + layout.y;
    let texture_translate_y = 1.0 - layout.y + bounds.y * scale_y;

    let mut matrix_for_draw = Matrix44::identity();
    matrix_for_draw.scale(scale_x, -scale_y);
    matrix_for_draw.translate(draw_translate_x, texture_translate_y);

    let mut matrix_for_mask = Matrix44::identity();
    matrix_for_mask.scale(scale_x * 2.0, scale_y * 2.0);
    matrix_for_mask.translate(
        draw_translate_x * 2.0 - 1.0,
        normalized_translate_y * 2.0 - 1.0,
    );

    Some((matrix_for_mask, matrix_for_draw))
}
