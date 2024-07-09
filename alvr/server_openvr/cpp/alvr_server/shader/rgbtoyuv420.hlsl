cbuffer YUVParams {
	float4 offset;
	float4 yCoeff;
	float4 uCoeff;
	float4 vCoeff;

	float renderWidth;
	float renderHeight;
	float _padding0;
	float _padding1;
};

Texture2D<float4> sourceTexture;

struct PS_OUTPUT
{
	float4 plane_Y: SV_Target0;
	float4 plane_UV: SV_Target1;
};

SamplerState bilinearSampler {
	Filter = MIN_MAG_LINEAR_MIP_POINT;
	AddressU = CLAMP;
	AddressV = CLAMP;
};

PS_OUTPUT main(float2 uv : TEXCOORD0) {
	PS_OUTPUT output;

	uint2 uvTexels = uint2(uv * float2(renderWidth, renderHeight));

	// Y @ 1x for YUV420
	float3 point1 = sourceTexture.Sample(bilinearSampler, uv).rgb;
	float y = dot(point1, yCoeff.rgb) + offset.x;

	// UV @ 1/2x for YUV420
	float2 image_uv = float2((uvTexels.x * 2) % renderWidth / renderWidth, (uvTexels.y * 2) % renderHeight / renderHeight);
	float3 point2 = sourceTexture.Sample(bilinearSampler, image_uv).rgb;
	float  u = dot(point2, uCoeff.rgb) + offset.y;
	float  v = dot(point2, vCoeff.rgb) + offset.z;

	output.plane_Y = float4(y, 0, 0, 1);
	output.plane_UV = float4(u, v, 0, 1);

	return output;
}