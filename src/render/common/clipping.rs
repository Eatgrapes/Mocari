use std::sync::Arc;

use crate::core::{Matrix44, draw_order_from_raw};
use crate::moc3::{Moc3DrawableBlendMode, Moc3DrawableMesh, Moc3DrawableVertex};

#[derive(Debug, Clone, PartialEq)]
/// Render-facing metadata extracted from a drawable mesh.
///
/// Build this with [`DrawableInfo::from_mesh`] before sorting draw order or
/// preparing clipping masks.
pub struct DrawableInfo {
    texture_index: i32,
    blend_mode: Moc3DrawableBlendMode,
    opacity: f32,
    draw_order: f32,
    render_order: i32,
    masks: Arc<[i32]>,
    mask_key: Arc<[i32]>,
    inverted_mask: bool,
    bounds: Option<ClippingRect>,
}

impl DrawableInfo {
    /// Creates render metadata from a runtime drawable mesh.
    pub fn from_mesh(mesh: &Moc3DrawableMesh) -> Self {
        let masks = Arc::<[i32]>::from(mesh.masks());
        let mask_key = sorted_mask_key(&masks);

        Self {
            texture_index: mesh.texture_index(),
            blend_mode: mesh.blend_mode(),
            opacity: mesh.opacity(),
            draw_order: mesh.draw_order(),
            render_order: mesh.render_order(),
            masks,
            mask_key,
            inverted_mask: mesh.is_inverted_mask(),
            bounds: drawable_vertex_bounds(mesh.vertices()),
        }
    }

    /// Returns the texture index referenced by this drawable.
    pub fn texture_index(&self) -> i32 {
        self.texture_index
    }

    /// Returns the drawable blend mode.
    pub fn blend_mode(&self) -> Moc3DrawableBlendMode {
        self.blend_mode
    }

    /// Returns the drawable opacity.
    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    /// Returns whether this drawable has non-zero opacity and valid bounds.
    pub fn is_visible(&self) -> bool {
        self.opacity > 0.0 && self.bounds.is_some()
    }

    /// Returns the raw draw order value.
    pub fn draw_order(&self) -> f32 {
        self.draw_order
    }

    /// Returns the runtime render order value.
    pub fn render_order(&self) -> i32 {
        self.render_order
    }

    /// Returns drawable indices used as masks for this drawable.
    pub fn masks(&self) -> &[i32] {
        self.masks.as_ref()
    }

    /// Returns whether this drawable uses inverted mask semantics.
    pub fn inverted_mask(&self) -> bool {
        self.inverted_mask
    }

    /// Returns the model-space bounding rectangle for this drawable.
    pub fn bounds(&self) -> Option<ClippingRect> {
        self.bounds
    }
}

fn sorted_mask_key(masks: &Arc<[i32]>) -> Arc<[i32]> {
    if masks.len() <= 1 {
        return Arc::clone(masks);
    }

    let mut key = masks.to_vec();
    key.sort_unstable();
    Arc::from(key)
}

/// Returns drawable indices sorted in the order they should be rendered.
pub fn draw_order_indices(drawables: &[DrawableInfo]) -> Vec<usize> {
    draw_order_indices_from(
        drawables.len(),
        |index| drawables[index].draw_order,
        |index| drawables[index].render_order,
    )
}

pub(crate) fn draw_order_indices_from(
    count: usize,
    mut draw_order: impl FnMut(usize) -> f32,
    mut render_order: impl FnMut(usize) -> i32,
) -> Vec<usize> {
    let mut indices = (0..count).collect::<Vec<_>>();
    if render_orders_are_total_rank_from(count, &mut render_order) {
        indices.sort_by_key(|&index| render_order(index));
        return indices;
    }
    indices.sort_by(|left, right| {
        draw_order_from_raw(draw_order(*left))
            .cmp(&draw_order_from_raw(draw_order(*right)))
            .then_with(|| render_order(*left).cmp(&render_order(*right)))
            .then_with(|| left.cmp(right))
    });
    indices
}

