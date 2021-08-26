[[block]]
struct PushConstants {
    brightness: f32;
    contrast: f32;
    saturation: f32;
    gamma: f32;
    sharpening: f32;
};
var<push_constant> pc: PushConstants;

[[group(0), binding(0)]]
var tex: texture_2d<f32>;

fn weighted_neighbor(coord: vec2<i32>, x_off: i32, y_off: i32) -> vec3<f32> {
    return textureLoad(tex, coord + vec2<i32>(x_off, y_off), 0).rgb * (-pc.sharpening / 8.0);
}

// https://forum.unity.com/threads/hue-saturation-brightness-contrast-shader.260649/
[[stage(fragment)]]
fn main([[location(0)]] uv: vec2<f32>) -> [[location(0)]] vec4<f32> {
    let coord = vec2<i32>(uv * vec2<f32>(textureDimensions(tex)));

    // Sharpening
    var pixel = textureLoad(tex, coord, 0).rgb * (pc.sharpening + 1.0);
    pixel = pixel + weighted_neighbor(coord, -1, -1);
    pixel = pixel + weighted_neighbor(coord, 0, -1);
    pixel = pixel + weighted_neighbor(coord, 1, -1);
    pixel = pixel + weighted_neighbor(coord, 1, 0);
    pixel = pixel + weighted_neighbor(coord, 1, 1);
    pixel = pixel + weighted_neighbor(coord, 0, 1);
    pixel = pixel + weighted_neighbor(coord, -1, 1);
    pixel = pixel + weighted_neighbor(coord, -1, 0);

    // Brightness
    pixel = pixel + pc.brightness;

    // Contrast
    pixel = pixel + (pixel - 0.5) * pc.contrast + 0.5;

    // Saturation
    pixel =
        pixel * (1.0 - pc.saturation) + dot(pixel, vec3<f32>(0.299, 0.587, 0.114)) * pc.saturation;

    // Gamma
    pixel = clamp(pixel, vec3<f32>(0.0), vec3<f32>(1.0));
    pixel = pow(pixel, vec3<f32>(1.0 / pc.gamma));

    return vec4<f32>(pixel, 1.0);
}
