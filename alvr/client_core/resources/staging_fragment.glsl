#version 300 es
#extension GL_OES_EGL_image_external_essl3 : enable

uniform samplerExternalOES tex;

in vec2 uv;
out vec4 out_color;

void main() {
    out_color = texture(tex, uv);
}
