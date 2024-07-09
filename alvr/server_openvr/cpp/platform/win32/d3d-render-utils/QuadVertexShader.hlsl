struct PixelType {
	float2 uv : TEXCOORD0; // TEXCOORD0 must be first if I don't want to define "position" in the pixel shader
	float4 position : SV_Position;
};

//https://gamedev.stackexchange.com/questions/98283/how-do-i-draw-a-full-screen-quad-in-directx-11
PixelType main(uint vertexID : SV_VertexID) {
	PixelType pix;
	pix.uv = float2(vertexID & 1, vertexID >> 1);
	pix.position = float4((pix.uv.x - 0.5f) * 2, -(pix.uv.y - 0.5f) * 2, 0, 1);
	return pix;
}