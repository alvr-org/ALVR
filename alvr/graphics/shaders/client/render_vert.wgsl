struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
}

@vertex
fn main(@builtin(vertex_index) vertex_index: u32, @builtin(view_index) view_index: i32) -> VertexOutput {
    var output: VertexOutput;
    output.uv = vec2<f32>(f32(i32(vertex_index & 1u) + view_index) / 2.0, f32(vertex_index >> 1u));
    output.position = vec4<f32>((output.uv - 0.5) * 2.0, 0.0, 1.0);

    return output;
}