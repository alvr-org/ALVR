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

#include "d3drender.h"
#include "openvr_driver.h"
#include "FFR.h"

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
	void GetEncodingResolution(uint32_t *width, uint32_t *height);

	ComPtr<ID3D11Texture2D> GetTexture();
private:
	std::shared_ptr<CD3DRender> m_pD3DRender;
	ComPtr<ID3D11Texture2D> m_pStagingTexture;

	ComPtr<ID3D11VertexShader> m_pVertexShader;
	ComPtr<ID3D11PixelShader> m_pPixelShader;

	ComPtr<ID3D11InputLayout> m_pVertexLayout;
	ComPtr<ID3D11Buffer> m_pVertexBuffer;
	ComPtr<ID3D11Buffer> m_pIndexBuffer;

	ComPtr<ID3D11SamplerState> m_pSamplerLinear;

	ComPtr<ID3D11Texture2D> m_pDepthStencil;
	ComPtr<ID3D11RenderTargetView> m_pRenderTargetView;
	ComPtr<ID3D11DepthStencilView> m_pDepthStencilView;

	ComPtr<ID3D11BlendState> m_pBlendStateFirst;
	ComPtr<ID3D11BlendState> m_pBlendState;

	ComPtr<ID3D11Resource> m_recenterTexture;
	ComPtr<ID3D11ShaderResourceView> m_recenterResourceView;
	ComPtr<ID3D11Resource> m_messageBGTexture;
	ComPtr<ID3D11ShaderResourceView> m_messageBGResourceView;

	std::unique_ptr<DirectX::SpriteFont> m_Font;
	std::unique_ptr<DirectX::SpriteBatch> m_SpriteBatch;

	uint64_t m_frameIndex2;
	struct SimpleVertex
	{
		DirectX::XMFLOAT3 Pos;
		DirectX::XMFLOAT2 Tex;
		uint32_t View;
	};
	// Parameter for Draw method. 2-triangles for both eyes.
	static const int VERTEX_INDEX_COUNT = 12;

	std::unique_ptr<d3d_render_utils::RenderPipeline> m_colorCorrectionPipeline;
	bool enableColorCorrection;

	std::unique_ptr<FFR> m_ffr;
	bool enableFFR;
};

