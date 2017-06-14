//===================== Copyright (c) Valve Corporation. All Rights Reserved. ======================
//
// Standalone process which represents the remote display in our VirtualDisplay example.
// This cannot simply be a separate thread in driver_virtual_display.dll as calling Present blocks
// any additional D3D work from happening on the gpu until vsync, and we need to be able to queue
// up the next frame (i.e. read pixel wait to determine when app rendering is complete).
//
//==================================================================================================

#include "sharedstate.h"
#include "threadtools.h"
#include "systemtime.h"
#include "d3drender.h"

namespace
{
	const DWORD k_WindowStyle = WS_CLIPSIBLINGS | WS_CLIPCHILDREN | WS_POPUP;
	const DWORD k_WindowStyleEx = 0; //WS_EX_TOPMOST;
	const WCHAR *const k_ClassName = L"Remote Display";

	static HWND g_hWnd = NULL;
	static CD3DRender *g_pD3DRender = NULL;

	LRESULT CALLBACK WndProc( HWND hWnd, UINT message, WPARAM wParam, LPARAM lParam )
	{
		switch ( message )
		{
		case WM_SIZE:
			if ( g_pD3DRender )
			{
				g_pD3DRender->UpdateBuffers();
			}
			return 1;

		case WM_DESTROY:
			PostQuitMessage( 0 );
			return 0;

		default:
			return DefWindowProcW( hWnd, message, wParam, lParam );
		}
	}

	struct WindowRect_t
	{
		uint32_t x, y, w, h;
	};

	HWND InitWindow( const WindowRect_t &wr )
	{
		HINSTANCE hInstance = GetModuleHandle( NULL );

		WNDCLASSEXW wc;
		ZeroMemory( &wc, sizeof( wc ) );

		wc.cbSize = sizeof( WNDCLASSEXW );
		wc.style = CS_HREDRAW | CS_VREDRAW | CS_OWNDC;
		wc.lpfnWndProc = WndProc;
		wc.cbClsExtra = 0;
		wc.cbWndExtra = 0;
		wc.hInstance = hInstance;
		wc.hIcon = NULL;
		wc.hCursor = NULL;
		wc.hbrBackground = NULL;
		wc.lpszMenuName = NULL;
		wc.lpszClassName = k_ClassName;
		wc.hIconSm = wc.hIcon;

		::RegisterClassExW( &wc );

		return ::CreateWindowExW( k_WindowStyleEx, wc.lpszClassName, wc.lpszClassName, k_WindowStyle, wr.x, wr.y, wr.w, wr.h, NULL, 0, hInstance, NULL );
	}

	static void ToggleDesktopComposition( BOOL bEnableComposition )
	{
		static HMODULE s_hDWMApiDLL = NULL;
		typedef HRESULT( WINAPI *DwmEnableComposition_t )( UINT );
		static DwmEnableComposition_t s_aDwmEnableComposition;

		if ( !s_hDWMApiDLL )
		{
			s_hDWMApiDLL = LoadLibrary( "DWMAPI.DLL" );

			if ( s_hDWMApiDLL )
			{
				s_aDwmEnableComposition = ( DwmEnableComposition_t )GetProcAddress( s_hDWMApiDLL, "DwmEnableComposition" );
			}
		}

		if ( s_aDwmEnableComposition )
		{
			s_aDwmEnableComposition( bEnableComposition );
		}
	}

	bool ParseArgs( int argc, char **argv, WindowRect_t *pWindowRect, DXGI_RATIONAL *pRefreshRate )
	{
		if ( argc < 6 )
			return false;

		pWindowRect->x = atoi( argv[ 1 ] );
		pWindowRect->y = atoi( argv[ 2 ] );
		pWindowRect->w = atoi( argv[ 3 ] );
		pWindowRect->h = atoi( argv[ 4 ] );

		pRefreshRate->Numerator = atoi( argv[ 5 ] );
		pRefreshRate->Denominator = atoi( argv[ 6 ] );

		return pWindowRect->w && pWindowRect->h && pRefreshRate->Numerator && pRefreshRate->Denominator;
	}

	class CPresentThread : public CThread
	{
	public:
		CPresentThread( IDXGISwapChain *pDXGISwapChain, float flFrameIntervalInSeconds )
			: m_pDXGISwapChain( pDXGISwapChain )
			, m_flFrameIntervalInSeconds( flFrameIntervalInSeconds )
			, m_pStagingTexture( NULL )
			, m_pNewFrame( NULL )
			, m_bExiting( false )
		{
			m_pDXGISwapChain->AddRef();

			// Init backbuffer and queue up to scan out to prime vsync timing.
			ID3D11Texture2D *pBuffer;
			if ( SUCCEEDED( m_pDXGISwapChain->GetBuffer( 0, __uuidof( ID3D11Texture2D ), ( void ** )&pBuffer ) ) )
			{
				ID3D11Device *pDevice;
				pBuffer->GetDevice( &pDevice );

				ID3D11RenderTargetView *pRTV;
				if ( SUCCEEDED( pDevice->CreateRenderTargetView( pBuffer, NULL, &pRTV ) ) )
				{
					ID3D11DeviceContext *pContext;
					pDevice->GetImmediateContext( &pContext );

					float flInitColor[] = { 0, 0, 1, 1 };
					pContext->ClearRenderTargetView( pRTV, flInitColor );
					pContext->Release();
					pRTV->Release();
				}

				pDevice->Release();
				pBuffer->Release();
			}
		}

