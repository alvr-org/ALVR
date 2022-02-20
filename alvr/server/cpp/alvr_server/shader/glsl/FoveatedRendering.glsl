uniform FoveationVars {
	uvec2 targetResolution;
	uvec2 optimizedResolution;
	vec2 eyeSizeRatio;
	vec2 centerSize;
	vec2 centerShift;
	vec2 edgeRatio;
};

vec2 TextureToEyeUV(vec2 textureUV, bool isRightEye) {
	// flip distortion horizontally for right eye
	// left: x * 2; right: (1 - x) * 2
	return vec2((textureUV.x + float(isRightEye) * (1. - 2. * textureUV.x)) * 2., textureUV.y);
}

vec2 EyeToTextureUV(vec2 eyeUV, bool isRightEye) {
	// saturate is used to avoid color bleeding between the two sides of the texture or with the black border when filtering
	//float2 clampedUV = saturate(eyeUV);
	// left: x / 2; right 1 - (x / 2)
	//return float2(clampedUV.x / 2. + float(isRightEye) * (1. - clampedUV.x), clampedUV.y);
	return vec2(eyeUV.x / 2. + float(isRightEye) * (1. - eyeUV.x), eyeUV.y);
}
