struct PushConstant {
    transform: mat4x4f,
    color: u32,
}
var<push_constant> pc: PushConstant;

@vertex
fn vertex_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4f {
    return pc.transform * vec4f(0.0, 0.0, -f32(vertex_index), 1.0);
}

@fragment
fn fragment_main() -> @location(0) vec4f {
    return unpack4x8unorm(pc.color);
}