		~CPresentThread()
		{
			SAFE_RELEASE( m_pStagingTexture );
			SAFE_RELEASE( m_pDXGISwapChain );
		}

		bool Init() override
		{
			if ( !m_sharedState.IsValid() )
				return false;

			m_pNewFrame = new IPCEvent( "RemoteDisplay_NewFrame", false, false );

			CSharedState::Ptr data( &m_sharedState );
			SystemTime::Init( data->m_nSystemBaseTimeTicks );
			data->m_bShutdown = false;

			return m_pNewFrame != NULL;
		}

		void Run() override
		{
			SetThreadPriority( GetCurrentThread(), THREAD_PRIORITY_MOST_URGENT );

			// Initial Present to prime our timing reference.
			// This is performed here to avoid blocking main thread initialization.
			Present();

			while ( !m_bExiting )
			{
				EventWriteString( L"RemoteDisplay: Waiting for new frame..." );

				m_pNewFrame->Wait();
				if ( m_bExiting )
					break;

				double flVsyncTimeInSeconds;
				{
					CSharedState::Ptr data( &m_sharedState );

					if ( data->m_bShutdown )
					{
						SendMessage( g_hWnd, WM_CLOSE, 0, 0 );
						continue;
					}

					if ( data->m_nTextureWidth && data->m_nTextureHeight )
					{
						// Load data into staging texture.
						D3D11_TEXTURE2D_DESC stagingTextureDesc;
						if ( m_pStagingTexture != NULL )
						{
							m_pStagingTexture->GetDesc( &stagingTextureDesc );
							if ( stagingTextureDesc.Width != data->m_nTextureWidth ||
								stagingTextureDesc.Height != data->m_nTextureHeight )
							{
								m_pStagingTexture->Release();
								m_pStagingTexture = NULL;
							}
						}
						if ( m_pStagingTexture == NULL )
						{
							ZeroMemory( &stagingTextureDesc, sizeof( stagingTextureDesc ) );
							stagingTextureDesc.Width = data->m_nTextureWidth;
							stagingTextureDesc.Height = data->m_nTextureHeight;
							stagingTextureDesc.Format = ( DXGI_FORMAT )data->m_nTextureFormat;
							stagingTextureDesc.MipLevels = 1;
							stagingTextureDesc.ArraySize = 1;
							stagingTextureDesc.SampleDesc.Count = 1;
							stagingTextureDesc.Usage = D3D11_USAGE_STAGING;
							stagingTextureDesc.CPUAccessFlags = D3D11_CPU_ACCESS_WRITE;

							g_pD3DRender->GetDevice()->CreateTexture2D( &stagingTextureDesc, NULL, &m_pStagingTexture );
						}

						EventWriteString( L"RemoteDisplay: MapBegin" );

						D3D11_MAPPED_SUBRESOURCE mapped = { 0 };
						if ( SUCCEEDED( g_pD3DRender->GetContext()->Map( m_pStagingTexture, 0, D3D11_MAP_WRITE, 0, &mapped ) ) )
						{
							EventWriteString( L"RemoteDisplay: MapEnd" );

							CD3DRender::CopyTextureData( ( BYTE * )mapped.pData, mapped.RowPitch,
								data->m_nTextureData, data->m_nTextureWidth * SharedState_t::TEXTURE_PITCH,
								data->m_nTextureWidth, data->m_nTextureHeight, SharedState_t::TEXTURE_PITCH );

							EventWriteString( L"RemoteDisplay: MemCpyDone" );

							g_pD3DRender->GetContext()->Unmap( m_pStagingTexture, 0 );
						}

						// Copy staging to backbuffer.
						ID3D11Texture2D *pBuffer;
						if ( SUCCEEDED( m_pDXGISwapChain->GetBuffer( 0, __uuidof( ID3D11Texture2D ), ( void ** )&pBuffer ) ) )
						{
							EventWriteString( L"RemoteDisplay: Queue copy to back-buffer from staging" );
							uint32_t nDisplayWidth, nDisplayHeight;
							g_pD3DRender->GetDisplaySize( &nDisplayWidth, &nDisplayHeight );
							if ( stagingTextureDesc.Width == nDisplayWidth && stagingTextureDesc.Height == nDisplayHeight )
							{
								g_pD3DRender->GetContext()->CopyResource( pBuffer, m_pStagingTexture );
							}
							else
							{
								D3D11_BOX box = { 0, 0, 0, stagingTextureDesc.Width, stagingTextureDesc.Height, 1 };
								g_pD3DRender->GetContext()->CopySubresourceRegion( pBuffer, 0, 0, 0, 0, m_pStagingTexture, 0, &box );
							}
							pBuffer->Release();
						}
					}

					flVsyncTimeInSeconds = data->m_flVsyncTimeInSeconds;
				}

				Present( flVsyncTimeInSeconds );
			}

			return;
		}

