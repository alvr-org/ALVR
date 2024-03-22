#version 300 es
#extension GL_OES_EGL_image_external_essl3 : enable
precision mediump float;

uniform samplerExternalOES texture;
in vec2 uv;
out vec4 color;

void main() {
    color = texture(texture, uv);
}