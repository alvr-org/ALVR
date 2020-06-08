#include "RenderPipeline.h"

using namespace std::string_literals;

namespace d3d_render_utils {

	RenderPipeline::RenderPipeline(ID3D11Device *device) {
		mDevice = device;
		mDevice->GetImmediateContext(&mImmediateContext);
	}

	void RenderPipeline::Initialize(std::vector<ID3D11Texture2D *> inputTextures, ID3D11VertexShader *quadVertexShader,
		ID3D11PixelShader *pixelShader, ID3D11Texture2D *renderTarget, ID3D11Buffer *shaderBuffer,
		bool enableAlphaBlend, bool overrideAlpha)
	{
		mInputTextureViews.clear();
		for (auto tex : inputTextures) {
			ID3D11ShaderResourceView *resourceView;
			OK_OR_THROW(mDevice->CreateShaderResourceView(tex, nullptr, &resourceView),
				"Failed to create input texture resosurce view.");
			mInputTextureViews.push_back(resourceView);
		}

		OK_OR_THROW(mDevice->CreateShaderResourceView(renderTarget, nullptr, &mRenderTargetResourceView),
			"Failed to create render target resosurce view.");

		OK_OR_THROW(mDevice->CreateRenderTargetView(renderTarget, nullptr, &mRenderTargetView),
			"Failed to create ID3D11RenderTargetView.");

		D3D11_TEXTURE2D_DESC renderTargetDesc;
		renderTarget->GetDesc(&renderTargetDesc);

		mViewport.Width = (FLOAT)renderTargetDesc.Width;
		mViewport.Height = (FLOAT)renderTargetDesc.Height;
		mViewport.MinDepth = 0.0f;
		mViewport.MaxDepth = 1.0f;
		mViewport.TopLeftX = 0.0f;
		mViewport.TopLeftY = 0.0f;

		mGenerateMipmaps = renderTargetDesc.MipLevels != 1;

		mShaderBuffer = shaderBuffer;
		mVertexShader = quadVertexShader;
		mPixelShader = pixelShader;

		D3D11_BLEND_DESC blendDesc = { 0 };
		blendDesc.RenderTarget[0].BlendEnable = enableAlphaBlend;
		blendDesc.RenderTarget[0].SrcBlend = (overrideAlpha ? D3D11_BLEND_ONE : D3D11_BLEND_SRC_ALPHA);
		blendDesc.RenderTarget[0].DestBlend = (overrideAlpha ? D3D11_BLEND_ZERO : D3D11_BLEND_INV_SRC_ALPHA);
		blendDesc.RenderTarget[0].BlendOp = D3D11_BLEND_OP_ADD;
		blendDesc.RenderTarget[0].SrcBlendAlpha = D3D11_BLEND_ONE;
		blendDesc.RenderTarget[0].DestBlendAlpha = D3D11_BLEND_ZERO;
		blendDesc.RenderTarget[0].BlendOpAlpha = D3D11_BLEND_OP_ADD;
		blendDesc.RenderTarget[0].RenderTargetWriteMask =
			(overrideAlpha ? D3D11_COLOR_WRITE_ENABLE_RED | D3D11_COLOR_WRITE_ENABLE_GREEN | D3D11_COLOR_WRITE_ENABLE_BLUE
				: D3D11_COLOR_WRITE_ENABLE_ALL);
		OK_OR_THROW(mDevice->CreateBlendState(&blendDesc, &mBlendState), "Failed to create blend state.");
	}

	void RenderPipeline::Initialize(std::vector<ID3D11Texture2D *> inputTextures, ID3D11VertexShader *quadVertexShader,
		std::vector<uint8_t> &pixelShaderCSO, ID3D11Texture2D *renderTarget, ID3D11Buffer *shaderBuffer,
		bool enableAlphaBlend, bool overrideAlpha)
	{
		auto pixelShader = CreatePixelShader(mDevice.Get(), pixelShaderCSO);
		Initialize(inputTextures, quadVertexShader, pixelShader, renderTarget,
			shaderBuffer, enableAlphaBlend, overrideAlpha);
	}

	void RenderPipeline::Render(ID3D11DeviceContext *otherContext) {
		ID3D11DeviceContext *context = otherContext != nullptr ? otherContext : mImmediateContext.Get();

		context->OMSetRenderTargets(1, mRenderTargetView.GetAddressOf(), nullptr);
		context->RSSetViewports(1, &mViewport);

		context->OMSetBlendState(mBlendState.Get(), nullptr, 0xffffffff);

		if (mShaderBuffer != nullptr) {
			context->PSSetConstantBuffers(0, 1, mShaderBuffer.GetAddressOf());
		}

		std::vector<ID3D11ShaderResourceView *> inputTextureViewPtrs;
		for (auto texView : mInputTextureViews) {
			inputTextureViewPtrs.push_back(texView.Get());
		}
		context->PSSetShaderResources(0, (UINT)mInputTextureViews.size(), &inputTextureViewPtrs[0]);

		context->VSSetShader(mVertexShader.Get(), nullptr, 0);
		context->PSSetShader(mPixelShader.Get(), nullptr, 0);

		context->IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
		context->Draw(4, 0);

		if (mGenerateMipmaps) {
			context->GenerateMips(mRenderTargetResourceView.Get());
		}
	}

}