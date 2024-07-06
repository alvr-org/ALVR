#version 300 es

const float DIV12 = 1.f / 12.92f;
const float DIV1 = 1.f / 1.055f;
const float THRESHOLD = 0.04045f;
const vec3 GAMMA = vec3(2.4f);

// Convert from limited colors to full
const float LIMITED_MIN = 16.0f / 255.0f;
const float LIMITED_MAX = 235.0f / 255.0f;

uniform sampler2D tex;
uniform int fix_limited_range;
uniform int enable_srgb_correction;
uniform float encoding_gamma;

in vec2 uv;
out vec4 out_color;

void main() {
    out_color = texture(tex, uv);

    if(fix_limited_range == 1) {
        // For some reason, the encoder shifts full-range color into the negatives and over one.
        out_color.rgb = LIMITED_MIN + ((LIMITED_MAX - LIMITED_MIN) * out_color.rgb);
    }

    if(enable_srgb_correction == 1) {
        vec3 condition = vec3(out_color.r < THRESHOLD, out_color.g < THRESHOLD, out_color.b < THRESHOLD);
        vec3 lowValues = out_color.rgb * DIV12;
        vec3 highValues = pow((out_color.rgb + 0.055f) * DIV1, GAMMA);
        out_color.rgb = condition * lowValues + (1.0f - condition) * highValues;
    }

    if(encoding_gamma != 0.0f) {
        vec3 enc_condition = vec3(out_color.r < 0.0f, out_color.g < 0.0f, out_color.b < 0.0f);
        vec3 enc_lowValues = out_color.rgb;
        vec3 enc_highValues = pow(out_color.rgb, vec3(encoding_gamma));
        out_color.rgb = enc_condition * enc_lowValues + (1.0f - enc_condition) * enc_highValues;
    }
}