fn render_orders_are_total_rank_from(
    count: usize,
    render_order: &mut impl FnMut(usize) -> i32,
) -> bool {
    if count == 0 {
        return false;
    }
    let mut seen = vec![false; count];
    let mut identity = true;
    for index in 0..count {
        let Ok(rank) = usize::try_from(render_order(index)) else {
            return false;
        };
        match seen.get_mut(rank) {
            Some(slot) if !*slot => *slot = true,
            _ => return false,
        }
        identity &= rank == index;
    }
    !identity
}

fn drawable_vertex_bounds(vertices: &[Moc3DrawableVertex]) -> Option<ClippingRect> {
    let first = vertices.first()?;
    let mut min_x = first.position()[0];
    let mut min_y = first.position()[1];
    let mut max_x = min_x;
    let mut max_y = min_y;

    for vertex in vertices.iter().skip(1) {
        let [x, y] = vertex.position();
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }

    Some(ClippingRect::new(
        min_x,
        min_y,
        max_x - min_x,
        max_y - min_y,
    ))
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// One RGBA channel in a shared mask texture.
pub enum MaskChannel {
    /// Red channel.
    Red,
    /// Green channel.
    Green,
    /// Blue channel.
    Blue,
    /// Alpha channel.
    Alpha,
}

impl MaskChannel {
    /// Returns the zero-based channel index.
    pub fn index(self) -> usize {
        match self {
            Self::Red => 0,
            Self::Green => 1,
            Self::Blue => 2,
            Self::Alpha => 3,
        }
    }

    /// Returns a shader-friendly one-hot channel flag.
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
/// Axis-aligned rectangle used for drawable bounds and mask atlas regions.
pub struct ClippingRect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl ClippingRect {
    /// Creates a rectangle from origin and size.
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Returns the x coordinate.
    pub fn x(&self) -> f32 {
        self.x
    }

    /// Returns the y coordinate.
    pub fn y(&self) -> f32 {
        self.y
    }

    /// Returns the rectangle width.
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Returns the rectangle height.
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
/// Assigned location of one clipping context inside a mask texture.
pub struct ClippingLayout {
    channel: MaskChannel,
    bounds: ClippingRect,
}

impl ClippingLayout {
    /// Creates a layout from a channel and normalized texture bounds.
    pub fn new(channel: MaskChannel, bounds: ClippingRect) -> Self {
        Self { channel, bounds }
    }

    /// Returns the mask texture channel used by this layout.
    pub fn channel(&self) -> MaskChannel {
        self.channel
    }

    /// Returns a shader-friendly one-hot channel flag.
    pub fn channel_flag(&self) -> [f32; 4] {
        self.channel.flag()
    }

    /// Returns normalized mask texture bounds.
    pub fn bounds(&self) -> ClippingRect {
        self.bounds
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
/// Errors produced while preparing clipping mask layouts.
pub enum ClippingLayoutError {
    /// The single-texture layout has more contexts than its RGBA grid supports.
    #[error("single mask texture supports at most 36 clipping contexts, got {mask_count}")]
    TooManyMasksForSingleTexture { mask_count: usize },
    /// A drawable needed for clipping has no valid bounds.
    #[error("drawable {drawable_index} has no clipping bounds")]
    MissingDrawableBounds { drawable_index: usize },
    /// A clipping context has not been assigned a layout.
    #[error("clipping context {context_index} has no layout")]
    MissingLayout { context_index: usize },
    /// A clipping context has no matrix for rendering its mask.
    #[error("clipping context {context_index} has no mask matrix")]
    MissingMaskMatrix { context_index: usize },
    /// A clipping context has no matrix for drawing clipped meshes.
    #[error("clipping context {context_index} has no draw matrix")]
    MissingDrawMatrix { context_index: usize },
    /// A mask drawable index did not point to a drawable.
    #[error("invalid mask drawable index {drawable_index}")]
    InvalidMaskDrawableIndex { drawable_index: i32 },
    /// The clipped drawable bounds collapsed to zero area.
    #[error("clipping context {context_index} has degenerate clipped bounds")]
    DegenerateClippedBounds { context_index: usize },
}

#[derive(Debug, Clone, PartialEq)]
/// A group of drawables that share the same mask set.
pub struct ClippingContext {
    masks: Arc<[i32]>,
    mask_key: Arc<[i32]>,
    inverted: bool,
    drawable_indices: Vec<usize>,
    layout: Option<ClippingLayout>,
    all_clipped_draw_rect: Option<ClippingRect>,
    matrix_for_mask: Option<Matrix44>,
    matrix_for_draw: Option<Matrix44>,
}

impl ClippingContext {
    /// Returns drawable indices that act as masks for this context.
    pub fn masks(&self) -> &[i32] {
        self.masks.as_ref()
    }

    /// Returns whether this context uses inverted mask semantics.
    pub fn inverted(&self) -> bool {
        self.inverted
    }

    /// Returns drawables clipped by this context.
    pub fn drawable_indices(&self) -> &[usize] {
        &self.drawable_indices
    }

    /// Returns the assigned mask texture layout, if prepared.
    pub fn layout(&self) -> Option<ClippingLayout> {
        self.layout
    }

    /// Returns the combined bounds of all drawables clipped by this context.
    pub fn all_clipped_draw_rect(&self) -> Option<ClippingRect> {
        self.all_clipped_draw_rect
    }

    /// Returns the transform used when drawing mask geometry.
    pub fn matrix_for_mask(&self) -> Option<Matrix44> {
        self.matrix_for_mask
    }

    /// Returns the transform used when drawing clipped geometry.
    pub fn matrix_for_draw(&self) -> Option<Matrix44> {
        self.matrix_for_draw
    }
}

#[derive(Debug, Clone, PartialEq)]
/// A complete clipping plan for one frame of drawable data.
pub struct ClippingPlan {
    contexts: Vec<ClippingContext>,
    unmasked_drawable_indices: Vec<usize>,
}

impl ClippingPlan {
    /// Groups visible drawables by mask set.
    pub fn from_drawables<'a>(drawables: impl IntoIterator<Item = &'a DrawableInfo>) -> Self {
        let mut contexts = Vec::<ClippingContext>::new();
        let mut unmasked_drawable_indices = Vec::new();

        for (drawable_index, drawable) in drawables.into_iter().enumerate() {
            if !drawable.is_visible() {
                continue;
            }

            if drawable.masks().is_empty() {
                unmasked_drawable_indices.push(drawable_index);
                continue;
            }

            if let Some(context) = contexts.iter_mut().find(|context| {
                context.inverted == drawable.inverted_mask()
                    && context.mask_key.as_ref() == drawable.mask_key.as_ref()
            }) {
                context.drawable_indices.push(drawable_index);
            } else {
                contexts.push(ClippingContext {
                    masks: Arc::clone(&drawable.masks),
                    mask_key: Arc::clone(&drawable.mask_key),
                    inverted: drawable.inverted_mask(),
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

    /// Returns all clipping contexts that need mask rendering.
    pub fn contexts(&self) -> &[ClippingContext] {
        &self.contexts
    }

    /// Returns visible drawable indices that do not use masks.
    pub fn unmasked_drawable_indices(&self) -> &[usize] {
        &self.unmasked_drawable_indices
    }

    /// Assigns clipping contexts into one RGBA mask texture.
    ///
    /// The layout supports up to 36 contexts: 9 cells per color channel.
    pub fn assign_single_texture_layouts(&mut self) -> Result<(), ClippingLayoutError> {
        let using_clip_count = self.contexts.len();
        if using_clip_count > 36 {
            return Err(ClippingLayoutError::TooManyMasksForSingleTexture {
                mask_count: using_clip_count,
            });
        }

        let div = using_clip_count / 4;
        let rem = using_clip_count % 4;
        let mut context_index = 0;

        for channel_index in 0..4 {
            let layout_count = div + usize::from(channel_index < rem);
            let channel = MaskChannel::from_index(channel_index).expect("valid RGBA channel");

            for layout_index in 0..layout_count {
                self.contexts[context_index].layout = Some(ClippingLayout::new(
                    channel,
                    clipping_layout_bounds(layout_index, layout_count),
                ));
                context_index += 1;
            }
        }

        Ok(())
    }

    /// Assigns mask layouts and computes mask/draw matrices for each context.
    pub fn prepare_single_texture_masks(
        &mut self,
        drawables: &[DrawableInfo],
    ) -> Result<(), ClippingLayoutError> {
        self.prepare_single_texture_masks_from_bounds(|drawable_index| {
            drawables.get(drawable_index).and_then(DrawableInfo::bounds)
        })
    }

    pub(crate) fn prepare_single_texture_masks_from_bounds(
        &mut self,
        mut drawable_bounds: impl FnMut(usize) -> Option<ClippingRect>,
    ) -> Result<(), ClippingLayoutError> {
        self.assign_single_texture_layouts()?;

        for context_index in 0..self.contexts.len() {
            let layout = self.contexts[context_index]
                .layout
                .ok_or(ClippingLayoutError::MissingLayout { context_index })?;
            let bounds = clipped_draw_total_bounds_from(
                self.contexts[context_index].drawable_indices(),
                &mut drawable_bounds,
            )?
            .ok_or(ClippingLayoutError::DegenerateClippedBounds { context_index })?
            .expanded(0.05);
            let (matrix_for_mask, matrix_for_draw) = clipping_matrices(bounds, layout.bounds())
                .ok_or(ClippingLayoutError::DegenerateClippedBounds { context_index })?;

            self.contexts[context_index].all_clipped_draw_rect = Some(bounds);
            self.contexts[context_index].matrix_for_mask = Some(matrix_for_mask);
            self.contexts[context_index].matrix_for_draw = Some(matrix_for_draw);
        }

        Ok(())
    }
}

fn clipping_layout_bounds(layout_index: usize, layout_count: usize) -> ClippingRect {
    match layout_count {
        0 => ClippingRect::new(0.0, 0.0, 0.0, 0.0),
        1 => ClippingRect::new(0.0, 0.0, 1.0, 1.0),
        2 => ClippingRect::new(layout_index as f32 * 0.5, 0.0, 0.5, 1.0),
        3 | 4 => {
            let xpos = layout_index % 2;
            let ypos = layout_index / 2;
            ClippingRect::new(xpos as f32 * 0.5, ypos as f32 * 0.5, 0.5, 0.5)
        }
        5..=9 => {
            let xpos = layout_index % 3;
            let ypos = layout_index / 3;
            ClippingRect::new(xpos as f32 / 3.0, ypos as f32 / 3.0, 1.0 / 3.0, 1.0 / 3.0)
        }
        _ => unreachable!("single texture channel layouts are capped at 9 cells"),
    }
}

fn clipped_draw_total_bounds_from(
    drawable_indices: &[usize],
    drawable_bounds: &mut impl FnMut(usize) -> Option<ClippingRect>,
) -> Result<Option<ClippingRect>, ClippingLayoutError> {
    let mut bounds: Option<ClippingRect> = None;

    for &drawable_index in drawable_indices {
        let drawable_bounds = drawable_bounds(drawable_index)
            .ok_or(ClippingLayoutError::MissingDrawableBounds { drawable_index })?;

        bounds = Some(match bounds {
            Some(bounds) => bounds.union(drawable_bounds),
            None => drawable_bounds,
        });
    }

    Ok(bounds)
}

fn clipping_matrices(bounds: ClippingRect, layout: ClippingRect) -> Option<(Matrix44, Matrix44)> {
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
