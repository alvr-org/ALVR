#include "FrameRender.h"
#include "Utils.h"
#include "Logger.h"
#include "resource.h"

extern HINSTANCE g_hInstance;

static const char *VERTEX_SHADER =
"Texture2D txLeft : register(t0);\n"
"Texture2D txRight : register(t1);\n"
"SamplerState samLinear : register(s0);\n"
"\n"
"struct VS_INPUT\n"
"{\n"
"	float4 Pos : POSITION;\n"
"	float2 Tex : TEXCOORD0;\n"
"};\n"
"\n"
"struct PS_INPUT\n"
"{\n"
"	float4 Pos : SV_POSITION;\n"
"	float2 Tex : TEXCOORD0;\n"
"};\n"
"PS_INPUT VS(VS_INPUT input)\n"
"{\n"
"	PS_INPUT output = (PS_INPUT)0;\n"
"	output.Pos = input.Pos;\n"
"	output.Tex = input.Tex;\n"
"\n"
"	return output;\n"
"}\n"
"float4 PS(PS_INPUT input) : SV_Target\n"
"{\n"
//"float offset = (1448.0 - 1024.0) / 2 / 1448.0;\n"
"float offset = 0.0;\n"
"float shrink_to = 1.0 - offset * 2;\n"
"float x = input.Tex.x;\n"
"float y = input.Tex.y;\n"
"	if (input.Tex.x < 0.5){\n"
"		x = x * 2;\n"
"		x = x * shrink_to + offset;\n"
"		y = y * shrink_to + offset;\n"
"		return txLeft.Sample(samLinear, float2(1.0 - x, 1.0 - y)); // We need this hack, because We cloud not resolve upside down issue by changing texcoord in buffer.\n"
"	}else{\n"
"		x = x * 2 - 1.0;\n"
"		x = x * shrink_to + offset;\n"
"		y = y * shrink_to + offset;\n"
"		return txLeft.Sample(samLinear, float2(1.0 - x, 1.0 - y)); // We need this hack, because We cloud not resolve upside down issue by changing texcoord in buffer.\n"
"	}\n"
"}\n";
static const char *PIXEL_SHADER = VERTEX_SHADER;


FrameRender::FrameRender(int renderWidth, int renderHeight, bool debugFrameIndex, CD3DRender *pD3DRender)
	: m_renderWidth(renderWidth)
	, m_renderHeight(renderHeight)
	, m_pD3DRender(pD3DRender)
	, m_debugFrameIndex(debugFrameIndex)
{
}


FrameRender::~FrameRender()
{
}

