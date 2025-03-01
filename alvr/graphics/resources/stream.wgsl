// todo: use expression directly when supported in naga
const DIV12: f32 = 0.0773993808;// 1.0 / 12.92
const DIV1: f32 = 0.94786729857; // 1.0 / 1.055
const THRESHOLD: f32 = 0.04045;
const GAMMA: vec3f = vec3f(2.4);

override ENABLE_SRGB_CORRECTION: bool;
override ENCODING_GAMMA: f32;

override ENABLE_UPSCALING: bool = false;
override UPSCALE_USE_EDGE_DIRECTION: bool = true;
override UPSCALE_EDGE_THRESHOLD: f32 = 4.0/255.0;
override UPSCALE_EDGE_SHARPNESS: f32 = 2.0;

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
    passthrough_mode: u32, // 0: Blend, 1: RGB chroma key, 2: HSV chroma key
    blend_alpha: f32,
    _align: u32,
    ck_channel0: vec4f,
    ck_channel1: vec4f,
    ck_channel2: vec4f,
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
    // tell upscaler to target a lower resolution for the edges
    var upscale_source_resolution = 1.0;
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
            upscale_source_resolution = upscale_source_resolution * edge_ratio.x;
        } else if corrected_uv.x > hi_bound.x {
            corrected_uv.x = right_edge.x;
            upscale_source_resolution = upscale_source_resolution * edge_ratio.x;
        } else {
            corrected_uv.x = center.x;
        }

        if corrected_uv.y < lo_bound.y {
            corrected_uv.y = left_edge.y;
            upscale_source_resolution = upscale_source_resolution * edge_ratio.y;
        } else if corrected_uv.y > hi_bound.y {
            corrected_uv.y = right_edge.y;
            upscale_source_resolution = upscale_source_resolution * edge_ratio.y;
        } else {
            corrected_uv.y = center.y;
        }

        corrected_uv = corrected_uv * view_size_ratio;

        if pc.view_idx == 1 {
            corrected_uv.x = 1.0 - corrected_uv.x;
        }
    }

    var color: vec3f;
    if ENABLE_UPSCALING {
        color = sgsr(vec4f(corrected_uv.x, corrected_uv.y, 0.0, 0.0), upscale_source_resolution).xyz;
    } else {
        color = textureSample(stream_texture, stream_sampler, corrected_uv).rgb;
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

    var alpha = pc.blend_alpha; // Default to Blend passthrough mode
    if pc.passthrough_mode != 0 { // Chroma key
        var current = color;
        if pc.passthrough_mode == 3 { // HSV mode
            current = rgb_to_hsv(color);
        }
        let mask = chroma_key_mask(current);

        // Note: because of this calculation, we require premultiplied alpha option in the XR layer
        color = max(color * mask, vec3f(0.0));
        alpha = mask;
    }

    return vec4f(color, alpha);
}

