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
	if (input.View == (uint)0) { // Left View
		return txLeft.Sample(samLinear, input.Tex);
	}
	else if (input.View == (uint)1) { // Right View
		return txRight.Sample(samLinear, input.Tex);
	}
	else if (input.View == (uint)2) { // Left View sRGB
		return LinearToSRGB(txLeft.Sample(samLinear, input.Tex));
	}
	else { // Right View sRGB
		return LinearToSRGB(txRight.Sample(samLinear, input.Tex));
	}
};