		void Present( double flVsyncTimeInSeconds = 0.0 )
		{
			while ( SystemTime::GetInSeconds() < flVsyncTimeInSeconds )
			{
				EventWriteString( L"RemoteDisplay: Waiting till right side of vsync.." );
				Sleep( 1 );
			}

			EventWriteString( L"RemoteDisplay: Presenting.." );
			const UINT nSyncInterval = 1;
			m_pDXGISwapChain->Present( nSyncInterval, 0 );
			EventWriteString( L"RemoteDisplay: Presented" );

			// Wait for frame to start scanning out.
			float flSecondsToNextVsync = float( flVsyncTimeInSeconds - SystemTime::GetInSeconds() ) + m_flFrameIntervalInSeconds;
			uint32_t nSleepMs = ( uint32_t )max( 0.0f, flSecondsToNextVsync * 1000.0f );

			wchar_t buffer[ 255 ];
			swprintf( buffer, ARRAYSIZE( buffer ), L"RemoteDisplay: Sleep=%d", nSleepMs );
			EventWriteString( buffer );

			Sleep( nSleepMs );

			EventWriteString( L"RemoteDisplay: SleepEnd" );

			UINT nLastPresentCount;
			m_pDXGISwapChain->GetLastPresentCount( &nLastPresentCount );

			DXGI_FRAME_STATISTICS stats;
			m_pDXGISwapChain->GetFrameStatistics( &stats );

			while ( stats.PresentCount != nLastPresentCount )
			{
				EventWriteString( L"RemoteDisplay: Waiting for next frame interval.." );
				Sleep( 1 );
				m_pDXGISwapChain->GetFrameStatistics( &stats );
			}

			EventWriteString( L"RemoteDisplay: SCANNING OUT!" );

			CSharedState::Ptr data( &m_sharedState );
			double flLastVsyncTimeInSeconds = data->m_flLastVsyncTimeInSeconds;
			data->m_flLastVsyncTimeInSeconds = SystemTime::GetInSeconds( stats.SyncQPCTime.QuadPart );
			data->m_nVsyncCounter = stats.SyncRefreshCount;

			swprintf( buffer, ARRAYSIZE( buffer ), L"VSYNC: %f %f %d",
				float( data->m_flLastVsyncTimeInSeconds - flLastVsyncTimeInSeconds ) * 1000.0f,
				float( data->m_flLastVsyncTimeInSeconds ) * 1000.0f,
				data->m_nVsyncCounter );
			EventWriteString( buffer );
		}

		void Stop()
		{
			m_bExiting = true;
			m_pNewFrame->SetEvent();
			Join();
		}

	private:
		IDXGISwapChain *m_pDXGISwapChain;
		float m_flFrameIntervalInSeconds;
		ID3D11Texture2D *m_pStagingTexture;
		CSharedState m_sharedState;
		IPCEvent *m_pNewFrame;
		bool m_bExiting;
	};
}

//--------------------------------------------------------------------------------------------------
// Entry point and Windows message pump.
//--------------------------------------------------------------------------------------------------
int WINAPI WinMain( HINSTANCE hInstance, HINSTANCE hPrevInstance, LPSTR lpCmdLine, int nShowCmd )
{
	extern int __argc;
	extern char **__argv;

	WindowRect_t windowRect;
	DXGI_RATIONAL refreshRate;

	if ( !ParseArgs( __argc, __argv, &windowRect, &refreshRate ) )
	{
		return 0;
	}

	g_hWnd = InitWindow( windowRect );
	if ( g_hWnd == NULL )
	{
		return 0;
	}

	g_pD3DRender = new CD3DRender();
	if ( !g_pD3DRender->Initialize( windowRect.w, windowRect.h ) ||
		!g_pD3DRender->CreateSwapChain( g_hWnd, refreshRate ) )
	{
		g_pD3DRender->Shutdown();
		delete g_pD3DRender;
		return 0;
	}

	ToggleDesktopComposition( FALSE );

	::ShowWindow( g_hWnd, SW_SHOWDEFAULT );

	g_pD3DRender->SetFullscreen( TRUE );

	CPresentThread *pPresentThread = new CPresentThread( g_pD3DRender->GetSwapChain(),
		float( refreshRate.Denominator ) / refreshRate.Numerator );
	pPresentThread->Start();

	MSG msg;
	ZeroMemory( &msg, sizeof( msg ) );

	while ( 1 )
	{
		if ( PeekMessage( &msg, NULL, 0, 0, PM_REMOVE ) )
		{
			TranslateMessage( &msg );
			DispatchMessage( &msg );
		}

		if ( msg.message == WM_QUIT )
		{
			break;
		}

		Sleep( 100 );
	}

	pPresentThread->Stop();
	delete pPresentThread;
	
	g_pD3DRender->Shutdown();
	delete g_pD3DRender;

	return 0;
}

