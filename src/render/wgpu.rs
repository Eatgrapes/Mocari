use wgpu::util::DeviceExt;

use crate::{
    core::Matrix44,
    moc3::{Moc3DrawableBlendMode, Moc3DrawableMesh, Moc3DrawableVertex},
};

pub fn live2d_wgsl_source() -> &'static str {
    include_str!("shaders/live2d.wgsl")
}

pub fn live2d_masked_wgsl_source() -> &'static str {
    include_str!("shaders/live2d_masked.wgsl")
}

pub fn mask_wgsl_source() -> &'static str {
    include_str!("shaders/mask.wgsl")
}

pub fn live2d_blend_state(blend_mode: Moc3DrawableBlendMode) -> wgpu::BlendState {
    match blend_mode {
        Moc3DrawableBlendMode::Normal => wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        },
        Moc3DrawableBlendMode::Additive => wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        },
        Moc3DrawableBlendMode::Multiplicative => wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Dst,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        },
    }
}

pub fn wgpu_mask_blend_state() -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Zero,
            dst_factor: wgpu::BlendFactor::OneMinusSrc,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Zero,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct WgpuDrawableVertex {
    position: [f32; 2],
    uv: [f32; 2],
    opacity: f32,
}

impl WgpuDrawableVertex {
    pub const STRIDE: wgpu::BufferAddress = 20;

    pub fn new(position: [f32; 2], uv: [f32; 2], opacity: f32) -> Self {
        Self {
            position,
            uv,
            opacity,
        }
    }

    pub fn position(&self) -> [f32; 2] {
        self.position
    }

    pub fn uv(&self) -> [f32; 2] {
        self.uv
    }

    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    pub fn buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 3] = [
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 8,
                shader_location: 1,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32,
                offset: 16,
                shader_location: 2,
            },
        ];

        wgpu::VertexBufferLayout {
            array_stride: Self::STRIDE,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES,
        }
    }
}

#[derive(Debug)]
pub struct WgpuDrawableBuffers {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    texture_index: i32,
    blend_mode: Moc3DrawableBlendMode,
    opacity: f32,
    draw_order: f32,
    masks: Vec<i32>,
    bounds: Option<WgpuClippingRect>,
}

impl WgpuDrawableBuffers {
    pub fn vertex_buffer(&self) -> &wgpu::Buffer {
        &self.vertex_buffer
    }

    pub fn index_buffer(&self) -> &wgpu::Buffer {
        &self.index_buffer
    }

    pub fn index_count(&self) -> u32 {
        self.index_count
    }

    pub fn texture_index(&self) -> i32 {
        self.texture_index
    }

    pub fn blend_mode(&self) -> Moc3DrawableBlendMode {
        self.blend_mode
    }

    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    pub fn draw_order(&self) -> f32 {
        self.draw_order
    }

    pub fn masks(&self) -> &[i32] {
        &self.masks
    }

    pub fn bounds(&self) -> Option<WgpuClippingRect> {
        self.bounds
    }
}

#[derive(Debug)]
pub struct WgpuMeshBuffers {
    drawables: Vec<WgpuDrawableBuffers>,
}

impl WgpuMeshBuffers {
    pub fn from_drawables(device: &wgpu::Device, meshes: &[Moc3DrawableMesh]) -> Option<Self> {
        let mut drawables = Vec::with_capacity(meshes.len());
        for mesh in meshes {
            drawables.push(create_wgpu_drawable_buffers(device, mesh)?);
        }

        Some(Self { drawables })
    }

    pub fn drawables(&self) -> &[WgpuDrawableBuffers] {
        &self.drawables
    }

