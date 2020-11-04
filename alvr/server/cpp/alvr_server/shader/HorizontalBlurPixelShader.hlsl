// Horizontal Blur

#include "FoveatedRendering.hlsli"

Texture2D<float4> compositionTexture;

SamplerState bilinearSampler {
	Filter = MIN_MAG_LINEAR_MIP_POINT;
	AddressU = CLAMP;
	AddressV = CLAMP;
};

static float minOffsetX = 1. / targetResolution.x;

float4 main(float2 uv : TEXCOORD0) : SV_Target{
	bool isRightEye = uv.x > 0.5;
	float2 eyeUV = TextureToEyeUV(uv, isRightEye);
	float blurWeight = GetFilteringWeight(eyeUV, targetResolution.x, optimizedResolution.x);

	float3 colorSum = float3(0, 0, 0);
	float weightSum = 0.;
	for (int x = -KERNEL_HALF_SIZE; x <= KERNEL_HALF_SIZE; x++) {
		float dX = float(x);
		float weight = UnscaledGaussian(dX, BLUR_STRENGTH * blurWeight);
		weightSum += weight;
		colorSum += weight * compositionTexture.Sample(
			bilinearSampler, EyeToTextureUV(float2(eyeUV.x + dX * minOffsetX, eyeUV.y), isRightEye)).rgb;
	}
	float3 finalColor = colorSum / weightSum;

	return float4(finalColor, 1);
}