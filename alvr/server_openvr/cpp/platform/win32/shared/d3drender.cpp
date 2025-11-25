//===================== Copyright (c) Valve Corporation. All Rights Reserved. ======================
#include "d3drender.h"
#include <d3d11_4.h>
#include <evntprov.h>

#pragma comment( lib, "dxgi.lib" )
#pragma comment( lib, "d3d11.lib" )
#pragma comment( lib, "rpcrt4.lib" )

#define Log( ... )

namespace
{
	inline bool operator==( const LUID &A, const LUID &B )
	{
		return A.HighPart == B.HighPart && A.LowPart == B.LowPart;
	}

	bool FindDXGIOutput( IDXGIFactory *pFactory, int32_t nWidth, int32_t nHeight, IDXGIAdapter **pOutAdapter, IDXGIOutput **pOutOutput, int32_t *pOutX, int32_t *pOutY )
	{
		IDXGIAdapter *pDXGIAdapter;
		for ( UINT nAdapterIndex = 0; pFactory->EnumAdapters( nAdapterIndex, &pDXGIAdapter ) != DXGI_ERROR_NOT_FOUND; nAdapterIndex++ )
		{
			IDXGIOutput *pDXGIOutput;
			for ( UINT nOutputIndex = 0; pDXGIAdapter->EnumOutputs( nOutputIndex, &pDXGIOutput ) != DXGI_ERROR_NOT_FOUND; nOutputIndex++ )
			{
				DXGI_OUTPUT_DESC desc;
				pDXGIOutput->GetDesc( &desc );

				//if ( desc.DesktopCoordinates.right - desc.DesktopCoordinates.left == nWidth &&
				//	desc.DesktopCoordinates.bottom - desc.DesktopCoordinates.top == nHeight )
				//{
					*pOutAdapter = pDXGIAdapter;
					*pOutOutput = pDXGIOutput;
					*pOutX = desc.DesktopCoordinates.left;
					*pOutY = desc.DesktopCoordinates.top;
					return true;
				//}
				pDXGIOutput->Release();
			}
			pDXGIAdapter->Release();
		}
		return false;
	}

	bool CreateDevice( IDXGIAdapter *pDXGIAdapter, ID3D11Device **pD3D11Device, ID3D11DeviceContext **pD3D11Context )
	{
		UINT creationFlags = 0;
#if _DEBUG
		creationFlags |= D3D11_CREATE_DEVICE_DEBUG;
#endif
		D3D_FEATURE_LEVEL eFeatureLevel;

		HRESULT hRes = D3D11CreateDevice( pDXGIAdapter, D3D_DRIVER_TYPE_UNKNOWN, NULL, creationFlags, NULL, 0, D3D11_SDK_VERSION, pD3D11Device, &eFeatureLevel, pD3D11Context );
#if _DEBUG
		// CreateDevice fails on Win10 in debug if the Win10 SDK isn't installed.
		if ( pD3D11Device == NULL )
		{
			hRes = D3D11CreateDevice( pDXGIAdapter, D3D_DRIVER_TYPE_UNKNOWN, NULL, 0, NULL, 0, D3D11_SDK_VERSION, pD3D11Device, &eFeatureLevel, pD3D11Context );
		}
#endif
		if ( FAILED( hRes ) )
		{
			Log( "Failed to create D3D11 device! (err=%u)", hRes );
			return false;
		}

		if ( eFeatureLevel < D3D_FEATURE_LEVEL_11_0 )
		{
			Log( "DX11 level hardware required!" );
			return false;
		}

		ID3D11Multithread *D3D11Multithread = NULL;
		HRESULT hr = (*pD3D11Context)->QueryInterface(__uuidof(ID3D11Multithread), (void **)&D3D11Multithread);
		if (SUCCEEDED(hr)) {
			Log("Successfully get ID3D11Multithread interface. We set SetMultithreadProtected(TRUE)");
			D3D11Multithread->SetMultithreadProtected(TRUE);
			D3D11Multithread->Release();
		}
		else {
			Log("Failed to get ID3D11Multithread interface. Ignore.");
		}

		return true;
	}

	class CEventHelper
	{
	public:
		CEventHelper()
		{
			UuidFromStringA( ( RPC_CSTR ) "8c8f13b1-60eb-4b6a-a433-de86104115ac", &guid );
			EventRegister( &guid, nullptr, nullptr, &handle );
		}

