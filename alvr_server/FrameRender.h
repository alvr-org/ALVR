#pragma once

#include <string>
#include <memory>
#include <stdint.h>

#include <d3d11.h>
#include <wrl.h>
#include <d3dcompiler.h>
#include <directxmath.h>
#include <directxcolors.h>
#include <SpriteFont.h>
#include <SimpleMath.h>

#include "openvr-utils\d3drender.h"
#include "openvr_driver.h"

using Microsoft::WRL::ComPtr;

class FrameRender
{
public:
	FrameRender(std::shared_ptr<CD3DRender> pD3DRender);
	virtual ~FrameRender();

	bool Startup();
	bool RenderFrame(ID3D11Texture2D *pTexture[][2], vr::VRTextureBounds_t bounds[][2], int layerCount, bool recentering, const std::string& message, const std::string& debugText);
	void RenderMessage(const std::string& message);
	void RenderDebugText(const std::string& debugText);
	void CreateResourceTexture();

	ComPtr<ID3D11Texture2D> GetTexture();
private:
	std::shared_ptr<CD3DRender> mD3DRender;
	ComPtr<ID3D11Texture2D> mStagingTexture;

	ComPtr<ID3D11VertexShader> mVertexShader;
	ComPtr<ID3D11PixelShader> mPixelShader;

	ComPtr<ID3D11InputLayout> mVertexLayout;
	ComPtr<ID3D11Buffer> mVertexBuffer;
	ComPtr<ID3D11Buffer> mIndexBuffer;

	ComPtr<ID3D11SamplerState> mSamplerLinear;

	ComPtr<ID3D11Texture2D> mDepthStencil;
	ComPtr<ID3D11RenderTargetView> mRenderTargetView;
	ComPtr<ID3D11DepthStencilView> mDepthStencilView;

	ComPtr<ID3D11BlendState> mBlendStateFirst;
	ComPtr<ID3D11BlendState> mBlendState;

	ComPtr<ID3D11Resource> mRecenterTexture;
	ComPtr<ID3D11ShaderResourceView> mRecenterResourceView;
	ComPtr<ID3D11Resource> mMessageBGTexture;
	ComPtr<ID3D11ShaderResourceView> mMessageBGResourceView;

	std::unique_ptr<DirectX::SpriteFont> mFont;
	std::unique_ptr<DirectX::SpriteBatch> mSpriteBatch;

	uint64_t m_frameIndex2;
	struct SimpleVertex
	{
		DirectX::XMFLOAT3 Pos;
		DirectX::XMFLOAT2 Tex;
		uint32_t View;
	};
	// Parameter for Draw method. 2-triangles for both eyes.
	static const int VERTEX_INDEX_COUNT = 12;
};

