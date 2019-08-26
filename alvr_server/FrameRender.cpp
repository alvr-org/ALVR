#include "FrameRender.h"
#include "Utils.h"
#include "Logger.h"
#include "resource.h"
#include "Settings.h"
#include "WICTextureLoader.h"

extern uint64_t g_DriverTestMode;


FrameRender::FrameRender(std::shared_ptr<CD3DRender> pD3DRender)
	: m_pD3DRender(pD3DRender)
{
}


FrameRender::~FrameRender()
{
}

bool FrameRender::Startup()
{
	if (m_pStagingTexture) {
		return true;
	}

	//
	// Create staging texture
	// This is input texture of Video Encoder and is render target of both eyes.
	//

	D3D11_TEXTURE2D_DESC stagingTextureDesc;
	ZeroMemory(&stagingTextureDesc, sizeof(stagingTextureDesc));
	stagingTextureDesc.Width = Settings::Instance().m_renderWidth;
	stagingTextureDesc.Height = Settings::Instance().m_renderHeight;
	stagingTextureDesc.Format = DXGI_FORMAT_R8G8B8A8_UNORM_SRGB;
	stagingTextureDesc.MipLevels = 1;
	stagingTextureDesc.ArraySize = 1;
	stagingTextureDesc.SampleDesc.Count = 1;
	stagingTextureDesc.Usage = D3D11_USAGE_DEFAULT;
	stagingTextureDesc.BindFlags = D3D11_BIND_SHADER_RESOURCE | D3D11_BIND_RENDER_TARGET;

	if (FAILED(m_pD3DRender->GetDevice()->CreateTexture2D(&stagingTextureDesc, NULL, &m_pStagingTexture)))
	{
		Log(L"Failed to create staging texture!");
		return false;
	}

	HRESULT hr = m_pD3DRender->GetDevice()->CreateRenderTargetView(m_pStagingTexture.Get(), NULL, &m_pRenderTargetView);
	if (FAILED(hr)) {
		Log(L"CreateRenderTargetView %p %s", hr, GetErrorStr(hr).c_str());
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
		Log(L"CreateTexture2D %p %s", hr, GetErrorStr(hr).c_str());
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
		Log(L"CreateDepthStencilView %p %s", hr, GetErrorStr(hr).c_str());
		return false;
	}

	m_pD3DRender->GetContext()->OMSetRenderTargets(1, m_pRenderTargetView.GetAddressOf(), m_pDepthStencilView.Get());

	D3D11_VIEWPORT viewport;
	viewport.Width = (float)Settings::Instance().m_renderWidth;
	viewport.Height = (float)Settings::Instance().m_renderHeight;
	viewport.MinDepth = 0.0f;
	viewport.MaxDepth = 1.0f;
	viewport.TopLeftX = 0;
	viewport.TopLeftY = 0;
	m_pD3DRender->GetContext()->RSSetViewports(1, &viewport);

	//
	// Compile shaders
	//
	
	std::vector<uint8_t> vshader;
	if (!ReadBinaryResource(vshader, IDR_VS)) {
		Log(L"Failed to load resource for IDR_VS.");
		return false;
	}

	hr = m_pD3DRender->GetDevice()->CreateVertexShader((const DWORD*)&vshader[0], vshader.size(), NULL, &m_pVertexShader);
	if (FAILED(hr)) {
		Log(L"CreateVertexShader %p %s", hr, GetErrorStr(hr).c_str());
		return false;
	}

	std::vector<uint8_t> pshader;
	if (!ReadBinaryResource(pshader, IDR_PS)) {
		Log(L"Failed to load resource for IDR_PS.");
		return false;
	}

	hr = m_pD3DRender->GetDevice()->CreatePixelShader((const DWORD*)&pshader[0], pshader.size(), NULL, &m_pPixelShader);
	if (FAILED(hr)) {
		Log(L"CreatePixelShader %p %s", hr, GetErrorStr(hr).c_str());
		return false;
	}

	//
	// Create input layout
	//

	// Define the input layout
	D3D11_INPUT_ELEMENT_DESC layout[] =
	{
		{ "POSITION", 0, DXGI_FORMAT_R32G32B32_FLOAT, 0, 0, D3D11_INPUT_PER_VERTEX_DATA, 0 },
	{ "TEXCOORD", 0, DXGI_FORMAT_R32G32_FLOAT, 0, 12, D3D11_INPUT_PER_VERTEX_DATA, 0 },
	{ "VIEW", 0, DXGI_FORMAT_R32_UINT, 0, 20, D3D11_INPUT_PER_VERTEX_DATA, 0 },
	};
	UINT numElements = ARRAYSIZE(layout);


	// Create the input layout
	hr = m_pD3DRender->GetDevice()->CreateInputLayout(layout, numElements, &vshader[0],
		vshader.size(), &m_pVertexLayout);
	if (FAILED(hr)) {
		Log(L"CreateInputLayout %p %s", hr, GetErrorStr(hr).c_str());
		return false;
	}

	// Set the input layout
	m_pD3DRender->GetContext()->IASetInputLayout(m_pVertexLayout.Get());

	//
	// Create vertex buffer
	//

	// Src texture has various geometry and we should use the part of the textures.
	// That part are defined by uv-coordinates of "bounds" passed to IVRDriverDirectModeComponent::SubmitLayer.
	// So we should update uv-coordinates for every frames and layers.
	D3D11_BUFFER_DESC bd;
	ZeroMemory(&bd, sizeof(bd));
	bd.Usage = D3D11_USAGE_DYNAMIC;
	bd.ByteWidth = sizeof(SimpleVertex) * 8;
	bd.BindFlags = D3D11_BIND_VERTEX_BUFFER;
	bd.CPUAccessFlags = D3D11_CPU_ACCESS_WRITE;

	hr = m_pD3DRender->GetDevice()->CreateBuffer(&bd, NULL, &m_pVertexBuffer);
	if (FAILED(hr)) {
		Log(L"CreateBuffer 1 %p %s", hr, GetErrorStr(hr).c_str());
		return false;
	}

	// Set vertex buffer
	UINT stride = sizeof(SimpleVertex);
	UINT offset = 0;
	m_pD3DRender->GetContext()->IASetVertexBuffers(0, 1, m_pVertexBuffer.GetAddressOf(), &stride, &offset);
	
	//
	// Create index buffer
	//

	WORD indices[] =
	{
		0,1,2,
		0,3,1,

		4,5,6,
		4,7,5
	};

	bd.Usage = D3D11_USAGE_DEFAULT;
	bd.ByteWidth = sizeof(indices);
	bd.BindFlags = D3D11_BIND_INDEX_BUFFER;
	bd.CPUAccessFlags = 0;

	D3D11_SUBRESOURCE_DATA InitData;
	ZeroMemory(&InitData, sizeof(InitData));
	InitData.pSysMem = indices;

	hr = m_pD3DRender->GetDevice()->CreateBuffer(&bd, &InitData, &m_pIndexBuffer);
	if (FAILED(hr)) {
		Log(L"CreateBuffer 2 %p %s", hr, GetErrorStr(hr).c_str());
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
		Log(L"CreateSamplerState %p %s", hr, GetErrorStr(hr).c_str());
		return false;
	}

	//
	// Load spritefont for debug text output
	//

	std::vector<uint8_t> fontBuffer;
	if (ReadBinaryResource(fontBuffer, IDR_FONT)) {
		m_Font = std::make_unique<DirectX::SpriteFont>(m_pD3DRender->GetDevice(), (uint8_t *)&fontBuffer[0], fontBuffer.size());
		m_SpriteBatch = std::make_unique<DirectX::SpriteBatch>(m_pD3DRender->GetContext());
	}
	else {
		Log(L"FindResource failed %d", GetLastError());
	}

	//
	// Create alpha blend state
	// We need alpha blending to support layer.
	//

	// BlendState for first layer.
	// Some VR apps (like StreamVR Home beta) submit the texture that alpha is zero on all pixels.
	// So we need to ignore alpha of first layer.
	D3D11_BLEND_DESC BlendDesc;
	ZeroMemory(&BlendDesc, sizeof(BlendDesc));
	BlendDesc.AlphaToCoverageEnable = FALSE;
	BlendDesc.IndependentBlendEnable = FALSE;
	for (int i = 0; i < 8; i++) {
		BlendDesc.RenderTarget[i].BlendEnable = TRUE;
		BlendDesc.RenderTarget[i].SrcBlend = D3D11_BLEND_ONE;
		BlendDesc.RenderTarget[i].DestBlend = D3D11_BLEND_ZERO;
		BlendDesc.RenderTarget[i].BlendOp = D3D11_BLEND_OP_ADD;
		BlendDesc.RenderTarget[i].SrcBlendAlpha = D3D11_BLEND_ONE;
		BlendDesc.RenderTarget[i].DestBlendAlpha = D3D11_BLEND_ZERO;
		BlendDesc.RenderTarget[i].BlendOpAlpha = D3D11_BLEND_OP_ADD;
		BlendDesc.RenderTarget[i].RenderTargetWriteMask = D3D11_COLOR_WRITE_ENABLE_RED | D3D11_COLOR_WRITE_ENABLE_GREEN | D3D11_COLOR_WRITE_ENABLE_BLUE;
	}

	hr = m_pD3DRender->GetDevice()->CreateBlendState(&BlendDesc, &m_pBlendStateFirst);
	if (FAILED(hr)) {
		Log(L"CreateBlendState %p %s", hr, GetErrorStr(hr).c_str());
		return false;
	}

	// BleandState for other layers than first.
	BlendDesc.AlphaToCoverageEnable = FALSE;
	BlendDesc.IndependentBlendEnable = FALSE;
	for (int i = 0; i < 8; i++) {
		BlendDesc.RenderTarget[i].BlendEnable = TRUE;
		BlendDesc.RenderTarget[i].SrcBlend = D3D11_BLEND_SRC_ALPHA;
		BlendDesc.RenderTarget[i].DestBlend = D3D11_BLEND_INV_SRC_ALPHA;
		BlendDesc.RenderTarget[i].BlendOp = D3D11_BLEND_OP_ADD;
		BlendDesc.RenderTarget[i].SrcBlendAlpha = D3D11_BLEND_ONE;
		BlendDesc.RenderTarget[i].DestBlendAlpha = D3D11_BLEND_ZERO;
		BlendDesc.RenderTarget[i].BlendOpAlpha = D3D11_BLEND_OP_ADD;
		BlendDesc.RenderTarget[i].RenderTargetWriteMask = D3D11_COLOR_WRITE_ENABLE_ALL;
	}

	hr = m_pD3DRender->GetDevice()->CreateBlendState(&BlendDesc, &m_pBlendState);
	if (FAILED(hr)) {
		Log(L"CreateBlendState %p %s", hr, GetErrorStr(hr).c_str());
		return false;
	}

	CreateResourceTexture();

	enableFFR = Settings::Instance().m_foveationStrengthMean > 0.;

	if (enableFFR) {
		m_ffr = std::make_unique<FFR>(m_pD3DRender->GetDevice());
		m_ffr->Initialize(m_pStagingTexture.Get());
	}

	Log(L"Staging Texture created");

	return true;
}