    pub fn draw_order_indices(&self) -> Vec<usize> {
        let mut indices = (0..self.drawables.len()).collect::<Vec<_>>();
        indices.sort_by(|left, right| {
            self.drawables[*left]
                .draw_order
                .total_cmp(&self.drawables[*right].draw_order)
                .then_with(|| left.cmp(right))
        });
        indices
    }
}

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
    let draw_translate_y = -bounds.y * scale_y + layout.y;

    let mut matrix_for_draw = Matrix44::identity();
    matrix_for_draw.scale(scale_x, scale_y);
    matrix_for_draw.translate(draw_translate_x, draw_translate_y);

    let mut matrix_for_mask = Matrix44::identity();
    matrix_for_mask.scale(scale_x * 2.0, scale_y * 2.0);
    matrix_for_mask.translate(draw_translate_x * 2.0 - 1.0, draw_translate_y * 2.0 - 1.0);

    Some((matrix_for_mask, matrix_for_draw))
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WgpuTextureError {
    InvalidTextureSize {
        width: u32,
        height: u32,
    },
    InvalidRgbaLength {
        width: u32,
        height: u32,
        expected: usize,
        actual: usize,
    },
}

impl std::fmt::Display for WgpuTextureError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTextureSize { width, height } => {
                write!(formatter, "invalid texture size {width}x{height}")
            }
            Self::InvalidRgbaLength {
                width,
                height,
                expected,
                actual,
            } => write!(
                formatter,
                "invalid rgba8 texture length for {width}x{height}: expected {expected}, got {actual}"
            ),
        }
    }
}

impl std::error::Error for WgpuTextureError {}

#[derive(Debug)]
pub struct WgpuTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
}

impl WgpuTexture {
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
pub struct WgpuMaskRenderTarget {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
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
pub struct WgpuTransform {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl WgpuTransform {
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

#[derive(Debug)]
pub struct WgpuMaskParams {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl WgpuMaskParams {
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

#[derive(Debug)]
pub struct WgpuPreparedClippingContext {
    mask_drawable_indices: Vec<usize>,
    mask_transform: WgpuTransform,
    mask_params: WgpuMaskParams,
}

impl WgpuPreparedClippingContext {
    pub fn mask_drawable_indices(&self) -> &[usize] {
        &self.mask_drawable_indices
    }

    pub fn mask_transform(&self) -> &WgpuTransform {
        &self.mask_transform
    }

