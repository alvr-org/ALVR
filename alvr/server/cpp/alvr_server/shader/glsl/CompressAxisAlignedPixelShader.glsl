// Compress to rectangular slices

#include "FoveatedRendering.glsl"

in vec2 uv;

uniform sampler2D compositionTexture;

layout (location = 0) out vec4 foveated;


void main() {
	bool isRightEye = uv.x > 0.5;
	vec2 eyeUV = TextureToEyeUV(uv, isRightEye);

	vec2 alignedUV = eyeUV / eyeSizeRatio;

	vec2 c0 = (1.-centerSize)/2.;
	vec2 c1 = (edgeRatio-1.)*c0*(centerShift+1.)/edgeRatio;
	vec2 c2 = (edgeRatio-1.)*centerSize+1.;

	vec2 loBound = c0*(centerShift+1.)/c2;
	vec2 hiBound = c0*(centerShift-1.)/c2+1.;
	vec2 underBound = float2(alignedUV.x<loBound.x,alignedUV.y<loBound.y);
	vec2 inBound = float2(loBound.x<alignedUV.x&&alignedUV.x<hiBound.x,loBound.y<alignedUV.y&&alignedUV.y<hiBound.y);
	vec2 overBound = float2(alignedUV.x>hiBound.x,alignedUV.y>hiBound.y);

	vec2 d1 = alignedUV*c2/edgeRatio+c1;
	vec2 d2 = alignedUV*c2;
	vec2 d3 = (alignedUV-1.)*c2+1.;
	vec2 g1 = alignedUV/loBound;
	vec2 g2 = (1.-alignedUV)/(1.-hiBound);

	vec2 center = d1;
	vec2 leftEdge = g1*d1+(1.-g1)*d2;
	vec2 rightEdge = g2*d1+(1.-g2)*d3;

	vec2 compressedUV = underBound*leftEdge+inBound*center+overBound*rightEdge;

	foveated = texture(compositionTexture, EyeToTextureUV(compressedUV, isRightEye));
	//return compositionTexture.Sample(
		//trilinearSampler, EyeToTextureUV(compressedUV, isRightEye));
}
