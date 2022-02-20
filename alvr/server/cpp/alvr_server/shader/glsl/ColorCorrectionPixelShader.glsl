
uniform ColorCorrectionParams {
	float renderWidth;
	float renderHeight;
	float brightness;
	float contrast;
	float saturation;
	float gamma;
	float sharpening;
	float _align;
};

layout (location = 0) out vec4 correctedColor;

const float DX = 1. / renderWidth;
const float DY = 1. / renderHeight;
const float sharpenNeighbourWeight = -sharpening / 8.;

uniform sampler2D sourceTexture;
uniform vec2 uv;

vec3 GetSharpenNeighborComponent(vec2 uv, float xoff, float yoff) {
	return texture(sourceTexture, uv + vec2(xoff, yoff)).rgb * sharpenNeighbourWeight;
}

vec3 blendLighten(vec3 base, vec3 blend) {
    return vec3(max(base.r,blend.r),max(base.g,blend.g),max(base.b,blend.b));
}

// https://forum.unity.com/threads/hue-saturation-brightness-contrast-shader.260649/
void main() {
	// sharpening
	vec3 pixel = texture(sourceTexture, uv).rgb * (sharpening + 1.0);
	pixel += GetSharpenNeighborComponent(uv, -DX, -DY);
	pixel += GetSharpenNeighborComponent(uv, 0, -DY);
	pixel += GetSharpenNeighborComponent(uv, +DX, -DY);
	pixel += GetSharpenNeighborComponent(uv, +DX, 0);
	pixel += GetSharpenNeighborComponent(uv, +DX, +DY);
	pixel += GetSharpenNeighborComponent(uv, 0, +DY);
	pixel += GetSharpenNeighborComponent(uv, -DX, +DY);
	pixel += GetSharpenNeighborComponent(uv, -DX, 0);

	pixel += brightness;                                                                            // brightness
	pixel = (pixel - 0.5) * contrast + 0.5f;                                                        // contast
    pixel = blendLighten(mix(dot(pixel, float3(0.299, 0.587, 0.114)), pixel, saturation), pixel);  // saturation + lighten only

	pixel = clamp(pixel, 0, 1);
	pixel = pow(pixel, 1. / gamma);                                                                 // gamma

	correctedColor = vec4(pixel, 1);
}
