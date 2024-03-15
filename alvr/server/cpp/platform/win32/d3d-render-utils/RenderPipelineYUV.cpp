#include "RenderPipelineYUV.h"

using namespace std::string_literals;

namespace d3d_render_utils {

	RenderPipelineYUV::RenderPipelineYUV(ID3D11Device *device) {
		mDevice = device;
		mDevice->GetImmediateContext(&mImmediateContext);
	}

	void RenderPipelineYUV::Initialize(std::vector<ID3D11Texture2D *> inputTextures, ID3D11VertexShader *quadVertexShader,
		ID3D11PixelShader *pixelShader, ID3D11Texture2D *renderTarget, ID3D11Buffer *shaderBuffer)
	{
		mInputTextureViews.clear();
		for (auto tex : inputTextures) {
			ID3D11ShaderResourceView *resourceView;
			OK_OR_THROW(mDevice->CreateShaderResourceView(tex, nullptr, &resourceView),
				"Failed to create input texture resosurce view.");
			mInputTextureViews.push_back(resourceView);
		}

		D3D11_TEXTURE2D_DESC renderTargetDesc;
		renderTarget->GetDesc(&renderTargetDesc);

		DXGI_FORMAT uvFormat = renderTargetDesc.Format == DXGI_FORMAT_NV12 ? DXGI_FORMAT_R8G8_UNORM : DXGI_FORMAT_R16G16_UNORM;
		DXGI_FORMAT yFormat = renderTargetDesc.Format == DXGI_FORMAT_NV12 ? DXGI_FORMAT_R8_UNORM : DXGI_FORMAT_R16_UNORM;

		// Create SRV for luminance (Y) plane
		D3D11_SHADER_RESOURCE_VIEW_DESC srvDescY = {};
		srvDescY.Format = yFormat;
		srvDescY.ViewDimension = D3D11_SRV_DIMENSION_TEXTURE2D;
		srvDescY.Texture2D.MostDetailedMip = 0;
		srvDescY.Texture2D.MipLevels = 1;
		ID3D11ShaderResourceView* pSRVY;
		OK_OR_THROW(mDevice->CreateShaderResourceView(renderTarget, &srvDescY, &mRenderTargetResourceViewY),
			"Failed to create render target resosurce view.");

		// Create SRV for chrominance (UV) planes
		D3D11_SHADER_RESOURCE_VIEW_DESC srvDescUV = {};
		srvDescUV.Format = uvFormat;
		srvDescUV.ViewDimension = D3D11_SRV_DIMENSION_TEXTURE2D;
		srvDescUV.Texture2D.MostDetailedMip = 0;
		srvDescUV.Texture2D.MipLevels = 1;
		ID3D11ShaderResourceView* pSRVUV;
		OK_OR_THROW(mDevice->CreateShaderResourceView(renderTarget, &srvDescUV, &mRenderTargetResourceViewUV),
			"Failed to create render target resosurce view.");

		// Create luminance (Y) render target view
		D3D11_RENDER_TARGET_VIEW_DESC rtvDescLuminance = {};
		rtvDescLuminance.Format = yFormat;
		rtvDescLuminance.ViewDimension = D3D11_RTV_DIMENSION_TEXTURE2D;
		rtvDescLuminance.Texture2D.MipSlice = 0;
		ID3D11RenderTargetView* pRTVLuminance;
		OK_OR_THROW(mDevice->CreateRenderTargetView(renderTarget, &rtvDescLuminance, &mRenderTargetViewY),
			"Failed to create ID3D11RenderTargetView.");

		// Create chrominance (UV) render target view
		D3D11_RENDER_TARGET_VIEW_DESC rtvDescChrominance = {};
		rtvDescChrominance.Format = uvFormat;
		rtvDescChrominance.ViewDimension = D3D11_RTV_DIMENSION_TEXTURE2D;
		rtvDescChrominance.Texture2D.MipSlice = 0;
		ID3D11RenderTargetView* pRTVChrominance;
		OK_OR_THROW(mDevice->CreateRenderTargetView(renderTarget, &rtvDescChrominance, &mRenderTargetViewUV),
			"Failed to create ID3D11RenderTargetView.");

		mViewportY.Width = (FLOAT)renderTargetDesc.Width;
		mViewportY.Height = (FLOAT)renderTargetDesc.Height;
		mViewportY.MinDepth = 0.0f;
		mViewportY.MaxDepth = 1.0f;
		mViewportY.TopLeftX = 0.0f;
		mViewportY.TopLeftY = 0.0f;

		mViewportUV.Width = (FLOAT)renderTargetDesc.Width;
		mViewportUV.Height = (FLOAT)renderTargetDesc.Height;
		mViewportUV.MinDepth = 0.0f;
		mViewportUV.MaxDepth = 1.0f;
		mViewportUV.TopLeftX = 0.0f;
		mViewportUV.TopLeftY = 0.0f;

		mGenerateMipmaps = renderTargetDesc.MipLevels != 1;

		mShaderBuffer = shaderBuffer;
		mVertexShader = quadVertexShader;
		mPixelShader = pixelShader;
	}

	void RenderPipelineYUV::Initialize(std::vector<ID3D11Texture2D *> inputTextures, ID3D11VertexShader *quadVertexShader,
		std::vector<uint8_t> &pixelShaderCSO, ID3D11Texture2D *renderTarget, ID3D11Buffer *shaderBuffer)
	{
		auto pixelShader = CreatePixelShader(mDevice.Get(), pixelShaderCSO);
		Initialize(inputTextures, quadVertexShader, pixelShader, renderTarget,
			shaderBuffer);
	}

	void RenderPipelineYUV::Render(ID3D11DeviceContext *otherContext) {
		ID3D11DeviceContext *context = otherContext != nullptr ? otherContext : mImmediateContext.Get();

		ID3D11RenderTargetView* aRenderTargets[] = { mRenderTargetViewY.Get(), mRenderTargetViewUV.Get() };
		D3D11_VIEWPORT aViewports[] = { mViewportY, mViewportUV };

		ID3D11RenderTargetView* aRenderTargetsHack[] = { mRenderTargetViewY.Get()};
		D3D11_VIEWPORT aViewportsHack[] = { mViewportY };

		context->OMSetRenderTargets(2, aRenderTargets, nullptr);
		context->RSSetViewports(2, aViewports);

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

		// HACK: Whyyyyyy does the Y channel only render the top-left corner
		// with both render targets enabled????
		context->OMSetRenderTargets(1, aRenderTargetsHack, nullptr);
		context->RSSetViewports(1, aViewportsHack);

		context->Draw(4, 0);

		if (mGenerateMipmaps) {
			context->GenerateMips(mRenderTargetResourceViewY.Get());
			context->GenerateMips(mRenderTargetResourceViewUV.Get());
		}
	}

}