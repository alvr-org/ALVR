#include "RenderUtils.h"

#include <D3d11_4.h>

using namespace std::string_literals;
using Microsoft::WRL::ComPtr;

namespace d3d_render_utils {

	void GetAdapterInfo(ID3D11Device *d3dDevice, int32_t &adapterIndex, std::wstring &adapterName) {
		ComPtr<IDXGIDevice> dxgiDevice;
		OK_OR_THROW(QUERY(d3dDevice, &dxgiDevice), "Failed to query DXGI device.");

		ComPtr<IDXGIAdapter> adapter;
		OK_OR_THROW(dxgiDevice->GetParent(__uuidof(IDXGIAdapter), (void**)&adapter), "Failed to get DXGI adapter.");

		ComPtr<IDXGIFactory> factory;
		OK_OR_THROW(adapter->GetParent(__uuidof(IDXGIFactory), (void**)&factory), "Failed to get DXGI factory.");

		DXGI_ADAPTER_DESC adapterDesc;
		adapter->GetDesc(&adapterDesc);

		ComPtr<IDXGIAdapter> enumeratedAdapter;
		for (UINT idx = 0; factory->EnumAdapters(idx, &enumeratedAdapter) != DXGI_ERROR_NOT_FOUND; idx++) {
			DXGI_ADAPTER_DESC enumeratedDesc;
			enumeratedAdapter->GetDesc(&enumeratedDesc);

			if (enumeratedDesc.AdapterLuid.HighPart == adapterDesc.AdapterLuid.HighPart &&
				enumeratedDesc.AdapterLuid.LowPart == adapterDesc.AdapterLuid.LowPart)
			{
				adapterIndex = idx;
				adapterName = adapterDesc.Description;
				return;
			}
		}

		throw MakeException("No valid adapter found.");
	}

	ID3D11Device *CreateDevice(IDXGIAdapter *dxgiAdapter) {
		UINT creationFlags = 0;
#if _DEBUG
		creationFlags |= D3D11_CREATE_DEVICE_DEBUG;
#endif

		D3D_FEATURE_LEVEL featureLevel;

		ID3D11Device *device;
		ComPtr<ID3D11DeviceContext> context;
		OK_OR_THROW(D3D11CreateDevice(dxgiAdapter, dxgiAdapter != nullptr ? D3D_DRIVER_TYPE_UNKNOWN : D3D_DRIVER_TYPE_HARDWARE,
			nullptr, creationFlags, nullptr, 0, D3D11_SDK_VERSION, &device, &featureLevel, &context),
			"Failed to create D3D11 device!");

		if (featureLevel < D3D_FEATURE_LEVEL_11_0) {
			throw MakeException("DX11 level hardware required!");
		}

		//todo: check if needed:
		ComPtr<ID3D11Multithread> multithread;
		if (SUCCEEDED(QUERY(context, &multithread))) {
			multithread->SetMultithreadProtected(true);
		}
		else {
			Debug("Failed to get ID3D11Multithread interface. Ignore.\n");
		}

		return device;
	}

	ID3D11Device *CreateDevice(uint32_t adapterIndex) {
		ComPtr<IDXGIFactory1> factory;
		OK_OR_THROW(CreateDXGIFactory1(__uuidof(IDXGIFactory1), (void **)&factory), "Failed to create DXGIFactory1!");

		ComPtr<IDXGIAdapter> adapter;
		OK_OR_THROW(factory->EnumAdapters(adapterIndex, &adapter), "Failed to create DXGIAdapter!");

		return CreateDevice(adapter.Get());
	}

