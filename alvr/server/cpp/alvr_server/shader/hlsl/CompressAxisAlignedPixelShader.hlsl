// Compress to rectangular slices

#include "FoveatedRendering.hlsli"



Texture2D<float4> compositionTexture;

SamplerState trilinearSampler {
	Filter = MIN_MAG_MIP_LINEAR;
	//AddressU = Wrap;
	//AddressV = Wrap;
};

float4 main(float2 uv : TEXCOORD0) : SV_Target{
	bool isRightEye = uv.x > 0.5;
	float2 eyeUV = TextureToEyeUV(uv, isRightEye);

	float2 alignedUV = eyeUV / eyeSizeRatio;

	float2 c0 = (1.-centerSize)/2.;
	float2 c1 = (edgeRatio-1.)*c0*(centerShift+1.)/edgeRatio;
	float2 c2 = (edgeRatio-1.)*centerSize+1.;

	float2 loBound = c0*(centerShift+1.)/c2;
	float2 hiBound = c0*(centerShift-1.)/c2+1.;
	float2 underBound = float2(alignedUV.x<loBound.x,alignedUV.y<loBound.y);
	float2 inBound = float2(loBound.x<alignedUV.x&&alignedUV.x<hiBound.x,loBound.y<alignedUV.y&&alignedUV.y<hiBound.y);
	float2 overBound = float2(alignedUV.x>hiBound.x,alignedUV.y>hiBound.y);

	float2 d1 = alignedUV*c2/edgeRatio+c1;
	float2 d2 = alignedUV*c2;
	float2 d3 = (alignedUV-1.)*c2+1.;
	float2 g1 = alignedUV/loBound;
	float2 g2 = (1.-alignedUV)/(1.-hiBound);

	float2 center = d1;
	float2 leftEdge = g1*d1+(1.-g1)*d2;
	float2 rightEdge = g2*d1+(1.-g2)*d3;

	float2 compressedUV = underBound*leftEdge+inBound*center+overBound*rightEdge;

	return compositionTexture.Sample(
		trilinearSampler, EyeToTextureUV(compressedUV, isRightEye));
}
