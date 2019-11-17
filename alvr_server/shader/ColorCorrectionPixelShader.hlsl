
cbuffer ColorCorrectionParams {
	float brightness;
	float contrast;
	float saturation;
	float gamma;
};

Texture2D<float4> sourceTexture;

SamplerState bilinearSampler {
	Filter = MIN_MAG_LINEAR_MIP_POINT;
	AddressU = CLAMP;
	AddressV = CLAMP;
};

// https://forum.unity.com/threads/hue-saturation-brightness-contrast-shader.260649/
float4 main(float2 uv : TEXCOORD0) : SV_Target{
	float3 pixel = sourceTexture.Sample(bilinearSampler, uv);

	pixel = pow(pixel, 1. / gamma);                                            // gamma
	pixel += brightness;                                                       // brightness
	pixel = (pixel - 0.5) * contrast + 0.5f;                                   // contast
	pixel = lerp(dot(pixel, float3(0.299, 0.587, 0.114)), pixel, saturation);  // saturation

	return float4(pixel, 1);
}