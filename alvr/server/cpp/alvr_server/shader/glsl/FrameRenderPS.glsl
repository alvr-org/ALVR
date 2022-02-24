//Texture2D txLeft : register(t0);
//Texture2D txRight : register(t1);
//SamplerState samLinear : register(s0);

uniform sampler2D txLeft;
uniform sampler2D txRight;

layout (location = 0) outView;

in vec4 Pos;
in vec2 Tex;
in uint View;

void main()
{
	if (View == uint(0)) { // Left View
		outView = texture(txLeft, input.Tex);
		//return txLeft.Sample(samLinear, input.Tex);
	}
	else { // Right View
        outView = texture(txRight, input.Tex);
		//return txRight.Sample(samLinear, input.Tex);
	}
};
