// Compress to rectangular slices

#include "FoveatedRendering.hlsli"



Texture2D<float4> compositionTexture;

SamplerState trilinearSampler {
	Filter = MIN_MAG_MIP_LINEAR;
	//AddressU = Wrap;
	//AddressV = Wrap;
};

float CompressAxis(float eyeUV, float centerSizeAxis, float centerShiftAxis, float edgeRatioAxis) {
	float c0 = (1. - centerSizeAxis) / 2.;
	float c1 = (edgeRatioAxis - 1.) * c0 * (centerShiftAxis + 1.) / edgeRatioAxis;
	float c2 = (edgeRatioAxis - 1.) * centerSizeAxis + 1.;

	float loBound = c0 * (centerShiftAxis + 1.) / c2;
	float hiBound = c0 * (centerShiftAxis - 1.) / c2 + 1.;
	float center = eyeUV * c2 / edgeRatioAxis + c1;

	if (eyeUV < loBound) {
		float d2 = eyeUV * c2;
		float g1 = eyeUV / max(loBound, 1e-6);
		return g1 * center + (1. - g1) * d2;
	}

	if (eyeUV > hiBound) {
		float d3 = (eyeUV - 1.) * c2 + 1.;
		float g2 = (1. - eyeUV) / max(1. - hiBound, 1e-6);
		return g2 * center + (1. - g2) * d3;
	}

	return center;
}

float4 main(float2 uv : TEXCOORD0) : SV_Target {
	bool isRightEye = uv.x > 0.5;
	float2 eyeUV = TextureToEyeUV(uv, isRightEye) / eyeSizeRatio;
	float2 centerShift = isRightEye ? centerShiftRight : centerShiftLeft;

	float2 compressedUV = float2(
		CompressAxis(eyeUV.x, centerSize.x, centerShift.x, edgeRatio.x),
		CompressAxis(eyeUV.y, centerSize.y, centerShift.y, edgeRatio.y)
	);

	return compositionTexture.Sample(trilinearSampler, EyeToTextureUV(compressedUV, isRightEye));
}