    pub fn mask_params(&self) -> &WgpuMaskParams {
        &self.mask_params
    }
}

#[derive(Debug)]
pub struct WgpuClippingResources {
    contexts: Vec<WgpuPreparedClippingContext>,
}

impl WgpuClippingResources {
    pub fn contexts(&self) -> &[WgpuPreparedClippingContext] {
        &self.contexts
    }
}

#[derive(Debug)]
pub struct WgpuLive2dRenderer {
    normal_pipeline: wgpu::RenderPipeline,
    additive_pipeline: wgpu::RenderPipeline,
    multiplicative_pipeline: wgpu::RenderPipeline,
    mask_pipeline: wgpu::RenderPipeline,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    transform_bind_group_layout: wgpu::BindGroupLayout,
    mask_params_bind_group_layout: wgpu::BindGroupLayout,
    identity_transform: WgpuTransform,
    sampler: wgpu::Sampler,
}

impl WgpuLive2dRenderer {
    pub fn new(device: &wgpu::Device, color_format: wgpu::TextureFormat) -> Self {
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("live2d.texture.bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let transform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("live2d.transform.bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(64),
                    },
                    count: None,
                }],
            });
        let mask_params_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("live2d.mask.params.bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(32),
                    },
                    count: None,
                }],
            });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("live2d.texture.sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("live2d.shader"),
            source: wgpu::ShaderSource::Wgsl(live2d_wgsl_source().into()),
        });
        let mask_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("live2d.mask.shader"),
            source: wgpu::ShaderSource::Wgsl(mask_wgsl_source().into()),
        });
        let bind_group_layouts = [
            Some(&texture_bind_group_layout),
            Some(&transform_bind_group_layout),
        ];
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("live2d.pipeline.layout"),
            bind_group_layouts: &bind_group_layouts,
            immediate_size: 0,
        });
        let normal_pipeline = create_live2d_pipeline(
            device,
            &pipeline_layout,
            &shader,
            color_format,
            Moc3DrawableBlendMode::Normal,
            "live2d.pipeline.normal",
        );
        let additive_pipeline = create_live2d_pipeline(
            device,
            &pipeline_layout,
            &shader,
            color_format,
            Moc3DrawableBlendMode::Additive,
            "live2d.pipeline.additive",
        );
        let multiplicative_pipeline = create_live2d_pipeline(
            device,
            &pipeline_layout,
            &shader,
            color_format,
            Moc3DrawableBlendMode::Multiplicative,
            "live2d.pipeline.multiplicative",
        );
        let mask_bind_group_layouts = [
            Some(&texture_bind_group_layout),
            Some(&transform_bind_group_layout),
            Some(&mask_params_bind_group_layout),
        ];
        let mask_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("live2d.mask.pipeline.layout"),
            bind_group_layouts: &mask_bind_group_layouts,
            immediate_size: 0,
        });
        let mask_pipeline = create_live2d_mask_pipeline(
            device,
            &mask_pipeline_layout,
            &mask_shader,
            "live2d.mask.pipeline",
        );
        let identity_transform =
            create_wgpu_transform(device, &transform_bind_group_layout, &Matrix44::identity());

        Self {
            normal_pipeline,
            additive_pipeline,
            multiplicative_pipeline,
            mask_pipeline,
            texture_bind_group_layout,
            transform_bind_group_layout,
            mask_params_bind_group_layout,
            identity_transform,
            sampler,
        }
    }

    pub fn pipeline(&self) -> &wgpu::RenderPipeline {
        self.pipeline_for_blend_mode(Moc3DrawableBlendMode::Normal)
    }

    pub fn pipeline_for_blend_mode(
        &self,
        blend_mode: Moc3DrawableBlendMode,
    ) -> &wgpu::RenderPipeline {
        match blend_mode {
            Moc3DrawableBlendMode::Normal => &self.normal_pipeline,
            Moc3DrawableBlendMode::Additive => &self.additive_pipeline,
            Moc3DrawableBlendMode::Multiplicative => &self.multiplicative_pipeline,
        }
    }

    pub fn mask_pipeline(&self) -> &wgpu::RenderPipeline {
        &self.mask_pipeline
    }

    pub fn texture_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.texture_bind_group_layout
    }

    pub fn transform_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.transform_bind_group_layout
    }

    pub fn mask_params_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.mask_params_bind_group_layout
    }

    pub fn identity_transform(&self) -> &WgpuTransform {
        &self.identity_transform
    }

    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    pub fn create_texture_bind_group(
        &self,
        device: &wgpu::Device,
        texture_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("live2d.texture.bind_group"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    pub fn create_transform(&self, device: &wgpu::Device, matrix: &Matrix44) -> WgpuTransform {
        create_wgpu_transform(device, &self.transform_bind_group_layout, matrix)
    }

    pub fn create_mask_params(
        &self,
        device: &wgpu::Device,
        layout: WgpuClippingLayout,
    ) -> WgpuMaskParams {
        create_wgpu_mask_params(device, &self.mask_params_bind_group_layout, layout)
    }

    pub fn create_clipping_resources(
        &self,
        device: &wgpu::Device,
        plan: &WgpuClippingPlan,
    ) -> Result<WgpuClippingResources, WgpuClippingLayoutError> {
        let mut contexts = Vec::with_capacity(plan.contexts().len());

        for (context_index, context) in plan.contexts().iter().enumerate() {
            let layout = context
                .layout()
                .ok_or(WgpuClippingLayoutError::MissingLayout { context_index })?;
            let mask_transform = self.create_transform(
                device,
                &context
                    .matrix_for_mask()
                    .ok_or(WgpuClippingLayoutError::MissingMaskMatrix { context_index })?,
            );
            let mask_params = self.create_mask_params(device, layout);
            let mask_drawable_indices = context
                .masks()
                .iter()
                .map(|&drawable_index| {
                    usize::try_from(drawable_index).map_err(|_| {
                        WgpuClippingLayoutError::InvalidMaskDrawableIndex { drawable_index }
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;

            contexts.push(WgpuPreparedClippingContext {
                mask_drawable_indices,
                mask_transform,
                mask_params,
            });
        }

        Ok(WgpuClippingResources { contexts })
    }

    pub fn create_rgba8_texture(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<WgpuTexture, WgpuTextureError> {
        let expected = rgba8_len(width, height)?;
        if rgba.len() != expected {
            return Err(WgpuTextureError::InvalidRgbaLength {
                width,
                height,
                expected,
                actual: rgba.len(),
            });
        }

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("live2d.texture.rgba8"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        queue.write_texture(
            texture.as_image_copy(),
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.create_texture_bind_group(device, &view);

        Ok(WgpuTexture {
            texture,
            view,
            bind_group,
            width,
            height,
        })
    }

    pub fn create_mask_render_target(
        &self,
        device: &wgpu::Device,
        size: u32,
    ) -> Result<WgpuMaskRenderTarget, WgpuTextureError> {
        if size == 0 {
            return Err(WgpuTextureError::InvalidTextureSize {
                width: size,
                height: size,
            });
        }

        let extent = wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("live2d.mask.texture"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.create_texture_bind_group(device, &view);

        Ok(WgpuMaskRenderTarget {
            texture,
            view,
            bind_group,
            width: size,
            height: size,
        })
    }

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
            let drawable = &mesh_buffers.drawables[drawable_index];
            if !drawable.masks.is_empty() {
                return Err(WgpuRenderError::UnsupportedClippingMasks {
                    drawable_index,
                    mask_count: drawable.masks.len(),
                });
            }
            let texture_bind_group = bind_group_for_texture(drawable.texture_index)?;

            render_pass.set_pipeline(self.pipeline_for_blend_mode(drawable.blend_mode));
            render_pass.set_bind_group(0, texture_bind_group, &[]);
            render_pass.set_bind_group(1, transform.bind_group(), &[]);
            render_pass.set_vertex_buffer(0, drawable.vertex_buffer.slice(..));
            render_pass
                .set_index_buffer(drawable.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..drawable.index_count, 0, 0..1);
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
                let texture_bind_group = bind_group_for_texture(drawable.texture_index)?;

                render_pass.set_pipeline(&self.mask_pipeline);
                render_pass.set_bind_group(0, texture_bind_group, &[]);
                render_pass.set_bind_group(1, context.mask_transform().bind_group(), &[]);
                render_pass.set_bind_group(2, context.mask_params().bind_group(), &[]);
                render_pass.set_vertex_buffer(0, drawable.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(drawable.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..drawable.index_count, 0, 0..1);
                drawn += 1;
            }
        }

        Ok(drawn)
    }
}

pub fn wgpu_vertices_from_drawable(mesh: &Moc3DrawableMesh) -> Vec<WgpuDrawableVertex> {
    mesh.vertices()
        .iter()
        .map(|vertex| wgpu_vertex_from_drawable_vertex(vertex, mesh.opacity()))
        .collect()
}

pub fn wgpu_vertex_from_drawable_vertex(
    vertex: &Moc3DrawableVertex,
    opacity: f32,
) -> WgpuDrawableVertex {
    WgpuDrawableVertex::new(vertex.position(), vertex.uv(), opacity)
}

pub fn encode_wgpu_vertices(vertices: &[WgpuDrawableVertex]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vertices.len() * WgpuDrawableVertex::STRIDE as usize);
    for vertex in vertices {
        bytes.extend_from_slice(&vertex.position[0].to_ne_bytes());
        bytes.extend_from_slice(&vertex.position[1].to_ne_bytes());
        bytes.extend_from_slice(&vertex.uv[0].to_ne_bytes());
        bytes.extend_from_slice(&vertex.uv[1].to_ne_bytes());
        bytes.extend_from_slice(&vertex.opacity.to_ne_bytes());
    }

    bytes
}

pub fn encode_wgpu_indices(indices: &[u16]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(indices.len() * 2);
    for index in indices {
        bytes.extend_from_slice(&index.to_ne_bytes());
    }

    bytes
}

pub fn encode_wgpu_matrix(matrix: &Matrix44) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(64);
    for value in matrix.as_slice() {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }

    bytes
}

pub fn encode_wgpu_mask_params(layout: WgpuClippingLayout) -> Vec<u8> {
    let bounds = layout.bounds();
    let base_rect = [
        bounds.x * 2.0 - 1.0,
        bounds.y * 2.0 - 1.0,
        (bounds.x + bounds.width) * 2.0 - 1.0,
        (bounds.y + bounds.height) * 2.0 - 1.0,
    ];
    let mut bytes = Vec::with_capacity(32);

    for value in layout.channel_flag().into_iter().chain(base_rect) {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }

    bytes
}

pub fn create_wgpu_drawable_buffers(
    device: &wgpu::Device,
    mesh: &Moc3DrawableMesh,
) -> Option<WgpuDrawableBuffers> {
    let vertices = wgpu_vertices_from_drawable(mesh);
    let vertex_bytes = encode_wgpu_vertices(&vertices);
    let index_bytes = encode_wgpu_indices(mesh.indices());
    let index_count = u32::try_from(mesh.indices().len()).ok()?;

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("live2d.drawable.vertices"),
        contents: &vertex_bytes,
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("live2d.drawable.indices"),
        contents: &index_bytes,
        usage: wgpu::BufferUsages::INDEX,
    });

    Some(WgpuDrawableBuffers {
        vertex_buffer,
        index_buffer,
        index_count,
        texture_index: mesh.texture_index(),
        blend_mode: mesh.blend_mode(),
        opacity: mesh.opacity(),
        draw_order: mesh.draw_order(),
        masks: mesh.masks().to_vec(),
        bounds: drawable_vertex_bounds(mesh.vertices()),
    })
}

fn drawable_vertex_bounds(vertices: &[Moc3DrawableVertex]) -> Option<WgpuClippingRect> {
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

    Some(WgpuClippingRect::new(
        min_x,
        min_y,
        max_x - min_x,
        max_y - min_y,
    ))
}

fn create_wgpu_mask_params(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    layout: WgpuClippingLayout,
) -> WgpuMaskParams {
    let bytes = encode_wgpu_mask_params(layout);
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("live2d.mask.params.uniform"),
        contents: &bytes,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("live2d.mask.params.bind_group"),
        layout: bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    });

    WgpuMaskParams { buffer, bind_group }
}

fn create_wgpu_transform(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    matrix: &Matrix44,
) -> WgpuTransform {
    let matrix_bytes = encode_wgpu_matrix(matrix);
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("live2d.transform.uniform"),
        contents: &matrix_bytes,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("live2d.transform.bind_group"),
        layout: bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    });

    WgpuTransform { buffer, bind_group }
}