		REGHANDLE handle;
		GUID guid;
	};

	CEventHelper s_eventHelper;
}

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
void EventWriteString( const wchar_t* pwchEvent )
{
	::EventWriteString( s_eventHelper.handle, 0, 0, pwchEvent );
}

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
CD3DRender::CD3DRender()
	: m_pDXGIFactory( NULL )
	, m_pDXGIOutput( NULL )
	, m_pDXGISwapChain( NULL )
	, m_pD3D11Device( NULL )
	, m_pD3D11Context( NULL )
	, m_nDisplayWidth( 0 )
	, m_nDisplayHeight( 0 )
	, m_nDisplayX( 0 )
	, m_nDisplayY( 0 )
{
	// Initialize DXGI
	{
		// Need to use DXGI 1.1 for shared texture support.
		IDXGIFactory1 *pDXGIFactory1;
		if ( FAILED( CreateDXGIFactory1( __uuidof( IDXGIFactory1 ), ( void ** )&pDXGIFactory1 ) ) )
		{
			Log( "Failed to create DXGIFactory1!" );
			return;
		}
		else if ( FAILED( pDXGIFactory1->QueryInterface( __uuidof( IDXGIFactory ), ( void ** )&m_pDXGIFactory ) ) )
		{
			pDXGIFactory1->Release();
			Log( "Failed to get DXGIFactory interface!" );
			return;
		}
		pDXGIFactory1->Release();
	}
}

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
CD3DRender::~CD3DRender()
{
	SAFE_RELEASE( m_pDXGIFactory );
}

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
void CD3DRender::GetDisplayPos( int32_t *pDisplayX, int32_t *pDisplayY )
{
	*pDisplayX = m_nDisplayX;
	*pDisplayY = m_nDisplayY;
}

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
void CD3DRender::GetDisplaySize( uint32_t *pDisplayWidth, uint32_t *pDisplayHeight )
{
	*pDisplayWidth = m_nDisplayWidth;
	*pDisplayHeight = m_nDisplayHeight;
}

