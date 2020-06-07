// Vertical Blur + Distortion

#include "FoveatedRendering.hlsli"

Texture2D<float4> horizontalBlurTexture;

SamplerState trilinearSampler {
	Filter = MIN_MAG_MIP_LINEAR;
	AddressU = CLAMP;
	AddressV = CLAMP;
};

static float minOffsetY = 1. / targetResolution.y;

float4 main(float2 uv : TEXCOORD0) : SV_Target {
	bool isRightEye = uv.x > 0.5;
	float2 distEyeUV = Distort(TextureToEyeUV(uv, isRightEye));
	/*float blurWeight = GetFilteringWeight(distEyeUV, targetResolution.y, optimizedResolution.y);

	float3 colorSum = float3(0, 0, 0);
	float weightSum = 0.;
	for (int y = -KERNEL_HALF_SIZE; y <= KERNEL_HALF_SIZE; y++) {
		float dY = float(y);
		float weight = UnscaledGaussian(dY, BLUR_STRENGTH * blurWeight);
		weightSum += weight;
		colorSum += weight * horizontalBlurTexture.Sample(
			trilinearSampler, EyeToTextureUV(float2(distEyeUV.x, distEyeUV.y + dY * minOffsetY), isRightEye)).rgb;
	}
	float3 finalColor = colorSum / weightSum;*/

	float3 finalColor = horizontalBlurTexture.Sample(
		trilinearSampler, EyeToTextureUV(distEyeUV, isRightEye)).rgb;

	return float4(finalColor, 1);
}