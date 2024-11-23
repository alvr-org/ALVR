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

override ENABLE_FFE: bool = false;

override VIEW_WIDTH_RATIO: f32 = 0.;
override VIEW_HEIGHT_RATIO: f32 = 0.;
override EDGE_X_RATIO: f32 = 0.;
override EDGE_Y_RATIO: f32 = 0.;

override C1_X: f32 = 0.;
override C1_Y: f32 = 0.;
override C2_X: f32 = 0.;
override C2_Y: f32 = 0.;
override LO_BOUND_X: f32 = 0.;
override LO_BOUND_Y: f32 = 0.;
override HI_BOUND_X: f32 = 0.;
override HI_BOUND_Y: f32 = 0.;

override A_LEFT_X: f32 = 0.;
override A_LEFT_Y: f32 = 0.;
override B_LEFT_X: f32 = 0.;
override B_LEFT_Y: f32 = 0.;

override A_RIGHT_X: f32 = 0.;
override A_RIGHT_Y: f32 = 0.;
override B_RIGHT_X: f32 = 0.;
override B_RIGHT_Y: f32 = 0.;
override C_RIGHT_X: f32 = 0.;
override C_RIGHT_Y: f32 = 0.;

override COLOR_ALPHA: f32 = 1.0;

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
}

@group(0) @binding(0) var stream_texture: texture_2d<f32>;
@group(0) @binding(1) var stream_sampler: sampler;

var<push_constant> view_idx: u32;

@vertex
fn vertex_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var result: VertexOutput;

    let screen_uv = vec2f(f32(vertex_index & 1), f32(vertex_index >> 1));
    result.position = vec4f((screen_uv - 0.5) * 2.0, 0.0, 1.0);
    result.uv = vec2f(screen_uv.x, screen_uv.y);

    return result;
}

@fragment
fn fragment_main(@location(0) uv: vec2f) -> @location(0) vec4f {
    var corrected_uv = uv;
    if ENABLE_FFE {
        let view_size_ratio = vec2f(VIEW_WIDTH_RATIO, VIEW_HEIGHT_RATIO);
        let edge_ratio = vec2f(EDGE_X_RATIO, EDGE_Y_RATIO);

        let c1 = vec2f(C1_X, C1_Y);
        let c2 = vec2f(C2_X, C2_Y);
        let lo_bound = vec2f(LO_BOUND_X, LO_BOUND_Y);
        let hi_bound = vec2f(HI_BOUND_X, HI_BOUND_Y);

        let a_left = vec2f(A_LEFT_X, A_LEFT_Y);
        let b_left = vec2f(B_LEFT_X, B_LEFT_Y);

        let a_right = vec2f(A_RIGHT_X, A_RIGHT_Y);
        let b_right = vec2f(B_RIGHT_X, B_RIGHT_Y);
        let c_right = vec2f(C_RIGHT_X, C_RIGHT_Y);

        if view_idx == 1 {
            corrected_uv.x = 1.0 - corrected_uv.x;
        }

        let center = (corrected_uv - c1) * edge_ratio / c2;
        let left_edge = (-b_left + sqrt(b_left * b_left + 4.0 * a_left * corrected_uv)) / (2.0 * a_left);
        let right_edge = (-b_right + sqrt(b_right * b_right - 4.0 * (c_right - a_right * corrected_uv))) / (2.0 * a_right);

        if corrected_uv.x < lo_bound.x {
            corrected_uv.x = left_edge.x;
        } else if corrected_uv.x > hi_bound.x {
            corrected_uv.x = right_edge.x;
        } else {
            corrected_uv.x = center.x;
        }

        if corrected_uv.y < lo_bound.y {
            corrected_uv.y = left_edge.y;
        } else if corrected_uv.y > hi_bound.y {
            corrected_uv.y = right_edge.y;
        } else {
            corrected_uv.y = center.y;
        }

        corrected_uv = corrected_uv * view_size_ratio;

        if view_idx == 1 {
            corrected_uv.x = 1.0 - corrected_uv.x;
        }
    }

    var color = textureSample(stream_texture, stream_sampler, corrected_uv).rgb;

    if FIX_LIMITED_RANGE {
        // For some reason, the encoder shifts full-range color into the negatives and over one.
        color = LIMITED_MIN + ((LIMITED_MAX - LIMITED_MIN) * color);
    }

    if ENABLE_SRGB_CORRECTION {
        let condition = vec3f(f32(color.r < THRESHOLD), f32(color.g < THRESHOLD), f32(color.b < THRESHOLD));
        let lowValues = color * DIV12;
        let highValues = pow((color + vec3f(0.055)) * DIV1, GAMMA);
        color = condition * lowValues + (1.0 - condition) * highValues;
    }

    if ENCODING_GAMMA != 0.0 {
        let enc_condition = vec3f(f32(color.r < 0.0), f32(color.g < 0.0), f32(color.b < 0.0));
        let enc_lowValues = color;
        let enc_highValues = pow(color, vec3f(ENCODING_GAMMA));
        color = enc_condition * enc_lowValues + (1.0 - enc_condition) * enc_highValues;
    }

    return vec4f(color, COLOR_ALPHA);
}