//--------------------------------------------------------------------------------------------------
// Purpose: Return the DXGI index and name of the adapter currently in use.
//--------------------------------------------------------------------------------------------------
bool CD3DRender::GetAdapterInfo( int32_t *pAdapterIndex, std::wstring &adapterName )
{
	if ( m_pD3D11Device == NULL )
		return false;

	bool bSuccess = false;

	IDXGIDevice *pDXGIDevice;
	if ( SUCCEEDED( m_pD3D11Device->QueryInterface( __uuidof( IDXGIDevice ), ( void ** )&pDXGIDevice ) ) )
	{
		IDXGIAdapter *pDXGIAdapter;
		if ( SUCCEEDED( pDXGIDevice->GetParent( __uuidof( IDXGIAdapter ), ( void ** )&pDXGIAdapter ) ) )
		{
			DXGI_ADAPTER_DESC adapterDesc;
			pDXGIAdapter->GetDesc( &adapterDesc );

			IDXGIFactory *pDXGIFactory;
			if ( SUCCEEDED( pDXGIAdapter->GetParent( __uuidof( IDXGIFactory ), ( void ** )&pDXGIFactory ) ) )
			{
				IDXGIAdapter *pEnumeratedAdapter;
				for ( UINT nAdapterIndex = 0; pDXGIFactory->EnumAdapters( nAdapterIndex, &pEnumeratedAdapter ) != DXGI_ERROR_NOT_FOUND; nAdapterIndex++ )
				{
					DXGI_ADAPTER_DESC enumeratedDesc;
					pEnumeratedAdapter->GetDesc( &enumeratedDesc );
					pEnumeratedAdapter->Release();

					if ( enumeratedDesc.AdapterLuid == adapterDesc.AdapterLuid )
					{
						if ( pAdapterIndex )
							*pAdapterIndex = nAdapterIndex;

						adapterName = adapterDesc.Description;

						bSuccess = true;
						break;
					}
				}
				pDXGIFactory->Release();
			}
			pDXGIAdapter->Release();
		}
		pDXGIDevice->Release();
	}

	return bSuccess;
}

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
ID3D11Texture2D *CD3DRender::GetSharedTexture( HANDLE hSharedTexture )
{
	if ( !hSharedTexture )
		return NULL;

	for ( SharedTextures_t::iterator it = m_SharedTextureCache.begin();
		it != m_SharedTextureCache.end(); ++it )
	{
		if ( it->m_hSharedTexture == hSharedTexture )
		{
			return it->m_pTexture;
		}
	}

	ID3D11Texture2D *pTexture;
	if ( SUCCEEDED( m_pD3D11Device->OpenSharedResource(
		hSharedTexture, __uuidof( ID3D11Texture2D ), ( void ** )&pTexture ) ) )
	{
		SharedTextureEntry_t entry { hSharedTexture, pTexture };
		m_SharedTextureCache.push_back( entry );
		return pTexture;
	}

	return NULL;
}

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
bool CD3DRender::Initialize( uint32_t nAdapterIndex )
{
	Shutdown();

	if ( m_pDXGIFactory == NULL )
		return false;

	IDXGIAdapter *pDXGIAdapter;
	if ( FAILED( m_pDXGIFactory->EnumAdapters( nAdapterIndex, &pDXGIAdapter ) ) )
		return false;

	bool bSuccess = CreateDevice( pDXGIAdapter, &m_pD3D11Device, &m_pD3D11Context );

	pDXGIAdapter->Release();

	return bSuccess;
}

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
bool CD3DRender::Initialize( uint32_t nDisplayWidth, uint32_t nDisplayHeight )
{
	Shutdown();

	if ( m_pDXGIFactory == NULL )
		return false;

	m_nDisplayWidth = nDisplayWidth;
	m_nDisplayHeight = nDisplayHeight;

	IDXGIAdapter *pDXGIAdapter;
	if ( !FindDXGIOutput( m_pDXGIFactory, m_nDisplayWidth, m_nDisplayHeight, &pDXGIAdapter, &m_pDXGIOutput, &m_nDisplayX, &m_nDisplayY ) )
		return false;

	bool bSuccess = CreateDevice( pDXGIAdapter, &m_pD3D11Device, &m_pD3D11Context );

	pDXGIAdapter->Release();

	return bSuccess;
}

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
void CD3DRender::Shutdown()
{
	SetFullscreen( FALSE );
	SAFE_RELEASE( m_pD3D11Context );
	SAFE_RELEASE( m_pD3D11Device );
	SAFE_RELEASE( m_pDXGISwapChain );
	SAFE_RELEASE( m_pDXGIOutput );
	m_nDisplayWidth = 0;
	m_nDisplayHeight = 0;
}

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
bool CD3DRender::CreateSwapChain( HWND hWnd, const DXGI_RATIONAL &refreshRate )
{
	if ( !m_pD3D11Device )
		return false;
	if ( !m_pDXGIOutput )
		return false;
	if ( !m_pDXGIFactory )
		return false;

	// Determine which video mode to use

	DXGI_MODE_DESC modeDesc;
	ZeroMemory( &modeDesc, sizeof( modeDesc ) );

	modeDesc.Width = m_nDisplayWidth;
	modeDesc.Height = m_nDisplayHeight;
	modeDesc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
	modeDesc.ScanlineOrdering = DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED;
	modeDesc.Scaling = DXGI_MODE_SCALING_UNSPECIFIED;
	modeDesc.RefreshRate = refreshRate;

	DXGI_MODE_DESC modeOut = modeDesc;

	if ( FAILED( m_pDXGIOutput->FindClosestMatchingMode( &modeDesc, &modeOut, m_pD3D11Device ) ) )
	{
		Log( "Failed to find closest matching mode!" );
		return false;
	}

	// Create a fullscreen swap chain for the window

	DXGI_SWAP_CHAIN_DESC swapChainDesc;
	ZeroMemory( &swapChainDesc, sizeof( swapChainDesc ) );
	swapChainDesc.BufferCount = 2;
	swapChainDesc.BufferDesc = modeOut;
	swapChainDesc.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT;
	swapChainDesc.OutputWindow = hWnd;
	swapChainDesc.SampleDesc.Count = 1;
	swapChainDesc.SampleDesc.Quality = 0;
	swapChainDesc.Windowed = TRUE;
	swapChainDesc.SwapEffect = DXGI_SWAP_EFFECT_DISCARD;
	swapChainDesc.Flags = DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH;

	if ( FAILED( m_pDXGIFactory->CreateSwapChain( m_pD3D11Device, &swapChainDesc, &m_pDXGISwapChain ) ) )
	{
		Log( "Failed to create swap chain!" );
		return false;
	}

	m_pDXGIFactory->MakeWindowAssociation( swapChainDesc.OutputWindow, DXGI_MWA_NO_WINDOW_CHANGES | DXGI_MWA_NO_ALT_ENTER );

	//!! Probably don't need this as long as we Flush after Present.
	IDXGIDevice1 *pDXGIDevice1;
	if ( SUCCEEDED( m_pD3D11Device->QueryInterface( __uuidof( IDXGIDevice1 ), ( void ** )&pDXGIDevice1 ) ) )
	{
		pDXGIDevice1->SetGPUThreadPriority( 7 );
		pDXGIDevice1->SetMaximumFrameLatency( 1 );
		pDXGIDevice1->Release();
	}

	return true;
}

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
void CD3DRender::SetFullscreen( BOOL bFullscreen )
{
	if ( m_pDXGISwapChain && m_pDXGIOutput )
	{
		// Bail if no change is necessary.
		BOOL bState;
		if ( SUCCEEDED( m_pDXGISwapChain->GetFullscreenState( &bState, NULL ) ) )
		{
			if ( bState == bFullscreen )
				return;
		}

		// Bail if not ready to go fullscreen yet.
		if ( bFullscreen )
		{
			if ( m_pDXGISwapChain->Present( 0, DXGI_PRESENT_TEST ) != S_OK )
				return;
		}

		if ( m_pDXGISwapChain->SetFullscreenState( bFullscreen, bFullscreen ? m_pDXGIOutput : NULL ) == S_OK )
		{
			UpdateBuffers(); // WM_SIZE doesn't get sent since window was created at proper size
		}
	}
}

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
void CD3DRender::UpdateBuffers()
{
	if ( m_pDXGISwapChain )
	{
		DXGI_SWAP_CHAIN_DESC swapChainDesc;
		if ( SUCCEEDED( m_pDXGISwapChain->GetDesc( &swapChainDesc ) ) )
		{
			m_pD3D11Context->ClearState(); //OMSetRenderTargets(0, NULL, NULL);
			m_pDXGISwapChain->ResizeBuffers( 0, ( UINT )m_nDisplayWidth, ( UINT )m_nDisplayHeight, swapChainDesc.BufferDesc.Format, swapChainDesc.Flags );
		}
	}
}

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
bool CD3DRender::GetAdapterLuid( int32_t nAdapterIndex, uint64_t *pAdapterLuid )
{
	bool bSuccess = false;

	if ( m_pDXGIFactory != NULL )
	{
		IDXGIAdapter *pDXGIAdapter;
		if ( SUCCEEDED( m_pDXGIFactory->EnumAdapters( nAdapterIndex, &pDXGIAdapter ) ) )
		{
			DXGI_ADAPTER_DESC adapterDesc;
			pDXGIAdapter->GetDesc( &adapterDesc );
			pDXGIAdapter->Release();

			*pAdapterLuid = *( uint64_t * )&adapterDesc.AdapterLuid;
			bSuccess = true;
		}
	}

	return bSuccess;
}

#define MIN(a, b) ((a) < (b) ? (a) : (b))

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
void CD3DRender::CopyTextureData( BYTE *pDst, uint32_t nDstRowPitch,
	const BYTE *pSrc, uint32_t nSrcRowPitch,
	uint32_t nWidth, uint32_t nHeight, uint32_t nPitch )
{
	if ( nDstRowPitch == nSrcRowPitch )
	{
		memcpy( pDst, pSrc, nSrcRowPitch * nHeight );
	}
	else
	{
		uint32_t nMinRowPitch = MIN(nDstRowPitch, nSrcRowPitch);
		for ( uint32_t i = 0; i < nHeight; i++ )
		{
			memcpy( pDst, pSrc, nMinRowPitch );
			pDst += nDstRowPitch;
			pSrc += nSrcRowPitch;
		}
	}
}

