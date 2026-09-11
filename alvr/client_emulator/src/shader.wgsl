// Unlit scene shader. Lighting and shadows are expected to be baked into the base colour texture,
// so the fragment stage is a plain texture fetch modulated by the material colour factor.

struct Uniforms {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var base_color: texture_2d<f32>;
@group(0) @binding(2) var base_color_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.view_proj * vec4<f32>(in.position, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

// Untextured materials bind a 1x1 white placeholder, so this single path covers both cases: the
// material colour factor arrives baked into in.color and multiplying by white leaves it unchanged.
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let sampled = textureSample(base_color, base_color_sampler, in.uv);
    return vec4<f32>(in.color * sampled.rgb, 1.0);
}
