struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) opacity: f32,
    @location(3) multiply: vec3<f32>,
    @location(4) screen: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) opacity: f32,
    @location(2) clip_position: vec4<f32>,
};

struct MaskParams {
    channel_flag: vec4<f32>,
    base_rect: vec4<f32>,
};

@group(0) @binding(0)
var live2d_texture: texture_2d<f32>;

@group(0) @binding(1)
var live2d_sampler: sampler;

@group(1) @binding(0)
var<uniform> live2d_transform: mat4x4<f32>;

@group(2) @binding(0)
var<uniform> mask_params: MaskParams;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    let clip_position = live2d_transform * vec4<f32>(input.position, 0.0, 1.0);

    var output: VertexOutput;
    output.position = clip_position;
    output.uv = input.uv;
    output.opacity = input.opacity;
    output.clip_position = clip_position;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let pos = input.clip_position.xy / input.clip_position.w;
    let inside = step(mask_params.base_rect.x, pos.x)
        * step(mask_params.base_rect.y, pos.y)
        * step(pos.x, mask_params.base_rect.z)
        * step(pos.y, mask_params.base_rect.w);
    let texture_alpha = textureSample(live2d_texture, live2d_sampler, input.uv).a;
    let source_alpha = texture_alpha * inside;
    return mask_params.channel_flag * source_alpha;
}
