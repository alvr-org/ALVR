// Draws a decoded video frame as a fullscreen quad, converting YUV to RGB on the GPU.
//
// The three planes arrive as separate single-channel textures because that is how the decoder
// produces them, and converting here avoids a CPU colour-space pass per frame.

struct Uniforms {
    // Sub-rectangle of the frame to show, as (offset_x, offset_y, scale_x, scale_y) in UV space.
    // This is how one eye is selected out of a side-by-side stereo frame.
    region: vec4<f32>,
    // 1.0 when the samples cover the full 0-255 range, 0.0 for the limited broadcast range.
    full_range: f32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var plane_y: texture_2d<f32>;
@group(0) @binding(2) var plane_u: texture_2d<f32>;
@group(0) @binding(3) var plane_v: texture_2d<f32>;
@group(0) @binding(4) var plane_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// Two triangles covering the viewport, generated from the vertex index so no vertex buffer is
// needed.
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
    );

    let position = positions[index];

    var out: VertexOutput;
    out.clip_position = vec4<f32>(position, 0.0, 1.0);
    // Clip space has Y up, texture space has Y down.
    let uv = vec2<f32>(position.x * 0.5 + 0.5, 0.5 - position.y * 0.5);
    out.uv = uniforms.region.xy + uv * uniforms.region.zw;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let y = textureSample(plane_y, plane_sampler, in.uv).r;
    let u = textureSample(plane_u, plane_sampler, in.uv).r;
    let v = textureSample(plane_v, plane_sampler, in.uv).r;

    // Limited range puts luma in 16-235 and chroma in 16-240, so both need rescaling before the
    // matrix; full range uses 0-255 directly and only chroma needs centring. Mixing the two up
    // produces a washed out or crushed image rather than an error, so the decoder reports which.
    let luma_offset = mix(0.0, 16.0 / 255.0, 1.0 - uniforms.full_range);
    let luma_scale = mix(1.0, 255.0 / 219.0, 1.0 - uniforms.full_range);
    let chroma_scale = mix(1.0, 255.0 / 224.0, 1.0 - uniforms.full_range);

    let luma = (y - luma_offset) * luma_scale;
    let chroma_u = (u - 128.0 / 255.0) * chroma_scale;
    let chroma_v = (v - 128.0 / 255.0) * chroma_scale;

    let rgb = vec3<f32>(
        luma + 1.5748 * chroma_v,
        luma - 0.1873 * chroma_u - 0.4681 * chroma_v,
        luma + 1.8556 * chroma_u,
    );

    return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