bool FrameRender::Startup(ID3D11Texture2D * pTexture[])
{
	if (m_pStagingTexture) {
		return true;
	}
	D3D11_TEXTURE2D_DESC srcDesc;
	pTexture[0]->GetDesc(&srcDesc);

	D3D11_TEXTURE2D_DESC stagingTextureDesc;
	ZeroMemory(&stagingTextureDesc, sizeof(stagingTextureDesc));
	stagingTextureDesc.Width = m_renderWidth * 2;
	stagingTextureDesc.Height = m_renderHeight;
	stagingTextureDesc.Format = srcDesc.Format;
	stagingTextureDesc.MipLevels = 1;
	stagingTextureDesc.ArraySize = 1;
	stagingTextureDesc.SampleDesc.Count = 1;
	stagingTextureDesc.Usage = D3D11_USAGE_DEFAULT;
	//stagingTextureDesc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
	stagingTextureDesc.BindFlags = D3D11_BIND_SHADER_RESOURCE | D3D11_BIND_RENDER_TARGET;

	if (FAILED(m_pD3DRender->GetDevice()->CreateTexture2D(&stagingTextureDesc, NULL, &m_pStagingTexture)))
	{
		Log("Failed to create staging texture!");
		return false;
	}

	HRESULT hr = m_pD3DRender->GetDevice()->CreateRenderTargetView(m_pStagingTexture.Get(), NULL, &m_pRenderTargetView);
	if (FAILED(hr)) {
		Log("CreateRenderTargetView %p %s", hr, GetDxErrorStr(hr).c_str());
		return false;
	}

	// Create depth stencil texture
	D3D11_TEXTURE2D_DESC descDepth;
	ZeroMemory(&descDepth, sizeof(descDepth));
	descDepth.Width = stagingTextureDesc.Width;
	descDepth.Height = stagingTextureDesc.Height;
	descDepth.MipLevels = 1;
	descDepth.ArraySize = 1;
	descDepth.Format = DXGI_FORMAT_D24_UNORM_S8_UINT;
	descDepth.SampleDesc.Count = 1;
	descDepth.SampleDesc.Quality = 0;
	descDepth.Usage = D3D11_USAGE_DEFAULT;
	descDepth.BindFlags = D3D11_BIND_DEPTH_STENCIL;
	descDepth.CPUAccessFlags = 0;
	descDepth.MiscFlags = 0;
	hr = m_pD3DRender->GetDevice()->CreateTexture2D(&descDepth, nullptr, &m_pDepthStencil);
	if (FAILED(hr)) {
		Log("CreateTexture2D %p %s", hr, GetDxErrorStr(hr).c_str());
		return false;
	}


	// Create the depth stencil view
	D3D11_DEPTH_STENCIL_VIEW_DESC descDSV;
	ZeroMemory(&descDSV, sizeof(descDSV));
	descDSV.Format = descDepth.Format;
	descDSV.ViewDimension = D3D11_DSV_DIMENSION_TEXTURE2D;
	descDSV.Texture2D.MipSlice = 0;
	hr = m_pD3DRender->GetDevice()->CreateDepthStencilView(m_pDepthStencil.Get(), &descDSV, &m_pDepthStencilView);
	if (FAILED(hr)) {
		Log("CreateDepthStencilView %p %s", hr, GetDxErrorStr(hr).c_str());
		return false;
	}

	m_pD3DRender->GetContext()->OMSetRenderTargets(1, m_pRenderTargetView.GetAddressOf(), m_pDepthStencilView.Get());

	D3D11_VIEWPORT viewport;
	viewport.Width = (float)m_renderWidth * 2;
	viewport.Height = (float)m_renderHeight;
	viewport.MinDepth = 0.0f;
	viewport.MaxDepth = 1.0f;
	viewport.TopLeftX = 0;
	viewport.TopLeftY = 0;
	m_pD3DRender->GetContext()->RSSetViewports(1, &viewport);


	ID3DBlob *vshader, *pshader, *error;

	hr = D3DCompile(VERTEX_SHADER, strlen(VERTEX_SHADER), "vs", NULL, NULL, "VS", "vs_4_0", 0, 0, &vshader, &error);
	Log("D3DCompile vs %p", hr);
	if (FAILED(hr)) {
		Log("%s", error->GetBufferPointer());
		return false;
	}
	if (error != NULL) {
		error->Release();
		error = NULL;
	}

	hr = m_pD3DRender->GetDevice()->CreateVertexShader((const DWORD*)vshader->GetBufferPointer(), vshader->GetBufferSize(), NULL, &m_pVertexShader);
	if (FAILED(hr)) {
		Log("CreateVertexShader %p %s", hr, GetDxErrorStr(hr).c_str());
		return false;
	}
	hr = D3DCompile(VERTEX_SHADER, strlen(VERTEX_SHADER), "ps", NULL, NULL, "PS", "ps_4_0", 0, 0, &pshader, &error);
	Log("D3DCompile ps %p", hr);
	if (FAILED(hr)) {
		Log("%s", error->GetBufferPointer());
		return false;
	}
	if (error != NULL) {
		error->Release();
	}

	hr = m_pD3DRender->GetDevice()->CreatePixelShader((const DWORD*)pshader->GetBufferPointer(), pshader->GetBufferSize(), NULL, &m_pPixelShader);
	if (FAILED(hr)) {
		Log("CreatePixelShader %p %s", hr, GetDxErrorStr(hr).c_str());
		return false;
	}

	// Define the input layout
	D3D11_INPUT_ELEMENT_DESC layout[] =
	{
		{ "POSITION", 0, DXGI_FORMAT_R32G32B32_FLOAT, 0, 0, D3D11_INPUT_PER_VERTEX_DATA, 0 },
	{ "TEXCOORD", 0, DXGI_FORMAT_R32G32_FLOAT, 0, 12, D3D11_INPUT_PER_VERTEX_DATA, 0 },
	};
	UINT numElements = ARRAYSIZE(layout);


	// Create the input layout
	hr = m_pD3DRender->GetDevice()->CreateInputLayout(layout, numElements, vshader->GetBufferPointer(),
		vshader->GetBufferSize(), &m_pVertexLayout);
	if (FAILED(hr)) {
		Log("CreateInputLayout %p %s", hr, GetDxErrorStr(hr).c_str());
		return false;
	}
	vshader->Release();

	// Set the input layout
	m_pD3DRender->GetContext()->IASetInputLayout(m_pVertexLayout.Get());

	// src textures has 1448x1448 pixels but dest texture(remote display) has 1024x1024 pixels.
	// Apply offset to crop center of src textures.
	float tex_offset = (1448 - 1024) / 2 / 1448.0;
	tex_offset = 0;

	// Create vertex buffer
	SimpleVertex vertices[] =
	{
		{ DirectX::XMFLOAT3(-1.0f, -1.0f, 0.5f), DirectX::XMFLOAT2(1.0f - tex_offset, 0.0f + tex_offset) },
	{ DirectX::XMFLOAT3(1.0f,  1.0f, 0.5f), DirectX::XMFLOAT2(0.0f + tex_offset, 1.0f - tex_offset) },
	{ DirectX::XMFLOAT3(1.0f, -1.0f, 0.5f), DirectX::XMFLOAT2(0.0f + tex_offset, 0.0f + tex_offset) },
	{ DirectX::XMFLOAT3(-1.0f,  1.0f, 0.5f), DirectX::XMFLOAT2(1.0f - tex_offset, 1.0f - tex_offset) },
	};

	D3D11_BUFFER_DESC bd;
	ZeroMemory(&bd, sizeof(bd));
	bd.Usage = D3D11_USAGE_DEFAULT;
	bd.ByteWidth = sizeof(SimpleVertex) * 4;
	bd.BindFlags = D3D11_BIND_VERTEX_BUFFER;
	bd.CPUAccessFlags = 0;
	D3D11_SUBRESOURCE_DATA InitData;
	ZeroMemory(&InitData, sizeof(InitData));
	InitData.pSysMem = vertices;
	hr = m_pD3DRender->GetDevice()->CreateBuffer(&bd, &InitData, &m_pVertexBuffer);
	if (FAILED(hr)) {
		Log("CreateBuffer 1 %p %s", hr, GetDxErrorStr(hr).c_str());
		return false;
	}

	// Set vertex buffer
	UINT stride = sizeof(SimpleVertex);
	UINT offset = 0;
	m_pD3DRender->GetContext()->IASetVertexBuffers(0, 1, m_pVertexBuffer.GetAddressOf(), &stride, &offset);

	// Create index buffer
	// Create vertex buffer
	WORD indices[] =
	{
		0,1,2,
		0,3,1
	};

	bd.Usage = D3D11_USAGE_DEFAULT;
	bd.ByteWidth = sizeof(WORD) * 6;
	bd.BindFlags = D3D11_BIND_INDEX_BUFFER;
	bd.CPUAccessFlags = 0;
	InitData.pSysMem = indices;
	hr = m_pD3DRender->GetDevice()->CreateBuffer(&bd, &InitData, &m_pIndexBuffer);
	if (FAILED(hr)) {
		Log("CreateBuffer 2 %p %s", hr, GetDxErrorStr(hr).c_str());
		return false;
	}

	// Set index buffer
	m_pD3DRender->GetContext()->IASetIndexBuffer(m_pIndexBuffer.Get(), DXGI_FORMAT_R16_UINT, 0);

	// Set primitive topology
	m_pD3DRender->GetContext()->IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);

	// Create the sample state
	D3D11_SAMPLER_DESC sampDesc;
	ZeroMemory(&sampDesc, sizeof(sampDesc));
	sampDesc.Filter = D3D11_FILTER_MIN_MAG_MIP_LINEAR;
	sampDesc.AddressU = D3D11_TEXTURE_ADDRESS_WRAP;
	sampDesc.AddressV = D3D11_TEXTURE_ADDRESS_WRAP;
	sampDesc.AddressW = D3D11_TEXTURE_ADDRESS_WRAP;
	sampDesc.ComparisonFunc = D3D11_COMPARISON_NEVER;
	sampDesc.MinLOD = 0;
	sampDesc.MaxLOD = D3D11_FLOAT32_MAX;
	hr = m_pD3DRender->GetDevice()->CreateSamplerState(&sampDesc, &m_pSamplerLinear);
	if (FAILED(hr)) {
		Log("CreateSamplerState 5 %p %s", hr, GetDxErrorStr(hr).c_str());
		return false;
	}

	HRSRC fontResource = FindResource(g_hInstance, MAKEINTRESOURCE(IDR_FONT), RT_RCDATA);

	m_Font = std::make_unique<DirectX::SpriteFont>(m_pD3DRender->GetDevice(), L"C:\\src\\virtual_display\\driver_virtual_display\\resources\\inconsolata.spritefont");
	m_SpriteBatch = std::make_unique<DirectX::SpriteBatch>(m_pD3DRender->GetContext());

	Log("Staging Texture created");

	return true;
}


