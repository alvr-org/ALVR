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

using Microsoft::WRL::ComPtr;

class FrameRender
{
public:
	FrameRender(int renderWidth, int renderHeight, bool debugFrameIndex, CD3DRender *pD3DRender);
	virtual ~FrameRender();

	bool Startup(ID3D11Texture2D *pTexture[]);
	bool RenderFrame(ID3D11Texture2D *pTexture[], int textureNum, const std::string& debugText);
	void RenderDebugText(const std::string& debugText);

	ComPtr<ID3D11Texture2D> GetTexture();
private:
	bool m_debugFrameIndex;
	CD3DRender *m_pD3DRender;
	int m_renderWidth;
	int m_renderHeight;
	ComPtr<ID3D11Texture2D> m_pStagingTexture;

	ComPtr<ID3D11VertexShader> m_pVertexShader;
	ComPtr<ID3D11PixelShader> m_pPixelShader;

	ComPtr<ID3D11InputLayout> m_pVertexLayout;
	ComPtr<ID3D11Buffer> m_pVertexBuffer;
	ComPtr<ID3D11Buffer> m_pIndexBuffer;

	ComPtr<ID3D11SamplerState> m_pSamplerLinear;

	ComPtr<ID3D11Texture2D> m_pDepthStencil;
	ComPtr<ID3D11ShaderResourceView> m_pShaderResourceView[2];
	ComPtr<ID3D11RenderTargetView> m_pRenderTargetView;
	ComPtr<ID3D11DepthStencilView> m_pDepthStencilView;

	std::unique_ptr<DirectX::SpriteFont> m_Font;
	std::unique_ptr<DirectX::SpriteBatch> m_SpriteBatch;

	uint64_t m_frameIndex2;
	struct SimpleVertex
	{
		DirectX::XMFLOAT3 Pos;
		DirectX::XMFLOAT2 Tex;
	};
};

