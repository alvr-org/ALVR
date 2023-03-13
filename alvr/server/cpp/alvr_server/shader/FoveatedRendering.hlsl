cbuffer FoveationVars {
	uint2 targetResolution;
	uint2 optimizedResolution;
	float2 eyeSizeRatio;
	float2 centerSize;
	float2 centerShift;
	float2 edgeRatio;
};

float2 TextureToEyeUV(float2 textureUV, bool isRightEye) {
	// flip distortion horizontally for right eye
	// left: x * 2; right: (1 - x) * 2
	return float2((textureUV.x + float(isRightEye) * (1. - 2. * textureUV.x)) * 2., textureUV.y);
}

float2 EyeToTextureUV(float2 eyeUV, bool isRightEye) {
	// saturate is used to avoid color bleeding between the two sides of the texture or with the black border when filtering
	//float2 clampedUV = saturate(eyeUV);
	// left: x / 2; right 1 - (x / 2)
	//return float2(clampedUV.x / 2. + float(isRightEye) * (1. - clampedUV.x), clampedUV.y);
	return float2(eyeUV.x * .5 + float(isRightEye) * (1. - eyeUV.x), eyeUV.y);
}