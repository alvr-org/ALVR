struct VertexOutput {
    [[location(0)]] uv: vec2<f32>;
    [[builtin(position)]] position: vec4<f32>;
}

[[stage(vertex)]]
fn main([[builtin(vertex_index)]] vertex_index: u32) -> [[builtin(position)]] vec4<f32> {
    let output: VertexOutput;
    output.uv = vec2<f32>(vertex_index & 1, vertex_index >> 1);
    output.position = vec4<f32>((output.uv - 0.5) * 2.0, 0.0, 1.0);

    return output;
}