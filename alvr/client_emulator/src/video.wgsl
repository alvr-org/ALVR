// Draws a decoded video frame as a fullscreen quad, converting YUV to RGB on the GPU.
//
// The three planes arrive as separate single-channel textures because that is how the decoder
// produces them, and converting here avoids a CPU colour-space pass per frame.
//
// Passthrough is applied here the same way the headset's stream shader (alvr/graphics) does it, so
// the video composites over the local scene as it would over the real world. The output is always
// premultiplied and blended with (one, one minus source alpha); an opaque stream has alpha 1 and
// simply replaces what is behind it.

struct Uniforms {
    // Sub-rectangle of the frame to show, as (offset_x, offset_y, scale_x, scale_y) in UV space.
    // This is how one eye is selected out of a side-by-side stereo frame.
    region: vec4<f32>,
    // 1.0 when the samples cover the full 0-255 range, 0.0 for the limited broadcast range.
    full_range: f32,
    // The same for the alpha plane, which is a separate stream and carries its own range.
    alpha_full_range: f32,
    // 0: none or Blend (constant alpha), 1: RGB chroma key, 2: HSV chroma key, 3: alpha stream.
    passthrough_mode: u32,
    // Alpha for mode 0: 1.0 for an opaque stream, 1 - threshold for Blend.
    blend_alpha: f32,
    // Chroma key ranges per channel, as (start_max, start_min, end_min, end_max).
    ck_channel0: vec4<f32>,
    ck_channel1: vec4<f32>,
    ck_channel2: vec4<f32>,
    // 1 when the color is already multiplied by alpha, as the headset's layer flag would say.
    premultiplied: u32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var plane_y: texture_2d<f32>;
@group(0) @binding(2) var plane_u: texture_2d<f32>;
@group(0) @binding(3) var plane_v: texture_2d<f32>;
@group(0) @binding(4) var plane_sampler: sampler;
// Luma of the alpha stream. A 1x1 white placeholder until that stream delivers a frame.
@group(0) @binding(5) var plane_alpha: texture_2d<f32>;

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

    var color = clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    var alpha = uniforms.blend_alpha;

    if uniforms.passthrough_mode == 3u {
        // The alpha plane is encoded as luma, so it needs the same range expansion as color luma.
        let a = textureSample(plane_alpha, plane_sampler, in.uv).r;
        let a_offset = mix(0.0, 16.0 / 255.0, 1.0 - uniforms.alpha_full_range);
        let a_scale = mix(1.0, 255.0 / 219.0, 1.0 - uniforms.alpha_full_range);
        alpha = clamp((a - a_offset) * a_scale, 0.0, 1.0);
    } else if uniforms.passthrough_mode == 1u || uniforms.passthrough_mode == 2u {
        var current = color;
        if uniforms.passthrough_mode == 2u {
            current = rgb_to_hsv(color);
        }
        let mask = chroma_key_mask(current);

        // Premultiplied by construction, which is why the headset always flags chroma key layers
        // as premultiplied.
        color = max(color * mask, vec3<f32>(0.0));
        alpha = mask;
    }

    if uniforms.premultiplied == 0u {
        color = color * alpha;
    }

    return vec4<f32>(color, alpha);
}

// Copied from alvr/graphics/resources/stream.wgsl, with the push constants replaced by uniforms.
fn chroma_key_mask(color: vec3<f32>) -> f32 {
    let start_max = vec3<f32>(uniforms.ck_channel0.x, uniforms.ck_channel1.x, uniforms.ck_channel2.x);
    let start_min = vec3<f32>(uniforms.ck_channel0.y, uniforms.ck_channel1.y, uniforms.ck_channel2.y);
    let end_min = vec3<f32>(uniforms.ck_channel0.z, uniforms.ck_channel1.z, uniforms.ck_channel2.z);
    let end_max = vec3<f32>(uniforms.ck_channel0.w, uniforms.ck_channel1.w, uniforms.ck_channel2.w);

    let start_mask = smoothstep(start_min.yz, start_max.yz, color.yz);
    let end_mask = smoothstep(end_min.yz, end_max.yz, color.yz);
    let sv_mask = max(start_mask, end_mask);
    // create t1 < t2 < t3, based on the rotated hue starting with start_max.x representing 0.
    let t3 = end_max.x - start_max.x;
    // check hue range
    if t3 > 1.0 {
        return max(sv_mask.x, sv_mask.y);
    }
    let x = fract(color.x - start_max.x);
    let t1 = fract(start_min.x - start_max.x);
    let t2 = fract(end_min.x - start_max.x);

    return max(max(smoothstep(t1, 0.0, x), smoothstep(t2, t3, x)), max(sv_mask.x, sv_mask.y));
}

fn rgb_to_hsv(rgb: vec3<f32>) -> vec3<f32> {
    let cmax = max(rgb.r, max(rgb.g, rgb.b));
    let cmin = min(rgb.r, min(rgb.g, rgb.b));
    let delta = cmax - cmin;

    var h = 0.0;
    var s = 0.0;
    let v = cmax;

    if cmax > cmin {
        s = delta / cmax;

        if rgb.r == cmax {
            h = (rgb.g - rgb.b) / delta;
        } else if rgb.g == cmax {
            h = 2.0 + (rgb.b - rgb.r) / delta;
        } else {
            h = 4.0 + (rgb.r - rgb.g) / delta;
        }
        h = fract(h / 6.0);
    }

    return vec3<f32>(h, s, v);
}
