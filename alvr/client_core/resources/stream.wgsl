// todo: use expression directly when supported in naga
const DIV12: f32 = 0.0773993808;// 1.0 / 12.92
const DIV1: f32 = 0.94786729857; // 1.0 / 1.055
const THRESHOLD: f32 = 0.04045;
const GAMMA: vec3f = vec3f(2.4);

// Convert from limited colors to full
const LIMITED_MIN: f32 = 0.06274509803; // 16.0 / 255.0
const LIMITED_MAX: f32 = 0.92156862745; // 235.0 / 255.0

override FIX_LIMITED_RANGE: bool;
override ENABLE_SRGB_CORRECTION: bool;
override ENCODING_GAMMA: f32;

var<push_constant> view_idx: u32;

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
}

@group(0) @binding(0) var stream_texture: texture_2d<f32>;
@group(0) @binding(1) var stream_sampler: sampler;

@vertex
fn vertex_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var result: VertexOutput;

    let screen_uv = vec2f(f32(vertex_index & 1), f32(vertex_index >> 1));
    result.position = vec4f((screen_uv - vec2f(0.5, 0.5)) * 2.0, 0.0, 1.0);
    result.uv = vec2f((screen_uv.x + f32(view_idx)) / 2.0, screen_uv.y);

    return result;
}

@fragment
fn fragment_main(@location(0) uv: vec2f) -> @location(0) vec4f {
    var result: vec3f = textureSample(stream_texture, stream_sampler, uv).rgb;

    if FIX_LIMITED_RANGE {
        // For some reason, the encoder shifts full-range color into the negatives and over one.
        result = LIMITED_MIN + ((LIMITED_MAX - LIMITED_MIN) * result);
    }

    if ENABLE_SRGB_CORRECTION {
        let condition = vec3f(f32(result.r < THRESHOLD), f32(result.g < THRESHOLD), f32(result.b < THRESHOLD));
        let lowValues = result * DIV12;
        let highValues = pow((result + vec3f(0.055)) * DIV1, GAMMA);
        result = condition * lowValues + (1.0 - condition) * highValues;
    }

    if ENCODING_GAMMA != 0.0 {
        let enc_condition = vec3f(f32(result.r < 0.0), f32(result.g < 0.0), f32(result.b < 0.0));
        let enc_lowValues = result;
        let enc_highValues = pow(result, vec3f(ENCODING_GAMMA));
        result = enc_condition * enc_lowValues + (1.0 - enc_condition) * enc_highValues;
    }

    return vec4f(result, 1.0);
}