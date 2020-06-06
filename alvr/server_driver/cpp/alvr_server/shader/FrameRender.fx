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
	else { // Right View
		return txRight.Sample(samLinear, input.Tex);
	}
};