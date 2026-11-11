// Skinned hand shader.
//
// Unlike the scene, a hand model carries no baked lighting — it is a bare mesh — so a flat unlit
// fetch would draw it as a silhouette with no readable curl at all. It is therefore shaded from
// the skinned normal, with one key light and a hemispheric ambient, which is enough to read the
// finger poses against the video behind it.
//
// The joint matrices already include wherever the hand is in the world, so positions and normals
// come out of the skinning in world space and the light direction is a world constant.

struct Uniforms {
    view_proj: mat4x4<f32>,
}

// Fixed size rather than a storage buffer: read-only storage in the vertex stage is a downlevel
// capability that not every backend wgpu can pick offers, while a uniform buffer this size is
// guaranteed everywhere. 64 joints is well beyond a hand's 26.
struct Skin {
    joints: array<mat4x4<f32>, 64>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<uniform> skin: Skin;
@group(0) @binding(2) var base_color: texture_2d<f32>;
@group(0) @binding(3) var base_color_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec3<f32>,
    @location(4) joints: vec4<u32>,
    @location(5) weights: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    // A vertex with no weights at all would collapse to the origin, which is how a mis-read skin
    // shows up as a spike through the scene; fall back to the first joint instead.
    let total = in.weights.x + in.weights.y + in.weights.z + in.weights.w;
    var weights = in.weights;
    if total < 0.0001 {
        weights = vec4<f32>(1.0, 0.0, 0.0, 0.0);
    } else {
        weights = weights / total;
    }

    let skinning = skin.joints[in.joints.x] * weights.x
        + skin.joints[in.joints.y] * weights.y
        + skin.joints[in.joints.z] * weights.z
        + skin.joints[in.joints.w] * weights.w;

    let world = skinning * vec4<f32>(in.position, 1.0);

    var out: VertexOutput;
    out.clip_position = uniforms.view_proj * world;
    out.uv = in.uv;
    out.color = in.color;
    // The skinning matrix is a rotation and a uniform scale, so it transforms normals directly;
    // no inverse transpose is needed.
    out.normal = (skinning * vec4<f32>(in.normal, 0.0)).xyz;
    return out;
}

const LIGHT_DIRECTION = vec3<f32>(0.32, 0.82, 0.47);

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.normal);
    let sampled = textureSample(base_color, base_color_sampler, in.uv);

    // Hemispheric ambient plus one key light: enough shape to read a curled finger, without
    // pretending to match whatever the scene behind it is lit by.
    let ambient = 0.42 + 0.16 * normal.y;
    let key = 0.58 * clamp(dot(normal, normalize(LIGHT_DIRECTION)), 0.0, 1.0);

    return vec4<f32>(in.color * sampled.rgb * (ambient + key), 1.0);
}
