#pragma once

#include "d3d-render-utils/RenderPipeline.h"

class FFR
{
public:
	FFR(ID3D11Device* device);
	void Initialize(ID3D11Texture2D* compositionTexture);
	void Render();
	void GetOptimizedResolution(uint32_t* width, uint32_t* height);
	ID3D11Texture2D* GetOutputTexture();

private:
	Microsoft::WRL::ComPtr<ID3D11Device> mDevice;
	Microsoft::WRL::ComPtr<ID3D11Texture2D> mOptimizedTexture;
	Microsoft::WRL::ComPtr<ID3D11VertexShader> mQuadVertexShader;

	std::vector<d3d_render_utils::RenderPipeline> mPipelines;
};

