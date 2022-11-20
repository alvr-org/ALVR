#version 450

layout (binding = 0) uniform sampler2D src;

layout (location = 0) in vec2 uv;

layout (location = 0) out vec4 outFragColor;

layout (push_constant) uniform constants
{
    float renderWidth;
    float renderHeight;
    float brightness;
    float contrast;
    float saturation;
    float gamma;
    float sharpening;
    float _align;
} ColorCorrectionParams;

vec3 GetSharpenNeighborComponent(vec2 uv, float xoff, float yoff)
{
    const float sharpenNeighbourWeight = -ColorCorrectionParams.sharpening / 8.;
    return texture(src, uv + vec2(xoff, yoff)).rgb * sharpenNeighbourWeight;
}

vec3 blendLighten(vec3 base, vec3 blend)
{
    return vec3(max(base.r, blend.r), max(base.g, blend.g), max(base.b, blend.b));
}

// https://forum.unity.com/threads/hue-saturation-brightness-contrast-shader.260649/
void main()
{
    const float DX = 1. / ColorCorrectionParams.renderWidth;
    const float DY = 1. / ColorCorrectionParams.renderHeight;

    // sharpening
    vec3 pixel = texture(src, uv).rgb * (ColorCorrectionParams.sharpening + 1.);
    pixel += GetSharpenNeighborComponent(uv, -DX, -DY);
    pixel += GetSharpenNeighborComponent(uv, 0, -DY);
    pixel += GetSharpenNeighborComponent(uv, +DX, -DY);
    pixel += GetSharpenNeighborComponent(uv, +DX, 0);
    pixel += GetSharpenNeighborComponent(uv, +DX, +DY);
    pixel += GetSharpenNeighborComponent(uv, 0, +DY);
    pixel += GetSharpenNeighborComponent(uv, -DX, +DY);
    pixel += GetSharpenNeighborComponent(uv, -DX, 0);

    pixel += ColorCorrectionParams.brightness; // brightness
    pixel = (pixel - 0.5) * ColorCorrectionParams.contrast + 0.5f; // contast
    pixel = blendLighten(mix(vec3(dot(pixel, vec3(0.299, 0.587, 0.114))), pixel, vec3(ColorCorrectionParams.saturation)), pixel); // saturation + lighten only

    pixel = clamp(pixel, 0., 1.);
    pixel = pow(pixel, vec3(1. / ColorCorrectionParams.gamma)); // gamma

    outFragColor = vec4(pixel, 1.);
}
