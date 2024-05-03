cbuffer FrameRenderParams {
	float encodingGamma;
	float _padding0;
	float _padding1;
	float _padding2;
};

Texture2D txLeft : register(t0);
Texture2D txRight : register(t1);
SamplerState samLinear : register(s0);

struct VS_INPUT
{
	float4 Pos : POSITION;
	float2 Tex : TEXCOORD;
	uint    View : VIEW;
};

struct PS_INPUT
{
	float4 Pos : SV_POSITION;
	float2 Tex : TEXCOORD;
	uint    View : VIEW;
};

#define SRGB_GAMMA_TO_NONLINEAR (1.0 / 2.4)
#define SRGB_GAMMA_TO_LINEAR (2.4)

float4 EncodingLinearToNonlinearRGB(float4 color, float gamma)
{
	float4 nonlinearColor;

	nonlinearColor.r = (color.r <= 0.0) ? color.r : pow(color.r, gamma);
	nonlinearColor.g = (color.g <= 0.0) ? color.g : pow(color.g, gamma);
	nonlinearColor.b = (color.b <= 0.0) ? color.b : pow(color.b, gamma);
	nonlinearColor.a = (color.a <= 0.0) ? color.a : pow(color.a, gamma);

	return nonlinearColor;
}

float4 LinearToNonlinearRGB(float4 color, float gamma)
{
	float4 nonlinearColor;

	nonlinearColor.r = (color.r <= 0.0031308) ? (color.r * 12.92) : (1.055 * pow(color.r, gamma) - 0.055);
	nonlinearColor.g = (color.g <= 0.0031308) ? (color.g * 12.92) : (1.055 * pow(color.g, gamma) - 0.055);
	nonlinearColor.b = (color.b <= 0.0031308) ? (color.b * 12.92) : (1.055 * pow(color.b, gamma) - 0.055);
	nonlinearColor.a = (color.a <= 0.0031308) ? (color.a * 12.92) : (1.055 * pow(color.a, gamma) - 0.055);

	return nonlinearColor;
}

float4 NonlinearToLinearRGB(float4 color, float gamma)
{
	float4 linearColor;

	linearColor.r = (color.r <= 0.04045) ? (color.r / 12.92) : pow((color.r + 0.055) / 1.055, gamma);
	linearColor.g = (color.g <= 0.04045) ? (color.g / 12.92) : pow((color.g + 0.055) / 1.055, gamma);
	linearColor.b = (color.b <= 0.04045) ? (color.b / 12.92) : pow((color.b + 0.055) / 1.055, gamma);
	linearColor.a = (color.a <= 0.04045) ? (color.a / 12.92) : pow((color.a + 0.055) / 1.055, gamma);

    return linearColor;
}

PS_INPUT VS(VS_INPUT input)
{
	PS_INPUT output = (PS_INPUT)0;
	output.Pos = input.Pos;
	output.Tex = input.Tex;
	output.View = input.View;

	return output;
}
float4 PS(PS_INPUT input) : SV_Target
{
	float4 color = float4(1.0, 0.0, 0.0, 1.0);
	uint correctionType = (input.View >> 1) & 0xF;
	uint shouldClamp = (input.View >> 5);

	if ((input.View & 1) == 1) {
		color = txRight.Sample(samLinear, input.Tex);
	}
	else {
		color = txLeft.Sample(samLinear, input.Tex);
	}
	
	if (shouldClamp == (uint)1) {
		color = clamp(color, 0.0, 1.0);
	}

	color = EncodingLinearToNonlinearRGB(color, encodingGamma);

	if (correctionType == (uint)1) { // Left View sRGB
		color = LinearToNonlinearRGB(color, SRGB_GAMMA_TO_NONLINEAR);
	}
	else if (correctionType == (uint)2) { // Left View non-HDR linear
		color = NonlinearToLinearRGB(color, SRGB_GAMMA_TO_LINEAR);
	}
	if (shouldClamp == (uint)2) {
		color = clamp(color, 0.0, 1.0);
	}

	return color;
};