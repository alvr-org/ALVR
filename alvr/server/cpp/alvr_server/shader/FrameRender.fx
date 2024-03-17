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

float4 LinearToSRGB(float4 linearRGB)
{
	float4 sRGB;

	// Apply sRGB transfer function to each channel
	sRGB.r = (linearRGB.r <= 0.0031308) ? (linearRGB.r * 12.92) : (1.055 * pow(linearRGB.r, 1.0 / 2.4) - 0.055);
	sRGB.g = (linearRGB.g <= 0.0031308) ? (linearRGB.g * 12.92) : (1.055 * pow(linearRGB.g, 1.0 / 2.4) - 0.055);
	sRGB.b = (linearRGB.b <= 0.0031308) ? (linearRGB.b * 12.92) : (1.055 * pow(linearRGB.b, 1.0 / 2.4) - 0.055);
	sRGB.a = (linearRGB.a <= 0.0031308) ? (linearRGB.a * 12.92) : (1.055 * pow(linearRGB.a, 1.0 / 2.4) - 0.055);

	return sRGB;
}

float4 sRGBToLinear(float4 color)
{
    float4 linearColor;

    linearColor.r = (color.r <= 0.04045) ? (color.r / 12.92) : pow((color.r + 0.055) / 1.055, 2.4);
    linearColor.g = (color.g <= 0.04045) ? (color.g / 12.92) : pow((color.g + 0.055) / 1.055, 2.4);
    linearColor.b = (color.b <= 0.04045) ? (color.b / 12.92) : pow((color.b + 0.055) / 1.055, 2.4);
	linearColor.a = (color.a <= 0.04045) ? (color.a / 12.92) : pow((color.a + 0.055) / 1.055, 2.4);

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
	if (correctionType == (uint)1) { // Left View sRGB
		color = LinearToSRGB(color);
	}
	else if (correctionType == (uint)2) { // Left View non-HDR linear
		color = sRGBToLinear(color);
	}
	if (shouldClamp == (uint)2) {
		color = clamp(color, 0.0, 1.0);
	}

	return color;
};