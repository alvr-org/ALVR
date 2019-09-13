//===================== Copyright (c) Valve Corporation. All Rights Reserved. ======================
//
// Helper class for working with D3D.
//
//==================================================================================================
#pragma once

#include <d3d11.h>
#include <stdint.h>
#include <vector>
#include <string>

void EventWriteString( const wchar_t* pwchEvent ); // gpuview event

#define SAFE_RELEASE( x ) if ( x ) { ( x )->Release(); ( x ) = NULL; }

class CD3DRender
{
public:
	CD3DRender();
	~CD3DRender();

	bool Initialize( uint32_t nDisplayWidth, uint32_t nDisplayHeight );
	bool Initialize( uint32_t nAdapterIndex );
	void Shutdown();

	void GetDisplayPos( int32_t *pDisplayX, int32_t *pDisplayY );
	void GetDisplaySize( uint32_t *pDisplayWidth, uint32_t *pDisplayHeight );
	bool GetAdapterInfo( int32_t *pAdapterIndex, std::wstring &adapterName );
	ID3D11Texture2D *GetSharedTexture( HANDLE hSharedTexture );

	bool CreateSwapChain( HWND hWnd, const DXGI_RATIONAL &refreshRate );
	void SetFullscreen( BOOL bFullscreen );
	void UpdateBuffers();

	ID3D11Device *GetDevice() { return m_pD3D11Device; }
	ID3D11DeviceContext *GetContext() { return m_pD3D11Context; }
	IDXGISwapChain *GetSwapChain() { return m_pDXGISwapChain; }

	bool GetAdapterLuid( int32_t nAdapterIndex, uint64_t *pAdapterLuid );

	static void CopyTextureData( BYTE *pDst, uint32_t nDstRowPitch,
		const BYTE *pSrc, uint32_t nSrcRowPitch,
		uint32_t nWidth, uint32_t nHeight, uint32_t nPitch );

private:
	IDXGIFactory *m_pDXGIFactory;
	IDXGIOutput *m_pDXGIOutput;
	IDXGISwapChain *m_pDXGISwapChain;
	ID3D11Device *m_pD3D11Device;
	ID3D11DeviceContext *m_pD3D11Context;
	uint32_t m_nDisplayWidth, m_nDisplayHeight;
	int32_t m_nDisplayX, m_nDisplayY;

	struct SharedTextureEntry_t
	{
		HANDLE m_hSharedTexture;
		ID3D11Texture2D *m_pTexture;
	};
	typedef std::vector< SharedTextureEntry_t > SharedTextures_t;
	SharedTextures_t m_SharedTextureCache;
};

