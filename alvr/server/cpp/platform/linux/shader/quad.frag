#version 450

layout (binding = 0) uniform sampler2D src;

layout (location = 0) in vec2 inPos;

layout (location = 0) out vec4 outFragColor;

void main()
{
    outFragColor = texture(src, inPos);
}
