#version 300 es

out vec2 uv;

void main() {
    uv = vec2(gl_VertexID & 1, gl_VertexID >> 1);
    gl_Position = vec4((uv - 0.5f) * 2.f, 0, 1);
}
