#pragma once

#pragma comment(lib, "dxgi.lib")
#pragma comment(lib, "d3d11.lib")

#include <vector>
#include <exception>
#include <stdexcept>
#include <string>
#include <functional>

#include <d3d11.h>
#include <wrl.h>

#include "alvr_server/Logger.h"
#include "alvr_server/Utils.h"

#define OK_OR_THROW(dxcall, msg) { HRESULT hr = dxcall; if (FAILED(hr)) throw MakeException("%ls HR=%p %ls", msg, hr, GetErrorStr(hr).c_str()); }
#define QUERY(from, ppd3d) from->QueryInterface(__uuidof(*(ppd3d)), (void**)(ppd3d))

namespace d3d_render_utils {

	void GetAdapterInfo(ID3D11Device *d3dDevice, int32_t &adapterIndex, std::wstring &adapterName);

	ID3D11Device *CreateDevice(IDXGIAdapter *dxgiAdapter = nullptr);
	ID3D11Device *CreateDevice(uint32_t adapterIndex);

	ID3D11Texture2D *CreateTexture(ID3D11Device *device, uint32_t width, uint32_t height,
		DXGI_FORMAT format = DXGI_FORMAT_R8G8B8A8_UNORM, bool mipmaps = false,
		bool shared = false, UINT sampleCount = 1);

	ID3D11Buffer *_CreateBuffer(ID3D11Device *device, const void *bufferData, 
		size_t bufferSize, D3D11_USAGE usage);

	template<typename T>
	ID3D11Buffer *CreateBuffer(ID3D11Device *device, const T &bufferData, 
		D3D11_USAGE usage = D3D11_USAGE_IMMUTABLE)
	{
		return _CreateBuffer(device, &bufferData, sizeof(T), usage);
	}

	void UpdateBuffer(ID3D11DeviceContext *context, ID3D11Buffer *buffer, const void *bufferData);

	ID3D11VertexShader *CreateVertexShader(ID3D11Device *device, std::vector<uint8_t> &vertexShaderCSO);
	ID3D11PixelShader *CreatePixelShader(ID3D11Device *device, std::vector<uint8_t> &pixelShaderCSO);

	ID3D11Texture2D *GetTextureFromHandle(ID3D11Device *device, HANDLE handle);
	HANDLE GetHandleFromTexture(ID3D11Texture2D *texture);

	void KeyedMutexSync(ID3D11Device *device, HANDLE sharedTextureHandle, uint64_t timeout, std::function<void()> acquiredSyncCallback);
}