//Texture2D txLeft : register(t0);
//Texture2D txRight : register(t1);
//SamplerState samLinear : register(s0);

in vec4 vPos;
in vec2 vTex;
in uint vView;

out vec4 Pos;
out vec2 Tex;
out uint View;

void main()
{
	Pos = vPos;
	Tex = vTex;
	View = vView;
}