fn chroma_key_mask(color: vec3f) -> f32 {
    let start_max = vec3f(pc.ck_channel0.x, pc.ck_channel1.x, pc.ck_channel2.x);
    let start_min = vec3f(pc.ck_channel0.y, pc.ck_channel1.y, pc.ck_channel2.y);
    let end_min = vec3f(pc.ck_channel0.z, pc.ck_channel1.z, pc.ck_channel2.z);
    let end_max = vec3f(pc.ck_channel0.w, pc.ck_channel1.w, pc.ck_channel2.w);

    let start_mask = smoothstep(start_min, start_max, color);
    let end_mask = smoothstep(end_min, end_max, color);

    return max(start_mask.x, max(start_mask.y, max(start_mask.z, max(end_mask.x, max(end_mask.y, end_mask.z)))));
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

//============================================================================================================
//
//
//                  Copyright (c) 2023, Qualcomm Innovation Center, Inc. All rights reserved.
//                              SPDX-License-Identifier: BSD-3-Clause
//
//============================================================================================================

fn fastLanczos2(x: f32) -> f32
{
    var wA: f32 = x - 4.0;
    let wB: f32 = x * wA - wA;
    wA *= wA;
    return wB * wA;
}

fn weightY(dx: f32, dy: f32, c: f32, data: vec3f) -> vec2f {
    let stdA: f32 = data.x;
    let dir: vec2f = data.yz;
    let edgeDis: f32 = ((dx * dir.y) + (dy * dir.x));
    let x: f32 = (((dx * dx) + (dy * dy)) + ((edgeDis * edgeDis) * ((clamp(((c * c) * stdA), 0.0, 1.0) * 0.7) + -1.0)));
    let w: f32 = fastLanczos2(x);
    return vec2f(w, w * c);
}

fn weightYned(dx: f32, dy: f32, c: f32, data: f32) -> vec2f {
    let stdA: f32 = data;
    let x: f32 = ((dx * dx) + (dy * dy)) * 0.55 + clamp(abs(c) * stdA, 0.0, 1.0);
    let w: f32 = fastLanczos2(x);
    return vec2f(w, w * c);
}

fn edgeDirection(left: vec4f, right: vec4f) -> vec2f
{
    var dir: vec2f;
    let RxLz: f32 = (right.x + (-left.z));
    let RwLy: f32 = (right.w + (-left.y));
    var delta: vec2f;
    delta.x = (RxLz + RwLy);
    delta.y = (RxLz + (-RwLy));
    let lengthInv: f32 = inverseSqrt((delta.x * delta.x + 3.075740e-05) + (delta.y * delta.y));
    dir.x = (delta.x * lengthInv);
    dir.y = (delta.y * lengthInv);
    return dir;
}

fn sgsr(in_TEXCOORD0: vec4f, source_resolution_multiplier: f32) -> vec4f {
    // https://github.com/SnapdragonStudios/snapdragon-gsr/issues/2
    let dim = vec2f(textureDimensions(stream_texture)) * source_resolution_multiplier;
    let viewport_info = vec4f(1/dim.x, 1/dim.y, dim.x, dim.y);

    var color: vec4f;
    let texSample = textureSampleLevel(stream_texture, stream_sampler, in_TEXCOORD0.xy, 0.0);
    color.x = texSample.x;
    color.y = texSample.y;
    color.z = texSample.z;

    // all of these 1 values are the OperationMode
    // see https://github.com/SnapdragonStudios/snapdragon-gsr/tree/main/sgsr/v1#operation-mode
    let imgCoord: vec2f = (in_TEXCOORD0.xy * viewport_info.zw) + vec2f(-0.5, 0.5);
    let imgCoordPixel: vec2f = floor(imgCoord);
    var coord: vec2f = (imgCoordPixel * viewport_info.xy);
    let pl: vec2f = (imgCoord + (-imgCoordPixel));
    var left: vec4f = textureGather(1, stream_texture, stream_sampler, coord);

    let edgeVote: f32 = abs(left.z - left.y) + abs(color[1] - left.y) + abs(color[1] - left.z);
    if edgeVote > UPSCALE_EDGE_THRESHOLD {
        coord.x += viewport_info.x;

        var right: vec4f = textureGather(1, stream_texture, stream_sampler, coord + vec2f(viewport_info.x, 0.0));
        var upDown: vec4f;
        let texGatherA = textureGather(1, stream_texture, stream_sampler, coord + vec2f(0.0, -viewport_info.y));
        upDown.x = texGatherA.w;
        upDown.y = texGatherA.z;
        let texGatherB = textureGather(1, stream_texture, stream_sampler, coord + vec2f(0.0, viewport_info.y));
        upDown.z = texGatherB.y;
        upDown.w = texGatherB.x;

        let mean: f32 = (left.y + left.z + right.x + right.w) * 0.25;
        left = left - vec4(mean);
        right = right - vec4(mean);
        upDown = upDown - vec4(mean);
        color.w = color[1] - mean;

        let sum: f32 = (((((abs(left.x) + abs(left.y)) + abs(left.z)) + abs(left.w)) + (((abs(right.x) + abs(right.y)) + abs(right.z)) + abs(right.w))) + (((abs(upDown.x) + abs(upDown.y)) + abs(upDown.z)) + abs(upDown.w)));
        let sumMean: f32 = 1.014185e+01 / sum;
        let stdA: f32 = (sumMean * sumMean);

        var aWY: vec2f;
        if UPSCALE_USE_EDGE_DIRECTION {
            let data = vec3f(stdA, edgeDirection(left, right));
            aWY = weightY(pl.x, pl.y + 1.0, upDown.x, data);
            aWY += weightY(pl.x - 1.0, pl.y + 1.0, upDown.y, data);
            aWY += weightY(pl.x - 1.0, pl.y - 2.0, upDown.z, data);
            aWY += weightY(pl.x, pl.y - 2.0, upDown.w, data);
            aWY += weightY(pl.x + 1.0, pl.y - 1.0, left.x, data);
            aWY += weightY(pl.x, pl.y - 1.0, left.y, data);
            aWY += weightY(pl.x, pl.y, left.z, data);
            aWY += weightY(pl.x + 1.0, pl.y, left.w, data);
            aWY += weightY(pl.x - 1.0, pl.y - 1.0, right.x, data);
            aWY += weightY(pl.x - 2.0, pl.y - 1.0, right.y, data);
            aWY += weightY(pl.x - 2.0, pl.y, right.z, data);
            aWY += weightY(pl.x - 1.0, pl.y, right.w, data);
        } else {
            let data: f32 = stdA;
            aWY = weightYned(pl.x, pl.y + 1.0, upDown.x, data);
            aWY += weightYned(pl.x - 1.0, pl.y + 1.0, upDown.y, data);
            aWY += weightYned(pl.x - 1.0, pl.y - 2.0, upDown.z, data);
            aWY += weightYned(pl.x, pl.y - 2.0, upDown.w, data);
            aWY += weightYned(pl.x + 1.0, pl.y - 1.0, left.x, data);
            aWY += weightYned(pl.x, pl.y - 1.0, left.y, data);
            aWY += weightYned(pl.x, pl.y, left.z, data);
            aWY += weightYned(pl.x + 1.0, pl.y, left.w, data);
            aWY += weightYned(pl.x - 1.0, pl.y - 1.0, right.x, data);
            aWY += weightYned(pl.x - 2.0, pl.y - 1.0, right.y, data);
            aWY += weightYned(pl.x - 2.0, pl.y, right.z, data);
            aWY += weightYned(pl.x - 1.0, pl.y, right.w, data);
        }

        let finalY: f32 = aWY.y / aWY.x;
        let maxY: f32 = max(max(left.y, left.z), max(right.x, right.w));
        let minY: f32 = min(min(left.y, left.z), min(right.x, right.w));
        var deltaY: f32 = clamp(UPSCALE_EDGE_SHARPNESS * finalY, minY, maxY) - color.w;

        //smooth high contrast input
        deltaY = clamp(deltaY, -23.0 / 255.0, 23.0 / 255.0);

        color.x = clamp((color.x + deltaY), 0.0, 1.0);
        color.y = clamp((color.y + deltaY), 0.0, 1.0);
        color.z = clamp((color.z + deltaY), 0.0, 1.0);
    }

    color.w = 1.0; //assume alpha channel is not used

    return color;
}
