// Compress to rectangular slices

#include "FoveatedRendering.hlsli"

const static float2 EDGE_COMPRESSED_SIZE = (1.-centerSize)/(2.*edgeRatio);



Texture2D<float4> compositionTexture;

SamplerState trilinearSampler {
	Filter = MIN_MAG_MIP_LINEAR;
	//AddressU = Wrap;
	//AddressU = Wrap; // not working, using fmod() instead
};

float4 main(float2 uv : TEXCOORD0) : SV_Target{
	bool isRightEye = uv.x > 0.5;
	float2 eyeUV = TextureToEyeUV(uv, isRightEye);

	float2 alignedUV = eyeUV / eyeSizeRatio;

    float2 loBound = edgeRatio*EDGE_COMPRESSED_SIZE*(centerShift+1.)/((edgeRatio-1.)*centerSize+1.);
    float2 hiBound = edgeRatio*EDGE_COMPRESSED_SIZE*(centerShift-1.)/((edgeRatio-1.)*centerSize+1.)+1.;
    float2 underBound = float2(alignedUV.x<loBound.x,alignedUV.y<loBound.y);
    float2 inBound = float2(loBound.x<alignedUV.x&&alignedUV.x<hiBound.x,loBound.y<alignedUV.y&&alignedUV.y<hiBound.y);
    float2 overBound = float2(alignedUV.x>hiBound.x,alignedUV.y>hiBound.y);
    
    float2 center = alignedUV*((edgeRatio-1.)*centerSize+1.)/edgeRatio+EDGE_COMPRESSED_SIZE*(edgeRatio-1.)*(centerShift+1.);
    float2 leftEdge = alignedUV*(edgeRatio-(edgeRatio-1.)*(1.-centerSize));
    float2 rightEdge = (alignedUV-1.)*(edgeRatio-(edgeRatio-1.)*(1.-centerSize))+1.;
    
	float2 compressedUV = underBound*leftEdge+inBound*center+overBound*rightEdge;

	return compositionTexture.Sample(
		trilinearSampler, EyeToTextureUV(fmod(compressedUV + 1., 1), isRightEye));
}