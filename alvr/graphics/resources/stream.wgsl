// todo: use expression directly when supported in naga
const DIV12: f32 = 0.0773993808;// 1.0 / 12.92
const DIV1: f32 = 0.94786729857; // 1.0 / 1.055
const THRESHOLD: f32 = 0.04045;
const GAMMA: vec3f = vec3f(2.4);

override ENABLE_SRGB_CORRECTION: bool;
override ENCODING_GAMMA: f32;

override ENABLE_FFE: bool = false;

override VIEW_WIDTH_RATIO: f32 = 0.0;
override VIEW_HEIGHT_RATIO: f32 = 0.0;
override EDGE_X_RATIO: f32 = 0.0;
override EDGE_Y_RATIO: f32 = 0.0;

override C1_X: f32 = 0.0;
override C1_Y: f32 = 0.0;
override C2_X: f32 = 0.0;
override C2_Y: f32 = 0.0;
override LO_BOUND_X: f32 = 0.0;
override LO_BOUND_Y: f32 = 0.0;
override HI_BOUND_X: f32 = 0.0;
override HI_BOUND_Y: f32 = 0.0;

override A_LEFT_X: f32 = 0.0;
override A_LEFT_Y: f32 = 0.0;
override B_LEFT_X: f32 = 0.0;
override B_LEFT_Y: f32 = 0.0;

override A_RIGHT_X: f32 = 0.0;
override A_RIGHT_Y: f32 = 0.0;
override B_RIGHT_X: f32 = 0.0;
override B_RIGHT_Y: f32 = 0.0;
override C_RIGHT_X: f32 = 0.0;
override C_RIGHT_Y: f32 = 0.0;

struct PushConstant {
    reprojection_transform: mat4x4f,
    view_idx: u32,
    alpha: f32,
    enable_chroma_key: u32,
    _align1: u32,
    ck_target_hsv: vec3f,
    _align2: u32,
    ck_dist_min_hsv: vec3f,
    _align3: u32,
    ck_dist_max_hsv: vec3f,
}
var<push_constant> pc: PushConstant;

@group(0) @binding(0) var stream_texture: texture_2d<f32>;
@group(0) @binding(1) var stream_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
}

@vertex
fn vertex_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var result: VertexOutput;

    result.uv = vec2f(f32(vertex_index & 1), f32(vertex_index >> 1));
    result.position = pc.reprojection_transform * vec4f(result.uv.x - 0.5, 0.5 - result.uv.y, 0.0, 1.0);

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

        if pc.view_idx == 1 {
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

        if pc.view_idx == 1 {
            corrected_uv.x = 1.0 - corrected_uv.x;
        }
    }

    var color = textureSample(stream_texture, stream_sampler, corrected_uv).rgb;

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

    var alpha = pc.alpha;
    if pc.enable_chroma_key == 1 {
        let color_hsv = rgb_to_hsv(color);
        let mask = chroma_key_alpha(color_hsv);
        let target_rgb = hsv_to_rgb(pc.ck_target_hsv);

        // Note: because of this calculation, we require premultiplied alpha option in the XR layer
        color = max(color * mask, vec3f(0.0));
        alpha = mask;
    }

    return vec4f(color, alpha);
}

fn circular_distance(a: f32, b: f32) -> f32 {
    let diff = abs(a - b);
    return min(diff, 1.0 - diff);
}

fn chroma_key_alpha(hsv: vec3f) -> f32 {
    let dh = circular_distance(hsv.x, pc.ck_target_hsv.x);
    let ds = abs(hsv.y - pc.ck_target_hsv.y);
    let dv = abs(hsv.z - pc.ck_target_hsv.z);

    let max_vec = smoothstep(pc.ck_dist_min_hsv, pc.ck_dist_max_hsv, vec3f(dh, ds, dv));
    
    return max(max_vec.x, max(max_vec.y, max_vec.z));
}

fn rgb_to_hsv(rgb: vec3f) -> vec3f {
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

    return vec3f(h, s, v);
}

// https://stackoverflow.com/questions/24852345/hsv-to-rgb-color-conversion
fn hsv_to_rgb(hsv: vec3f) -> vec3f {
    var h = hsv.x;
    let s = hsv.y;
    let v = hsv.z;

    let i = i32(h * 6.0);
    let f = fract(h * 6.0);

    let w = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));

    if i == 0 {
        return vec3f(v, t, w);
    } else if i == 1 {
        return vec3f(q, v, w);
    } else if i == 2 {
        return vec3f(w, v, t);
    } else if i == 3 {
        return vec3f(w, q, v);
    } else if i == 4 {
        return vec3f(t, w, v);
    } else {
        return vec3f(v, w, q);
    }
}