bool FrameRender::RenderFrame(ID3D11Texture2D *pTexture[][2], vr::VRTextureBounds_t bounds[][2], int layerCount, bool recentering, const std::string &message, const std::string& debugText)
{
	// Set render target
	m_pD3DRender->GetContext()->OMSetRenderTargets(1, m_pRenderTargetView.GetAddressOf(), m_pDepthStencilView.Get());

	// Set viewport
	D3D11_VIEWPORT viewport;
	viewport.Width = (float)Settings::Instance().m_renderWidth;
	viewport.Height = (float)Settings::Instance().m_renderHeight;
	viewport.MinDepth = 0.0f;
	viewport.MaxDepth = 1.0f;
	viewport.TopLeftX = 0;
	viewport.TopLeftY = 0;
	m_pD3DRender->GetContext()->RSSetViewports(1, &viewport);

	// Clear the back buffer
	m_pD3DRender->GetContext()->ClearRenderTargetView(m_pRenderTargetView.Get(), DirectX::Colors::MidnightBlue);

	// Overlay recentering texture on top of all layers.
	int recenterLayer = -1;
	if (recentering) {
		recenterLayer = layerCount;
		layerCount++;
	}

	for (int i = 0; i < layerCount; i++) {
		ID3D11Texture2D *textures[2];
		vr::VRTextureBounds_t bound[2];

		if (i == recenterLayer) {
			textures[0] = (ID3D11Texture2D *)m_recenterTexture.Get();
			textures[1] = (ID3D11Texture2D *)m_recenterTexture.Get();
			bound[0].uMin = bound[0].vMin = bound[1].uMin = bound[1].vMin = 0.0f;
			bound[0].uMax = bound[0].vMax = bound[1].uMax = bound[1].vMax = 1.0f;
		}
		else {
			textures[0] = pTexture[i][0];
			textures[1] = pTexture[i][1];
			bound[0] = bounds[i][0];
			bound[1] = bounds[i][1];
		}
		if (textures[0] == NULL || textures[1] == NULL) {
			Log(L"Ignore NULL layer. layer=%d/%d%s%s", i, layerCount
				, recentering ? L" (recentering)" : L"", !message.empty() ? L" (message)" : L"");
			continue;
		}

		D3D11_TEXTURE2D_DESC srcDesc;
		textures[0]->GetDesc(&srcDesc);

		Log(L"RenderFrame layer=%d/%d %dx%d %d%s%s", i, layerCount, srcDesc.Width, srcDesc.Height, srcDesc.Format
			, recentering ? L" (recentering)" : L"", !message.empty() ? L" (message)" : L"");

		D3D11_SHADER_RESOURCE_VIEW_DESC SRVDesc = {};
		SRVDesc.Format = srcDesc.Format;
		SRVDesc.ViewDimension = D3D11_SRV_DIMENSION_TEXTURE2D;
		SRVDesc.Texture2D.MostDetailedMip = 0;
		SRVDesc.Texture2D.MipLevels = 1;

		ComPtr<ID3D11ShaderResourceView> pShaderResourceView[2];

		HRESULT hr = m_pD3DRender->GetDevice()->CreateShaderResourceView(textures[0], &SRVDesc, pShaderResourceView[0].ReleaseAndGetAddressOf());
		if (FAILED(hr)) {
			Log(L"CreateShaderResourceView %p %s", hr, GetErrorStr(hr).c_str());
			return false;
		}
		hr = m_pD3DRender->GetDevice()->CreateShaderResourceView(textures[1], &SRVDesc, pShaderResourceView[1].ReleaseAndGetAddressOf());
		if (FAILED(hr)) {
			Log(L"CreateShaderResourceView %p %s", hr, GetErrorStr(hr).c_str());
			return false;
		}
		
		if (i == 0) {
			m_pD3DRender->GetContext()->OMSetBlendState(m_pBlendStateFirst.Get(), NULL, 0xffffffff);
		}
		else {
			m_pD3DRender->GetContext()->OMSetBlendState(m_pBlendState.Get(), NULL, 0xffffffff);
		}
		
		// Clear the depth buffer to 1.0 (max depth)
		// We need clear depth buffer to correctly render layers.
		m_pD3DRender->GetContext()->ClearDepthStencilView(m_pDepthStencilView.Get(), D3D11_CLEAR_DEPTH, 1.0f, 0);

		//
		// Update uv-coordinates in vertex buffer according to bounds.
		//

		SimpleVertex vertices[] =
		{
			// Left View
			{ DirectX::XMFLOAT3(-1.0f, -1.0f, 0.5f), DirectX::XMFLOAT2(bound[0].uMin, bound[0].vMax), 0 },
		{ DirectX::XMFLOAT3(0.0f,  1.0f, 0.5f), DirectX::XMFLOAT2(bound[0].uMax, bound[0].vMin), 0 },
		{ DirectX::XMFLOAT3(0.0f, -1.0f, 0.5f), DirectX::XMFLOAT2(bound[0].uMax, bound[0].vMax), 0 },
		{ DirectX::XMFLOAT3(-1.0f,  1.0f, 0.5f), DirectX::XMFLOAT2(bound[0].uMin, bound[0].vMin), 0 },
		// Right View
		{ DirectX::XMFLOAT3(0.0f, -1.0f, 0.5f), DirectX::XMFLOAT2(bound[1].uMin, bound[1].vMax), 1 },
		{ DirectX::XMFLOAT3(1.0f,  1.0f, 0.5f), DirectX::XMFLOAT2(bound[1].uMax, bound[1].vMin), 1 },
		{ DirectX::XMFLOAT3(1.0f, -1.0f, 0.5f), DirectX::XMFLOAT2(bound[1].uMax, bound[1].vMax), 1 },
		{ DirectX::XMFLOAT3(0.0f,  1.0f, 0.5f), DirectX::XMFLOAT2(bound[1].uMin, bound[1].vMin), 1 },
		};

		// TODO: Which is better? UpdateSubresource or Map
		//m_pD3DRender->GetContext()->UpdateSubresource(m_pVertexBuffer.Get(), 0, nullptr, &vertices, 0, 0);

		D3D11_MAPPED_SUBRESOURCE mapped = { 0 };
		hr = m_pD3DRender->GetContext()->Map(m_pVertexBuffer.Get(), 0, D3D11_MAP_WRITE_DISCARD, 0, &mapped);
		if (FAILED(hr)) {
			Log(L"Map %p %s", hr, GetErrorStr(hr).c_str());
			return false;
		}
		memcpy(mapped.pData, vertices, sizeof(vertices));

		m_pD3DRender->GetContext()->Unmap(m_pVertexBuffer.Get(), 0);

		// Set the input layout
		m_pD3DRender->GetContext()->IASetInputLayout(m_pVertexLayout.Get());

		//
		// Set buffers
		//

		UINT stride = sizeof(SimpleVertex);
		UINT offset = 0;
		m_pD3DRender->GetContext()->IASetVertexBuffers(0, 1, m_pVertexBuffer.GetAddressOf(), &stride, &offset);

		m_pD3DRender->GetContext()->IASetIndexBuffer(m_pIndexBuffer.Get(), DXGI_FORMAT_R16_UINT, 0);
		m_pD3DRender->GetContext()->IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);

		//
		// Set shaders
		//

		m_pD3DRender->GetContext()->VSSetShader(m_pVertexShader.Get(), nullptr, 0);
		m_pD3DRender->GetContext()->PSSetShader(m_pPixelShader.Get(), nullptr, 0);

		ID3D11ShaderResourceView *shaderResourceView[2] = { pShaderResourceView[0].Get(), pShaderResourceView[1].Get() };
		m_pD3DRender->GetContext()->PSSetShaderResources(0, 2, shaderResourceView);

		m_pD3DRender->GetContext()->PSSetSamplers(0, 1, m_pSamplerLinear.GetAddressOf());
		
		//
		// Draw
		//

		m_pD3DRender->GetContext()->DrawIndexed(VERTEX_INDEX_COUNT, 0, 0);
	}

	if (!message.empty()) {
		RenderMessage(message);
	}
	RenderDebugText(debugText);

	if (enableFFR) {
		m_ffr->Render();
	}

	m_pD3DRender->GetContext()->Flush();

	return true;
}


