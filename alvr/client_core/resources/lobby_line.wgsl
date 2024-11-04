var<push_constant> transform: mat4x4f;

@vertex
fn vertex_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4f {
    return transform * vec4f(0.0, 0.0, -f32(vertex_index), 1.0);
}

@fragment
fn fragment_main() -> @location(0) vec4f {
    return vec4f(1.0);
}
