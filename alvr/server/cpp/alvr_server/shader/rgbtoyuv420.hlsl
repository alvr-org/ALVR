cbuffer YUVParams {
	float renderWidth;
	float renderHeight;
	float padding1;
	float padding2;
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

float3 LinearToSRGB(float3 linearRGB)
{
	float3 sRGB;

	// Apply sRGB transfer function to each channel
	sRGB.x = (linearRGB.x <= 0.0031308) ? (linearRGB.x * 12.92) : (1.055 * pow(linearRGB.x, 1.0 / 2.4) - 0.055);
	sRGB.y = (linearRGB.y <= 0.0031308) ? (linearRGB.y * 12.92) : (1.055 * pow(linearRGB.y, 1.0 / 2.4) - 0.055);
	sRGB.z = (linearRGB.z <= 0.0031308) ? (linearRGB.z * 12.92) : (1.055 * pow(linearRGB.z, 1.0 / 2.4) - 0.055);

	return sRGB;
}

PS_OUTPUT main(float2 uv : TEXCOORD0) {
	PS_OUTPUT output;
    
	// Bless this page for ending my stint of plugging in random values from other projects:
	// https://kdashg.github.io/misc/colors/from-coeffs.html

	// RGB to YUV BT.2020 conversion limited range
	//const float3 offset = float3( 16.0/255.0,    0.501,  0.501);
	//const float3 YCoeff = float3( 0.2256,  0.5832, 0.0509);
	//const float3 UCoeff = float3(-0.1227, -0.3166, 0.4392);
	//const float3 VCoeff = float3( 0.4392, -0.4039 , -0.0353);

	// BT.709 limited range
	//const float3 offset = float3( 16.0/255.0,    0.501,  0.501);
	//const float3 YCoeff = float3( 0.1826,  0.6142, 0.062);
	//const float3 UCoeff = float3(-0.1006, -0.3386, 0.4392);
	//const float3 VCoeff = float3( 0.4382, -0.398,  -0.0402);

	// BT.2020 full range
	const float3 offset = float3( 0.0000,   0.50196,  0.50196);
	const float3 YCoeff = float3( 0.26270,  0.67800,  0.05930);
	const float3 UCoeff = float3(-0.13908, -0.35896,  0.49804);
	const float3 VCoeff = float3( 0.49804, -0.45798, -0.04006);

	// BT.709 full range
	//const float3 offset = float3( 0.0000,   0.50196,  0.50196);
	//const float3 YCoeff = float3( 0.21260,  0.71520,  0.07220);
	//const float3 UCoeff = float3(-0.11412, -0.38392,  0.49804);
	//const float3 VCoeff = float3( 0.49804, -0.45237, -0.04567);

	uint2 uvTexels = uint2(uv * float2(renderWidth, renderHeight));

	// Y @ 1x for YUV420
	float3 point1 = LinearToSRGB(sourceTexture.Sample(bilinearSampler, uv).rgb);
	float y = dot(point1, YCoeff) + offset.x;

	// UV @ 1/2x for YUV420
	float2 image_uv = float2((uvTexels.x * 2) % renderWidth / renderWidth, (uvTexels.y * 2) % renderHeight / renderHeight);
	float3 point2 = LinearToSRGB(sourceTexture.Sample(bilinearSampler, image_uv).rgb);
	float  u = dot(point2, UCoeff) + offset.y;
	float  v = dot(point2, VCoeff) + offset.z;

	output.plane_Y = float4(y, 0, 0, 1);
	output.plane_UV = float4(u, v, 0, 1);

	return output;
}