void FrameRender::RenderMessage(const std::string &message)
{
	m_SpriteBatch->Begin();

	std::vector<wchar_t> buf(message.size() + 1);
	_snwprintf_s(&buf[0], buf.size(), buf.size(), L"%hs", message.c_str());

	DirectX::SimpleMath::Vector2 origin = m_Font->MeasureString(&buf[0]);

	int eyeWidth = Settings::Instance().m_renderWidth / 2;
	int x = Settings::Instance().m_renderWidth / 6;
	int y = Settings::Instance().m_renderHeight / 3;
	float scale = Settings::Instance().m_renderWidth / (float)3072;

	RECT rc;
	rc.left = x - 10;
	rc.top = y - 10;
	rc.right = x + static_cast<int>(origin.x * scale) + 10;
	rc.bottom = y + static_cast<int>(origin.y * scale) + 10;
	m_SpriteBatch->Draw(m_messageBGResourceView.Get(), rc);
	rc.left += eyeWidth;
	rc.right += eyeWidth;
	m_SpriteBatch->Draw(m_messageBGResourceView.Get(), rc);

	DirectX::SimpleMath::Vector2 FontPos;
	FontPos.x = static_cast<float>(x);
	FontPos.y = static_cast<float>(y);

	m_Font->DrawString(m_SpriteBatch.get(), &buf[0],
		FontPos, DirectX::Colors::Gray, 0.f, DirectX::XMFLOAT2(), scale);

	FontPos.x += eyeWidth;

	m_Font->DrawString(m_SpriteBatch.get(), &buf[0],
		FontPos, DirectX::Colors::Gray, 0.f, DirectX::XMFLOAT2(), scale);

	m_SpriteBatch->End();
}

