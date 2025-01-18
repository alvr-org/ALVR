#version 300 es

uniform int view_idx;

out vec2 uv;

void main() {
    vec2 screen_uv = vec2(gl_VertexID & 1, gl_VertexID >> 1);
    gl_Position = vec4((screen_uv - 0.5f) * 2.f, 0, 1);
    uv = vec2((screen_uv.x + float(view_idx)) / 2.f, screen_uv.y);
}
