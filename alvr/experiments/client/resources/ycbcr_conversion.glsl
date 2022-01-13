// Note: naga (and wgsl) does not have combined samplers.
// spirv generated with https://shader-playground.timjones.io

#version 450

layout(set = 0, binding = 0) uniform sampler2D combined_ycbcr_sampler;
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 color;

void main() {
    color = texture(combined_ycbcr_sampler, uv);
}