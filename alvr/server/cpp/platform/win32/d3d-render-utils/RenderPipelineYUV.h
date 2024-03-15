#pragma once

#include "RenderUtils.h"

namespace d3d_render_utils {

	class RenderPipelineYUV {
	public:
		RenderPipelineYUV(ID3D11Device *device);

		void Initialize(std::vector<ID3D11Texture2D *> inputTextures,
			ID3D11VertexShader *quadVertexShader, std::vector<uint8_t> &pixelShaderCSO,
			ID3D11Texture2D *renderTarget, ID3D11Buffer *shaderBuffer = nullptr);
		void Initialize(std::vector<ID3D11Texture2D *> inputTextures,
			ID3D11VertexShader *quadVertexShader, ID3D11PixelShader *pixelShader,
			ID3D11Texture2D *renderTarget, ID3D11Buffer *shaderBuffer = nullptr);

		void Render(ID3D11DeviceContext *otherContext = nullptr);

	private:
		D3D11_VIEWPORT mViewportY;
		D3D11_VIEWPORT mViewportUV;
		bool mGenerateMipmaps;

		Microsoft::WRL::ComPtr<ID3D11Device> mDevice;
		Microsoft::WRL::ComPtr<ID3D11DeviceContext> mImmediateContext;
		std::vector<Microsoft::WRL::ComPtr<ID3D11ShaderResourceView>> mInputTextureViews;
		Microsoft::WRL::ComPtr<ID3D11RasterizerState> mRasterizerState;
		Microsoft::WRL::ComPtr<ID3D11RenderTargetView> mRenderTargetViewY;
		Microsoft::WRL::ComPtr<ID3D11RenderTargetView> mRenderTargetViewUV;
		Microsoft::WRL::ComPtr<ID3D11ShaderResourceView> mRenderTargetResourceViewY;
		Microsoft::WRL::ComPtr<ID3D11ShaderResourceView> mRenderTargetResourceViewUV;
		Microsoft::WRL::ComPtr<ID3D11Buffer> mShaderBuffer;
		Microsoft::WRL::ComPtr<ID3D11VertexShader> mVertexShader;
		Microsoft::WRL::ComPtr<ID3D11PixelShader> mPixelShader;
		Microsoft::WRL::ComPtr<ID3D11BlendState> mBlendState;
	};
}
