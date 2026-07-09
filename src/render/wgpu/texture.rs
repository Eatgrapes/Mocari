use wgpu::util::DeviceExt;

use crate::core::Matrix44;

use crate::render::common::{ClippingLayout as WgpuClippingLayout, MaskChannel as WgpuMaskChannel};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WgpuTextureError {
    #[error("invalid texture size {width}x{height}")]
    InvalidTextureSize { width: u32, height: u32 },
    #[error("invalid rgba8 texture length for {width}x{height}: expected {expected}, got {actual}")]
    InvalidRgbaLength {
        width: u32,
        height: u32,
        expected: usize,
        actual: usize,
    },
}

#[derive(Debug)]
pub struct WgpuTexture {
    pub(super) texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) bind_group: wgpu::BindGroup,
    pub(super) width: u32,
    pub(super) height: u32,
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
pub struct WgpuTransform {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    bytes: Vec<u8>,
}

impl WgpuTransform {
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn update_matrix(&mut self, queue: &wgpu::Queue, matrix: &Matrix44) -> bool {
        update_uniform_bytes(
            queue,
            &self.buffer,
            &mut self.bytes,
            encode_wgpu_matrix(matrix),
        )
    }
}

#[derive(Debug)]
pub struct WgpuMaskParams {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    bytes: Vec<u8>,
}

impl WgpuMaskParams {
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn update_layout(&mut self, queue: &wgpu::Queue, layout: WgpuClippingLayout) -> bool {
        update_uniform_bytes(
            queue,
            &self.buffer,
            &mut self.bytes,
            encode_wgpu_mask_params(layout),
        )
    }
}

#[derive(Debug)]
pub struct WgpuClipParams {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    bytes: Vec<u8>,
}

impl WgpuClipParams {
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn update_params(
        &mut self,
        queue: &wgpu::Queue,
        matrix: &Matrix44,
        channel: WgpuMaskChannel,
        inverted: bool,
    ) -> bool {
        update_uniform_bytes(
            queue,
            &self.buffer,
            &mut self.bytes,
            encode_wgpu_clip_params(matrix, channel, inverted),
        )
    }
}

fn update_uniform_bytes(
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    current: &mut Vec<u8>,
    next: Vec<u8>,
) -> bool {
    if *current == next {
        return false;
    }

    queue.write_buffer(buffer, 0, &next);
    *current = next;
    true
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
        bounds.x() * 2.0 - 1.0,
        bounds.y() * 2.0 - 1.0,
        (bounds.x() + bounds.width()) * 2.0 - 1.0,
        (bounds.y() + bounds.height()) * 2.0 - 1.0,
    ];
    let mut bytes = Vec::with_capacity(32);

    for value in layout.channel_flag().into_iter().chain(base_rect) {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }

    bytes
}

pub fn encode_wgpu_clip_params(
    matrix: &Matrix44,
    channel: WgpuMaskChannel,
    inverted: bool,
) -> Vec<u8> {
    let mut bytes = encode_wgpu_matrix(matrix);
    let inverted_flag = [if inverted { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0];
    for value in channel.flag().into_iter().chain(inverted_flag) {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }

    bytes
}

pub(super) fn create_wgpu_mask_params(
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

    WgpuMaskParams {
        buffer,
        bind_group,
        bytes,
    }
}

pub(super) fn create_wgpu_clip_params(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    matrix: &Matrix44,
    channel: WgpuMaskChannel,
    inverted: bool,
) -> WgpuClipParams {
    let bytes = encode_wgpu_clip_params(matrix, channel, inverted);
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("live2d.clip.params.uniform"),
        contents: &bytes,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("live2d.clip.params.bind_group"),
        layout: bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    });

    WgpuClipParams {
        buffer,
        bind_group,
        bytes,
    }
}

pub(super) fn create_wgpu_transform(
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

    WgpuTransform {
        buffer,
        bind_group,
        bytes: matrix_bytes,
    }
}

pub(super) fn rgba8_len(width: u32, height: u32) -> Result<usize, WgpuTextureError> {
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