void FrameRender::RenderDebugText(const std::string & debugText)
{
	if (!Settings::Instance().m_DebugFrameIndex) {
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
	if (enableFFR) {
		return m_ffr->GetOutputTexture();
	}
	else {
		return m_pStagingTexture;
	}

}

void FrameRender::CreateResourceTexture()
{
	std::vector<uint8_t> texture;
	if (!ReadBinaryResource(texture, IDR_RECENTER_TEXTURE)) {
		Log(L"Failed to load resource for IDR_RECENTER_TEXTURE.");
		return;
	}
	CoInitialize(NULL);

	HRESULT hr = DirectX::CreateWICTextureFromMemory(m_pD3DRender->GetDevice(), (uint8_t *)&texture[0], texture.size(),
		&m_recenterTexture, &m_recenterResourceView);
	if (!m_recenterTexture) {
		Log(L"Failed to create recenter texture. %d %s", hr, GetErrorStr(hr).c_str());
	}else if (!m_recenterResourceView) {
		Log(L"Failed to create recenter resource view. %d %s", hr, GetErrorStr(hr).c_str());
	}

	if (!ReadBinaryResource(texture, IDR_MESSAGE_BG_TEXTURE)) {
		Log(L"Failed to load resource for IDR_MESSAGE_BG_TEXTURE.");
		return;
	}

	hr = DirectX::CreateWICTextureFromMemory(m_pD3DRender->GetDevice(), (uint8_t *)&texture[0], texture.size(),
		&m_messageBGTexture, &m_messageBGResourceView);
	if (!m_messageBGTexture) {
		Log(L"Failed to create message_bg texture. %d %s", hr, GetErrorStr(hr).c_str());
	}
	else if (!m_messageBGResourceView) {
		Log(L"Failed to create message_bg resource view. %d %s", hr, GetErrorStr(hr).c_str());
	}
}

void FrameRender::GetEncodingResolution(uint32_t *width, uint32_t *height) {
	if (enableFFR) {
		m_ffr->GetOptimizedResolution(width, height);
	}
	else {
		*width = Settings::Instance().m_renderWidth;
		*height = Settings::Instance().m_renderHeight;
	}
	
}