struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) opacity: f32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) opacity: f32,
    @location(2) clip_position: vec4<f32>,
};

struct ClipParams {
    clip_matrix: mat4x4<f32>,
    channel_flag: vec4<f32>,
};

@group(0) @binding(0)
var live2d_texture: texture_2d<f32>;

@group(0) @binding(1)
var live2d_sampler: sampler;

@group(1) @binding(0)
var<uniform> live2d_transform: mat4x4<f32>;

@group(2) @binding(0)
var live2d_mask_texture: texture_2d<f32>;

@group(2) @binding(1)
var live2d_mask_sampler: sampler;

@group(3) @binding(0)
var<uniform> clip_params: ClipParams;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    let position = vec4<f32>(input.position, 0.0, 1.0);

    var output: VertexOutput;
    output.position = live2d_transform * position;
    output.uv = input.uv;
    output.opacity = input.opacity;
    output.clip_position = clip_params.clip_matrix * position;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let sample = textureSample(live2d_texture, live2d_sampler, input.uv);
    let alpha = sample.a * input.opacity;
    let color = vec4<f32>(sample.rgb * alpha, alpha);
    let mask_uv = input.clip_position.xy / input.clip_position.w;
    let mask_sample = textureSample(live2d_mask_texture, live2d_mask_sampler, mask_uv);
    let mask_value = dot(vec4<f32>(1.0) - mask_sample, clip_params.channel_flag);
    return color * mask_value;
}