bool FrameRender::RenderFrame(ID3D11Texture2D * pTexture[], int textureNum, const std::string& debugText)
{

	D3D11_TEXTURE2D_DESC srcDesc;
	pTexture[0]->GetDesc(&srcDesc);

	Log("RenderFrame %dx%d %d", srcDesc.Width, srcDesc.Height, srcDesc.Format);
	
	if (textureNum == 1) {
		m_pD3DRender->GetContext()->CopyResource(m_pStagingTexture.Get(), pTexture[0]);
	}
	else {
		D3D11_SHADER_RESOURCE_VIEW_DESC SRVDesc = {};
		SRVDesc.Format = srcDesc.Format;
		SRVDesc.ViewDimension = D3D11_SRV_DIMENSION_TEXTURE2D;
		SRVDesc.Texture2D.MostDetailedMip = 0;
		SRVDesc.Texture2D.MipLevels = 1;

		HRESULT hr = m_pD3DRender->GetDevice()->CreateShaderResourceView(pTexture[0], &SRVDesc, m_pShaderResourceView[0].ReleaseAndGetAddressOf());
		if (FAILED(hr)) {
			Log("CreateShaderResourceView %p %s", hr, GetDxErrorStr(hr).c_str());
			return false;
		}
		hr = m_pD3DRender->GetDevice()->CreateShaderResourceView(pTexture[1], &SRVDesc, m_pShaderResourceView[1].ReleaseAndGetAddressOf());
		if (FAILED(hr)) {
			Log("CreateShaderResourceView %p %s", hr, GetDxErrorStr(hr).c_str());
			return false;
		}

		m_pD3DRender->GetContext()->OMSetRenderTargets(1, m_pRenderTargetView.GetAddressOf(), m_pDepthStencilView.Get());

		D3D11_VIEWPORT viewport;
		viewport.Width = (float)m_renderWidth * 2;
		viewport.Height = (float)m_renderHeight;
		viewport.MinDepth = 0.0f;
		viewport.MaxDepth = 1.0f;
		viewport.TopLeftX = 0;
		viewport.TopLeftY = 0;
		m_pD3DRender->GetContext()->RSSetViewports(1, &viewport);

		// Set the input layout
		m_pD3DRender->GetContext()->IASetInputLayout(m_pVertexLayout.Get());


		// Set vertex buffer
		UINT stride = sizeof(SimpleVertex);
		UINT offset = 0;
		m_pD3DRender->GetContext()->IASetVertexBuffers(0, 1, m_pVertexBuffer.GetAddressOf(), &stride, &offset);

		// Set index buffer
		m_pD3DRender->GetContext()->IASetIndexBuffer(m_pIndexBuffer.Get(), DXGI_FORMAT_R16_UINT, 0);

		// Set primitive topology
		m_pD3DRender->GetContext()->IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);

		// Clear the back buffer
		m_pD3DRender->GetContext()->ClearRenderTargetView(m_pRenderTargetView.Get(), DirectX::Colors::MidnightBlue);

		// Clear the depth buffer to 1.0 (max depth)
		m_pD3DRender->GetContext()->ClearDepthStencilView(m_pDepthStencilView.Get(), D3D11_CLEAR_DEPTH, 1.0f, 0);

		// Render the cube
		m_pD3DRender->GetContext()->VSSetShader(m_pVertexShader.Get(), nullptr, 0);
		m_pD3DRender->GetContext()->PSSetShader(m_pPixelShader.Get(), nullptr, 0);

		ID3D11ShaderResourceView *shaderResourceView[2] = { m_pShaderResourceView[0].Get(), m_pShaderResourceView[1].Get() };
		m_pD3DRender->GetContext()->PSSetShaderResources(0, 2, shaderResourceView);
		//m_pD3DRender->GetContext()->PSSetShaderResources(0, 1, shaderResourceView);

		m_pD3DRender->GetContext()->PSSetSamplers(0, 1, m_pSamplerLinear.GetAddressOf());
		m_pD3DRender->GetContext()->DrawIndexed(6, 0, 0);

		RenderDebugText(debugText);

		m_pD3DRender->GetContext()->Flush();
	}

	return false;
}


void FrameRender::RenderDebugText(const std::string & debugText)
{
	if (!m_debugFrameIndex) {
		return;
	}

	m_SpriteBatch->Begin();

	std::vector<wchar_t> buf(debugText.size() + 1);
	_snwprintf_s(&buf[0], buf.size(), buf.size(), L"%hs", debugText.c_str());

	DirectX::SimpleMath::Vector2 origin = m_Font->MeasureString(&buf[0]);

	DirectX::SimpleMath::Vector2 FontPos;
	FontPos.x = 100;
	FontPos.y = 100;

	m_Font->DrawString(m_SpriteBatch.get(), &buf[0],
		FontPos, DirectX::Colors::Green, 0.f);

	m_SpriteBatch->End();
}

ComPtr<ID3D11Texture2D> FrameRender::GetTexture()
{
	return m_pStagingTexture;
}

