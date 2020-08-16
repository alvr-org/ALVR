// Compress to rectangular slices

#include "FoveatedRendering.hlsli"

const static float2 COMPRESSED_TO_SOURCE = float2(optimizedResolution) / float2(targetResolution);
const static float2 PADDING = 1. / float2(targetResolution);


Texture2D<float4> compositionTexture;

SamplerState trilinearSampler {
	Filter = MIN_MAG_MIP_LINEAR;
	//AddressU = Wrap;
	//AddressU = Wrap; // not working, using fmod() instead
};

static float minOffsetY = 1. / targetResolution.y;

float4 main(float2 uv : TEXCOORD0) : SV_Target{
	bool isRightEye = uv.x > 0.5;
	float2 eyeUV = TextureToEyeUV(uv, isRightEye);

	float2 alignedUV = eyeUV * COMPRESSED_TO_SOURCE;
	float2 edge = foveationScale + 4. * PADDING;

	float2 overEdge = float2(alignedUV.x > edge.x, alignedUV.y > edge.y);
	float2 overHalfEdge = float2(alignedUV.x > edge.x / 2., alignedUV.y > edge.y / 2.);

	float sourceScale = (overEdge.x + 1.) * (overEdge.y + 1.); // [1 or 2] * [1 or 2]

	float2 compressedOffset = 1. / 2. * float2(overEdge.x * (1. - overHalfEdge.y),
											 overEdge.y * (1. - overHalfEdge.x)); // left + top

	float2 foveationRescale =
		1. / 2. +                                                       // |||} center
		3. * compressedOffset +                                         // ||\ left + top
		overEdge.x * overHalfEdge.y + overEdge.y * overHalfEdge.x +     // |\ right + bottom
		overEdge.x * overEdge.y;                                        // \ corners

	float2 paddingCount =
		2. +                                                            // ||} center
		overEdge.x * float2(3, -1. + 2. * overHalfEdge.y - overEdge.y) +  // |\ left + right + corners
		overEdge.y * float2(-1. + 2. * overHalfEdge.x - overEdge.x, 3);   // \ top + bottom + corners

	float2 compressedUV = (alignedUV - paddingCount * PADDING) * sourceScale +
		focusPosition - foveationRescale * foveationScale - compressedOffset;

	return compositionTexture.Sample(
		trilinearSampler, EyeToTextureUV(fmod(compressedUV + 1., 1), isRightEye));
}