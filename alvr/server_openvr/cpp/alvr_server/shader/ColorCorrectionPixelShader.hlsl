
cbuffer ColorCorrectionParams {
	float renderWidth;
	float renderHeight;
	float brightness;
	float contrast;
	float saturation;
	float gamma;
	float sharpening;
	float _align;
};

const static float DX = 1. / renderWidth;
const static float DY = 1. / renderHeight;
const static float sharpenNeighbourWeight = -sharpening / 8.;

Texture2D<float4> sourceTexture;

SamplerState bilinearSampler {
	Filter = MIN_MAG_LINEAR_MIP_POINT;
	AddressU = CLAMP;
	AddressV = CLAMP;
};

float3 GetSharpenNeighborComponent(float2 uv, float xoff, float yoff) {
	return sourceTexture.Sample(bilinearSampler, uv + float2(xoff, yoff)).rgb * sharpenNeighbourWeight;
}

float3 blendLighten(float3 base, float3 blend) {
    return float3(max(base.r,blend.r),max(base.g,blend.g),max(base.b,blend.b));
}

// https://forum.unity.com/threads/hue-saturation-brightness-contrast-shader.260649/
float4 main(float2 uv : TEXCOORD0) : SV_Target{
	// sharpening
	float3 pixel = sourceTexture.Sample(bilinearSampler, uv).rgb * (sharpening + 1.);
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
    pixel = blendLighten(lerp(dot(pixel, float3(0.299, 0.587, 0.114)), pixel, saturation), pixel);  // saturation + lighten only

	pixel = clamp(pixel, 0, 1);
	pixel = pow(pixel, 1. / gamma);                                                                 // gamma

	return float4(pixel, 1);
}