fn rgba8_len(width: u32, height: u32) -> Result<usize, WgpuTextureError> {
    if width == 0 || height == 0 {
        return Err(WgpuTextureError::InvalidTextureSize { width, height });
    }

    let len = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(WgpuTextureError::InvalidTextureSize { width, height })?;

    Ok(len)
}

fn create_live2d_pipeline(
    device: &wgpu::Device,
    pipeline_layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    color_format: wgpu::TextureFormat,
    blend_mode: Moc3DrawableBlendMode,
    label: &'static str,
) -> wgpu::RenderPipeline {
    let vertex_buffers = [WgpuDrawableVertex::buffer_layout()];
    let color_targets = [Some(wgpu::ColorTargetState {
        format: color_format,
        blend: Some(live2d_blend_state(blend_mode)),
        write_mask: wgpu::ColorWrites::ALL,
    })];

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &vertex_buffers,
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &color_targets,
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn create_live2d_mask_pipeline(
    device: &wgpu::Device,
    pipeline_layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    label: &'static str,
) -> wgpu::RenderPipeline {
    let vertex_buffers = [WgpuDrawableVertex::buffer_layout()];
    let color_targets = [Some(wgpu::ColorTargetState {
        format: wgpu::TextureFormat::Rgba8Unorm,
        blend: Some(wgpu_mask_blend_state()),
        write_mask: wgpu::ColorWrites::ALL,
    })];

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &vertex_buffers,
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &color_targets,
        }),
        multiview_mask: None,
        cache: None,
    })
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
