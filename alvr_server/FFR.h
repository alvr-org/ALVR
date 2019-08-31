#pragma once

#include "d3d-render-utils/RenderPipeline.h"

class FFR
{
public:
	FFR(ID3D11Device *device);
	void Initialize(ID3D11Texture2D *compositionTexture);
	void Render();
	void GetOptimizedResolution(uint32_t *width, uint32_t *height);
	ID3D11Texture2D *GetOutputTexture();

private:
	struct FoveationVars {
		uint32_t targetEyeWidth;
		uint32_t targetEyeHeight;
		uint32_t optimizedEyeWidth;
		uint32_t optimizedEyeHeight;
		float focusPositionX;
		float focusPositionY;
		float foveationScaleX;

		float foveationScaleY;
		float boundStartX;
		float boundStartY;
		float distortedWidth;
		float distortedHeight;
	};

	FoveationVars mFoveationVars;
	static FoveationVars CalculateFoveationVars();

	Microsoft::WRL::ComPtr<ID3D11Device> mDevice;
	Microsoft::WRL::ComPtr<ID3D11Texture2D> mDistortedTexture; // staging texture
	Microsoft::WRL::ComPtr<ID3D11VertexShader> mQuadVertexShader;

	d3d_render_utils::RenderPipeline mHorizontalBlurPipeline;
	d3d_render_utils::RenderPipeline mDistortionPipeline;
};