	ID3D11Texture2D *CreateTexture(ID3D11Device *device, uint32_t width, uint32_t height, 
		DXGI_FORMAT format, bool mipmaps, bool shared, UINT sampleCount)
	{
		D3D11_TEXTURE2D_DESC desc = { 0 };
		desc.Width = width;
		desc.Height = height;
		desc.Format = format;
		desc.SampleDesc.Count = sampleCount;
		desc.MipLevels = mipmaps ? 0 : 1;
		desc.MiscFlags = (shared ? D3D11_RESOURCE_MISC_SHARED : 0) | (mipmaps ? D3D11_RESOURCE_MISC_GENERATE_MIPS : 0);
		// D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX | D3D11_RESOURCE_MISC_SHARED_NTHANDLE

		desc.ArraySize = 1;
		desc.SampleDesc.Quality = 0;
		desc.Usage = D3D11_USAGE_DEFAULT;
		desc.BindFlags = D3D11_BIND_RENDER_TARGET | D3D11_BIND_SHADER_RESOURCE;
		desc.CPUAccessFlags = 0;

		ID3D11Texture2D *texture;
		OK_OR_THROW(device->CreateTexture2D(&desc, nullptr, &texture), "Failed to create texture.");
		return texture;
	}

	ID3D11Buffer *_CreateBuffer(ID3D11Device *device, const void *bufferData, size_t bufferSize, D3D11_USAGE usage) {
		D3D11_BUFFER_DESC bufferDesc = { 0 };
		bufferDesc.Usage = usage;
		bufferDesc.ByteWidth = (UINT)bufferSize;
		bufferDesc.BindFlags = D3D11_BIND_CONSTANT_BUFFER;
		bufferDesc.StructureByteStride = 0;

		D3D11_SUBRESOURCE_DATA dataDesc = { 0 };
		dataDesc.pSysMem = bufferData;

		ID3D11Buffer *buffer;
		OK_OR_THROW(device->CreateBuffer(&bufferDesc, bufferData != nullptr ? &dataDesc : nullptr, &buffer), "Failed to create D3D11 buffer.");
		return buffer;
	}

	void UpdateBuffer(ID3D11DeviceContext *context, ID3D11Buffer *buffer, const void *bufferData) {
		context->UpdateSubresource(buffer, 0, nullptr, bufferData, 0, 0);
	}

	ID3D11VertexShader *CreateVertexShader(ID3D11Device *device, std::vector<uint8_t> &vertexShaderCSO) {
		ID3D11VertexShader *vertexShader;
		OK_OR_THROW(device->CreateVertexShader(&vertexShaderCSO[0], vertexShaderCSO.size(), nullptr, &vertexShader),
			"Failed to create vertex shader.");
		return vertexShader;
	}

	ID3D11PixelShader *CreatePixelShader(ID3D11Device *device, std::vector<uint8_t> &pixelShaderCSO) {
		ID3D11PixelShader *pixelShader;
		OK_OR_THROW(device->CreatePixelShader(&pixelShaderCSO[0], pixelShaderCSO.size(), nullptr, &pixelShader),
			"Failed to create pixel shader.");
		return pixelShader;
	}

	ID3D11Texture2D *GetTextureFromHandle(ID3D11Device *device, HANDLE handle) {
		ID3D11Texture2D *texture;
		OK_OR_THROW(device->OpenSharedResource(handle, __uuidof(ID3D11Texture2D), (void **)&texture),
			"[VDispDvr] SyncTexture is NULL!");
		return texture;
	}

	HANDLE GetHandleFromTexture(ID3D11Texture2D *texture) {
		auto exceptMsg = "Failed to get handle from shared texture";

		ComPtr<IDXGIResource> resource;
		OK_OR_THROW(QUERY(texture, &resource), exceptMsg);

		HANDLE handle;
		OK_OR_THROW(resource->GetSharedHandle(&handle), exceptMsg);

		return handle;
	}

	void KeyedMutexSync(ID3D11Device *device, HANDLE handle, uint64_t timeout, std::function<void()> callback) {
		ComPtr<ID3D11Texture2D> syncTexture = GetTextureFromHandle(device, handle);

		ComPtr<IDXGIKeyedMutex> keyedMutex;
		OK_OR_THROW(QUERY(syncTexture, &keyedMutex), "Failed to query mutex");

		// TODO: Reasonable timeout and timeout handling
		OK_OR_THROW(keyedMutex->AcquireSync(0, (DWORD)timeout), "[VDispDvr] ACQUIRESYNC FAILED!!!");

		callback();

		keyedMutex->ReleaseSync(0);
	}
}
