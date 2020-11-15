#pragma once

#include <d3d11.h>
#include <iostream>
#include <unordered_map>
#include <memory>
#include <wrl.h>
#include <D3dx9core.h>

class RGBToNV12ConverterD3D11 {
public:
	RGBToNV12ConverterD3D11(ID3D11Device *pDevice, ID3D11DeviceContext *pContext, int nWidth, int nHeight)
		: pD3D11Device(pDevice), pD3D11Context(pContext)
	{
		pD3D11Device->AddRef();
		pD3D11Context->AddRef();

		pTexBgra = NULL;
		D3D11_TEXTURE2D_DESC desc;
		ZeroMemory(&desc, sizeof(D3D11_TEXTURE2D_DESC));
		desc.Width = nWidth;
		desc.Height = nHeight;
		desc.MipLevels = 1;
		desc.ArraySize = 1;
		desc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
		desc.SampleDesc.Count = 1;
		desc.Usage = D3D11_USAGE_DEFAULT;
		desc.BindFlags = D3D11_BIND_RENDER_TARGET;
		desc.CPUAccessFlags = 0;
		pDevice->CreateTexture2D(&desc, NULL, &pTexBgra);

		pDevice->QueryInterface(__uuidof(ID3D11VideoDevice), (void **)&pVideoDevice);
		pContext->QueryInterface(__uuidof(ID3D11VideoContext), (void **)&pVideoContext);

		D3D11_VIDEO_PROCESSOR_CONTENT_DESC contentDesc =
		{
			D3D11_VIDEO_FRAME_FORMAT_PROGRESSIVE,
		{ 1, 1 }, desc.Width, desc.Height,
		{ 1, 1 }, desc.Width, desc.Height,
		D3D11_VIDEO_USAGE_PLAYBACK_NORMAL
		};
		pVideoDevice->CreateVideoProcessorEnumerator(&contentDesc, &pVideoProcessorEnumerator);

		pVideoDevice->CreateVideoProcessor(pVideoProcessorEnumerator, 0, &pVideoProcessor);
		D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC inputViewDesc = { 0, D3D11_VPIV_DIMENSION_TEXTURE2D,{ 0, 0 } };
		pVideoDevice->CreateVideoProcessorInputView(pTexBgra, pVideoProcessorEnumerator, &inputViewDesc, &pInputView);
	}

	~RGBToNV12ConverterD3D11()
	{
		for (auto& it : outputViewMap)
		{
			ID3D11VideoProcessorOutputView* pOutputView = it.second;
			pOutputView->Release();
		}

		pInputView->Release();
		pVideoProcessorEnumerator->Release();
		pVideoProcessor->Release();
		pVideoContext->Release();
		pVideoDevice->Release();
		pTexBgra->Release();
		pD3D11Context->Release();
		pD3D11Device->Release();
	}
	void ConvertRGBToNV12(ID3D11Texture2D*pRGBSrcTexture, ID3D11Texture2D* pDestTexture)
	{
		pD3D11Context->CopyResource(pTexBgra, pRGBSrcTexture);
		ID3D11VideoProcessorOutputView* pOutputView = nullptr;
		auto it = outputViewMap.find(pDestTexture);
		if (it == outputViewMap.end())
		{
			D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC outputViewDesc = { D3D11_VPOV_DIMENSION_TEXTURE2D };
			pVideoDevice->CreateVideoProcessorOutputView(pDestTexture, pVideoProcessorEnumerator, &outputViewDesc, &pOutputView);
			outputViewMap.insert({ pDestTexture, pOutputView });
		}
		else
		{
			pOutputView = it->second;
		}

		D3D11_VIDEO_PROCESSOR_STREAM stream = { TRUE, 0, 0, 0, 0, NULL, pInputView, NULL };
		HRESULT hr = pVideoContext->VideoProcessorBlt(pVideoProcessor, pOutputView, 0, 1, &stream);
		if (FAILED(hr)) {
			Error("Failed on VideoProcessorBlt. hr=%p %s", hr, GetDxErrorStr(hr).c_str());
		}
		return;
	}

private:
	ID3D11Device * pD3D11Device = NULL;
	ID3D11DeviceContext *pD3D11Context = NULL;
	ID3D11VideoDevice *pVideoDevice = NULL;
	ID3D11VideoContext *pVideoContext = NULL;
	ID3D11VideoProcessor *pVideoProcessor = NULL;
	ID3D11VideoProcessorInputView *pInputView = NULL;
	ID3D11VideoProcessorOutputView *pOutputView = NULL;
	ID3D11Texture2D *pTexBgra = NULL;
	ID3D11VideoProcessorEnumerator *pVideoProcessorEnumerator = nullptr;
	std::unordered_map<ID3D11Texture2D*, ID3D11VideoProcessorOutputView*> outputViewMap;
};