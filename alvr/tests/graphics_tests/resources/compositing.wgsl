[[block]]
struct PushConstants {
    rect_offset: vec2<f32>;
    rect_extent: vec2<f32>; 
};

var<push_constant> pc: PushConstants;

[[group(0), binding(0)]]
var layer: texture_2d<f32>;

[[group(0), binding(1)]]
var sampler: sampler;

[[stage(fragment)]]
fn main([[location(0)]] uv: vec2<f32>) -> [[location(0)]] vec4<f32> {
    let new_uv = (pc.rect_offset + uv * pc.rect_extent) / vec2<f32>(textureDimension(layer))
    return textureSample(layer, sampler, new_uv);
}
