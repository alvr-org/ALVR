#version 300 es

const float PI = 3.14159265359f;
const float FAR = 150.0f;
const float SKY_SIDE = 500.0f;
const float HAND_BASE_HALF_SIDE = 0.1f;
const float HAND_SQUEEZE_MULTIPLIER = 1.2f;

uniform lowp int object_type;
uniform mat4 transform; // each object type uses this differently
uniform float squeeze_amount;

out vec3 position;
out vec2 uv;

void main() {
    float norm_vert_a = float(gl_VertexID & 1);
    float norm_vert_b = float(gl_VertexID >> 1);
    if(object_type == 0) { // Ground
        position = vec3((norm_vert_a * 2.0f - 1.0f) * FAR, 0.0f, (norm_vert_b * 2.0f - 1.0f) * FAR);
        gl_Position = transform * vec4(position, 1.0f);
    } else if(object_type == 1) { // HUD
        gl_Position = transform * vec4(norm_vert_a - 0.5f, norm_vert_b - 0.5f, 0.0f, 1.0f);
        uv = vec2(norm_vert_a, 1.0f - norm_vert_b);
    } else if(object_type == 2) { // Hands (single bone)
        gl_Position = transform * vec4(0.0f, 0.0f, -float(gl_VertexID), 1.0f);
    }
}
