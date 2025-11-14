#version 300 es
#extension GL_OES_EGL_image_external_essl3 : enable

precision mediump float;

uniform samplerExternalOES tex;

// Convert from limited colors to full
const float LIMITED_MIN = 16.0 / 255.0;
const float LIMITED_MAX = 235.0 / 255.0;

in vec2 uv;
out vec4 out_color;

void main() {
    vec3 color = texture(tex, uv).rgb;
#ifdef FIX_LIMITED_RANGE
    color = LIMITED_MIN + ((LIMITED_MAX - LIMITED_MIN) * color);
#endif
    out_color = vec4(color, 1.0);
}
