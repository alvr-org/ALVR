#version 450

layout (location = 0) in vec2 inPos;

layout (location = 0) out vec2 outPos;

const vec2 madd = vec2(0.5, 0.5);

void main()
{
    outPos = inPos.xy * madd + madd;
    gl_Position = vec4(inPos, 0.0, 1.0);
}
