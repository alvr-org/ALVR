//===================== Copyright (c) Valve Corporation. All Rights Reserved. ======================
//
// Example OpenVR driver for demonstrating IVRVirtualDisplay interface.
//
//==================================================================================================
#include "openvr_driver.h"
#include "sharedstate.h"
#include "threadtools.h"
#include "systemtime.h"
#include "d3drender.h"

namespace
{
	//-----------------------------------------------------------------------------
	// Settings
	//-----------------------------------------------------------------------------
	static const char * const k_pch_VirtualDisplay_Section = "driver_virtual_display";
	static const char * const k_pch_VirtualDisplay_SerialNumber_String = "serialNumber";
	static const char * const k_pch_VirtualDisplay_ModelNumber_String = "modelNumber";
	static const char * const k_pch_VirtualDisplay_AdditionalLatencyInSeconds_Float = "additionalLatencyInSeconds";
	static const char * const k_pch_VirtualDisplay_DisplayWidth_Int32 = "displayWidth";
	static const char * const k_pch_VirtualDisplay_DisplayHeight_Int32 = "displayHeight";
	static const char * const k_pch_VirtualDisplay_DisplayRefreshRateNumerator_Int32 = "displayRefreshRateNumerator";
	static const char * const k_pch_VirtualDisplay_DisplayRefreshRateDenominator_Int32 = "displayRefreshRateDenominator";
	static const char * const k_pch_VirtualDisplay_AdapterIndex_Int32 = "adapterIndex";

	//-----------------------------------------------------------------------------
	void Log( const char *pFormat, ... )
	{
		va_list args;
		va_start( args, pFormat );

		char buffer[ 1024 ];
		vsprintf_s( buffer, pFormat, args );
		strcat_s( buffer, "\n" );
		vr::VRDriverLog()->Log( buffer );

		va_end( args );
	}

	//-----------------------------------------------------------------------------
	// Interface to separate process standing in for an actual remote device.
	// This needs to be a separate process because D3D blocks gpu work within
	// a process on Present.
	//-----------------------------------------------------------------------------
	class CRemoteDevice
	{
	public:
		CRemoteDevice()
			: m_flFrameIntervalInSeconds( 0.0f )
			, m_pNewFrame( NULL )
		{
			ZeroMemory( &m_procInfo, sizeof( m_procInfo ) );
		}

		~CRemoteDevice()
		{}

		bool Initialize(
			uint32_t nWindowX, uint32_t nWindowY, uint32_t nWindowWidth, uint32_t nWindowHeight,
			uint32_t nRefreshRateNumerator, uint32_t nRefreshRateDenominator )
		{
			if ( nWindowWidth == 0 || nWindowHeight == 0 ||
				nRefreshRateNumerator == 0 || nRefreshRateDenominator == 0 )
			{
				Log( "RemoteDevice: Invalid parameters. w=%d h=%d refresh=%d/%d",
					nWindowWidth, nWindowHeight, nRefreshRateNumerator, nRefreshRateDenominator );
				return false;
			}

			m_flFrameIntervalInSeconds = float( nRefreshRateDenominator ) / nRefreshRateNumerator;

			if ( !m_sharedState.IsValid() )
			{
				Log( "RemoteDevice: Failed to initialize shared memory!" );
				return false;
			}
			else
			{
				CSharedState::Ptr data( &m_sharedState );
				data->m_nTextureWidth = 0;
				data->m_nTextureHeight = 0;
				data->m_flLastVsyncTimeInSeconds = 0.0f;
				data->m_nVsyncCounter = 0;
				data->m_nSystemBaseTimeTicks = SystemTime::GetBaseTicks();
			}

			m_pNewFrame = new IPCEvent( "RemoteDisplay_NewFrame", false, false );
			if ( m_pNewFrame == NULL )
			{
				Log( "RemoteDevice: Failed to create IPC event!" );
				return false;
			}

			std::string sInstallPath = vr::VRProperties()->GetStringProperty( vr::VRDriverHandle(), vr::Prop_InstallPath_String );
			std::string sWorkingPath = sInstallPath + "\\bin\\win32";

			char buffer[ 256 ];
			sprintf_s( buffer, "\"%s\\virtual_display.exe\" %d %d %d %d %d %d",
				sWorkingPath.c_str(),
				nWindowX, nWindowY, nWindowWidth, nWindowHeight,
				nRefreshRateNumerator, nRefreshRateDenominator );

			Log( "Launching %s", buffer );

			STARTUPINFO startupInfo = { 0 };
			startupInfo.cb = sizeof( STARTUPINFO );
			if ( !CreateProcess( NULL, buffer, NULL, NULL, FALSE, NULL, NULL, sWorkingPath.c_str(), &startupInfo, &m_procInfo ) )
			{
				Log( "RemoteDevice: Failed to launch external process." );
				return false;
			}

			return true;
		}

		void Shutdown()
		{
			if ( m_procInfo.hProcess )
			{
				TerminateProcess( m_procInfo.hProcess, 0 );
				CloseHandle( m_procInfo.hProcess );
				CloseHandle( m_procInfo.hThread );
				ZeroMemory( &m_procInfo, sizeof( m_procInfo ) );
			}
		}

		float GetFrameIntervalInSeconds() const
		{
			return m_flFrameIntervalInSeconds;
		}

		void Transmit( uint32_t nWidth, uint32_t nHeight, DXGI_FORMAT format,
			const BYTE *pData, uint32_t nRowPitch, double flVsyncTimeInSeconds )
		{
			EventWriteString( L"[VDispDvr] Transmit(begin)" );

			nWidth = min( nWidth, SharedState_t::MAX_TEXTURE_WIDTH );
			nHeight = min( nHeight, SharedState_t::MAX_TEXTURE_HEIGHT );

			if ( 1 )
			{
				CSharedState::Ptr data( &m_sharedState );
				data->m_nTextureWidth = nWidth;
				data->m_nTextureHeight = nHeight;
				data->m_nTextureFormat = format;
				data->m_flVsyncTimeInSeconds = flVsyncTimeInSeconds;

				CD3DRender::CopyTextureData( data->m_nTextureData, nWidth * SharedState_t::TEXTURE_PITCH,
					pData, nRowPitch, nWidth, nHeight, SharedState_t::TEXTURE_PITCH );

			}

			m_pNewFrame->SetEvent();

			EventWriteString( L"[VDispDvr] Transmit(end)" );
		}

		void GetTimingInfo( double *pflLastVsyncTimeInSeconds, uint32_t *pnVsyncCounter )
		{
			CSharedState::Ptr data( &m_sharedState );
			*pflLastVsyncTimeInSeconds = data->m_flLastVsyncTimeInSeconds;
			*pnVsyncCounter = data->m_nVsyncCounter;
		}

	private:
		CSharedState m_sharedState;
		IPCEvent *m_pNewFrame;
		float m_flFrameIntervalInSeconds;
		PROCESS_INFORMATION m_procInfo;
	};

	//----------------------------------------------------------------------------
	// Blocks on reading backbuffer from gpu, so WaitForPresent can return
	// as soon as we know rendering made it this frame.  This step of the pipeline
	// should run about 3ms per frame.
	//----------------------------------------------------------------------------
	class CEncoder : public CThread
	{
	public:
		CEncoder( CD3DRender *pD3DRender, CRemoteDevice *pRemoteDevice )
			: m_pRemoteDevice( pRemoteDevice )
			, m_pD3DRender( pD3DRender )
			, m_pStagingTexture( NULL )
			, m_bExiting( false )
		{
			m_encodeFinished.Set();
		}

		~CEncoder()
		{
			SAFE_RELEASE( m_pStagingTexture );
		}

		bool CopyToStaging( ID3D11Texture2D *pTexture )
		{
			// Create a staging texture to copy frame data into that can in turn
			// be read back (for blocking until rendering is finished).
			if ( m_pStagingTexture == NULL )
			{
				D3D11_TEXTURE2D_DESC srcDesc;
				pTexture->GetDesc( &srcDesc );

				D3D11_TEXTURE2D_DESC stagingTextureDesc;
				ZeroMemory( &stagingTextureDesc, sizeof( stagingTextureDesc ) );
				stagingTextureDesc.Width = srcDesc.Width;
				stagingTextureDesc.Height = srcDesc.Height;
				stagingTextureDesc.Format = srcDesc.Format;
				stagingTextureDesc.MipLevels = 1;
				stagingTextureDesc.ArraySize = 1;
				stagingTextureDesc.SampleDesc.Count = 1;
				stagingTextureDesc.Usage = D3D11_USAGE_STAGING;
				stagingTextureDesc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;

				if ( FAILED( m_pD3DRender->GetDevice()->CreateTexture2D( &stagingTextureDesc, NULL, &m_pStagingTexture ) ) )
				{
					Log( "Failed to create staging texture!" );
					return false;
				}
			}

			m_pD3DRender->GetContext()->CopyResource( m_pStagingTexture, pTexture );

			return true;
		}

		void Run() override
		{
			SetThreadPriority( GetCurrentThread(), THREAD_PRIORITY_MOST_URGENT );

			while ( !m_bExiting )
			{
				EventWriteString( L"[VDispDvr] Encoder waiting for new frame..." );

				m_newFrameReady.Wait();
				if ( m_bExiting )
					break;

				if ( m_pStagingTexture )
				{
					D3D11_TEXTURE2D_DESC desc;
					m_pStagingTexture->GetDesc( &desc );

					EventWriteString( L"[VDispDvr] Map:Staging(begin)" );

					D3D11_MAPPED_SUBRESOURCE mapped = { 0 };
					if ( SUCCEEDED( m_pD3DRender->GetContext()->Map( m_pStagingTexture, 0, D3D11_MAP_READ, 0, &mapped ) ) )
					{
						EventWriteString( L"[VDispDvr] Map:Staging(end)" );

						m_pRemoteDevice->Transmit( desc.Width, desc.Height, desc.Format,
							( const BYTE * )mapped.pData, mapped.RowPitch, m_flVsyncTimeInSeconds );

						m_pD3DRender->GetContext()->Unmap( m_pStagingTexture, 0 );
					}
				}

				m_encodeFinished.Set();
			}
		}

		void Stop()
		{
			m_bExiting = true;
			m_newFrameReady.Set();
			Join();
		}

		void NewFrameReady( double flVsyncTimeInSeconds )
		{
			m_flVsyncTimeInSeconds = flVsyncTimeInSeconds;
			m_encodeFinished.Reset();
			m_newFrameReady.Set();
		}

		void WaitForEncode()
		{
			m_encodeFinished.Wait();
		}

	private:
		CThreadEvent m_newFrameReady, m_encodeFinished;
		CRemoteDevice *m_pRemoteDevice;
		CD3DRender *m_pD3DRender;
		ID3D11Texture2D *m_pStagingTexture;
		double m_flVsyncTimeInSeconds;
		bool m_bExiting;
	};
}

//-----------------------------------------------------------------------------
// Purpose: This object represents our device (registered below).
// It implements the IVRVirtualDisplay component interface to provide us
// hooks into the render pipeline.
//-----------------------------------------------------------------------------
class CDisplayRedirectLatest : public vr::ITrackedDeviceServerDriver, public vr::IVRVirtualDisplay
{
public:
	CDisplayRedirectLatest()
		: m_unObjectId( vr::k_unTrackedDeviceIndexInvalid )
		, m_nGraphicsAdapterLuid( 0 )
		, m_flLastVsyncTimeInSeconds( 0.0 )
		, m_nVsyncCounter( 0 )
		, m_pD3DRender( NULL )
		, m_pFlushTexture( NULL )
		, m_pRemoteDevice( NULL )
		, m_pEncoder( NULL )
	{
		vr::VRSettings()->GetString( k_pch_VirtualDisplay_Section,
			vr::k_pch_Null_SerialNumber_String, m_rchSerialNumber, ARRAYSIZE( m_rchSerialNumber ) );
		vr::VRSettings()->GetString( k_pch_VirtualDisplay_Section,
			vr::k_pch_Null_ModelNumber_String, m_rchModelNumber, ARRAYSIZE( m_rchModelNumber ) );

		m_flAdditionalLatencyInSeconds = max( 0.0f,
			vr::VRSettings()->GetFloat( k_pch_VirtualDisplay_Section,
				k_pch_VirtualDisplay_AdditionalLatencyInSeconds_Float ) );

		int32_t nDisplayWidth = vr::VRSettings()->GetInt32(
			k_pch_VirtualDisplay_Section,
			k_pch_VirtualDisplay_DisplayWidth_Int32 );
		int32_t nDisplayHeight = vr::VRSettings()->GetInt32(
			k_pch_VirtualDisplay_Section,
			k_pch_VirtualDisplay_DisplayHeight_Int32 );

		int32_t nDisplayRefreshRateNumerator = vr::VRSettings()->GetInt32(
			k_pch_VirtualDisplay_Section,
			k_pch_VirtualDisplay_DisplayRefreshRateNumerator_Int32 );
		int32_t nDisplayRefreshRateDenominator = vr::VRSettings()->GetInt32(
			k_pch_VirtualDisplay_Section,
			k_pch_VirtualDisplay_DisplayRefreshRateDenominator_Int32 );

		int32_t nAdapterIndex = vr::VRSettings()->GetInt32(
			k_pch_VirtualDisplay_Section,
			k_pch_VirtualDisplay_AdapterIndex_Int32 );

		m_pD3DRender = new CD3DRender();

		// First initialize using the specified display dimensions to determine
		// which graphics adapter the headset is attached to (if any).
		if ( !m_pD3DRender->Initialize( nDisplayWidth, nDisplayHeight ) )
		{
			Log( "Could not find headset with display size %dx%d.", nDisplayWidth, nDisplayHeight );
			return;
		}

		int32_t nDisplayX, nDisplayY;
		m_pD3DRender->GetDisplayPos( &nDisplayX, &nDisplayY );

		int32_t nDisplayAdapterIndex;
		const int32_t nBufferSize = 128;
		wchar_t wchAdapterDescription[ nBufferSize ];
		if ( !m_pD3DRender->GetAdapterInfo( &nDisplayAdapterIndex, wchAdapterDescription, nBufferSize ) )
		{
			Log( "Failed to get headset adapter info!" );
			return;
		}

		char chAdapterDescription[ nBufferSize ];
		wcstombs_s( 0, chAdapterDescription, nBufferSize, wchAdapterDescription, nBufferSize );
		Log( "Headset connected to %s.", chAdapterDescription );

		// If no adapter specified, choose the first one the headset *isn't* plugged into.
		if ( nAdapterIndex < 0 )
		{
			nAdapterIndex = ( nDisplayAdapterIndex == 0 ) ? 1 : 0;
		}
		else if ( nDisplayAdapterIndex == nAdapterIndex )
		{
			Log( "Headset needs to be plugged into a separate graphics card." );
			return;
		}

		// Store off the LUID of the primary gpu we want to use.
		if ( !m_pD3DRender->GetAdapterLuid( nAdapterIndex, &m_nGraphicsAdapterLuid ) )
		{
			Log( "Failed to get adapter index for graphics adapter!" );
			return;
		}

		// Now reinitialize using the other graphics card.
		if ( !m_pD3DRender->Initialize( nAdapterIndex ) )
		{
			Log( "Could not create graphics device for adapter %d.  Requires a minimum of two graphics cards.", nAdapterIndex );
			return;
		}

		if ( !m_pD3DRender->GetAdapterInfo( &nDisplayAdapterIndex, wchAdapterDescription, nBufferSize ) )
		{
			Log( "Failed to get primary adapter info!" );
			return;
		}

		wcstombs_s( 0, chAdapterDescription, nBufferSize, wchAdapterDescription, nBufferSize );
		Log( "Using %s as primary graphics adapter.", chAdapterDescription );

		// Spawn our separate process to manage headset presentation.
		m_pRemoteDevice = new CRemoteDevice();
		if ( !m_pRemoteDevice->Initialize(
			nDisplayX, nDisplayY, nDisplayWidth, nDisplayHeight,
			nDisplayRefreshRateNumerator, nDisplayRefreshRateDenominator ) )
		{
			return;
		}

		// Spin up a separate thread to handle the overlapped encoding/transmit step.
		m_pEncoder = new CEncoder( m_pD3DRender, m_pRemoteDevice );
		m_pEncoder->Start();
	}

	virtual ~CDisplayRedirectLatest()
	{
		if ( m_pEncoder )
		{
			m_pEncoder->Stop();
			delete m_pEncoder;
		}

		if ( m_pRemoteDevice )
		{
			m_pRemoteDevice->Shutdown();
			delete m_pRemoteDevice;
		}

		if ( m_pFlushTexture )
		{
			m_pFlushTexture->Release();
		}

		if ( m_pD3DRender )
		{
			m_pD3DRender->Shutdown();
			delete m_pD3DRender;
		}
	}

	bool IsValid() const
	{
		return m_pEncoder != NULL;
	}

	// ITrackedDeviceServerDriver

	virtual vr::EVRInitError Activate( uint32_t unObjectId ) override
	{
		m_unObjectId = unObjectId;

		vr::PropertyContainerHandle_t ulContainer =
			vr::VRProperties()->TrackedDeviceToPropertyContainer( unObjectId );

		vr::VRProperties()->SetStringProperty( ulContainer,
			vr::Prop_ModelNumber_String, m_rchModelNumber );
		vr::VRProperties()->SetFloatProperty( ulContainer,
			vr::Prop_SecondsFromVsyncToPhotons_Float, m_flAdditionalLatencyInSeconds );
		vr::VRProperties()->SetUint64Property( ulContainer,
			vr::Prop_GraphicsAdapterLuid_Uint64, m_nGraphicsAdapterLuid );

		return vr::VRInitError_None;
	}

	virtual void Deactivate() override
	{
		m_unObjectId = vr::k_unTrackedDeviceIndexInvalid;
	}

	virtual void *GetComponent( const char *pchComponentNameAndVersion ) override
	{
		if ( !_stricmp( pchComponentNameAndVersion, vr::IVRVirtualDisplay_Version ) )
		{
			return static_cast< vr::IVRVirtualDisplay * >( this );
		}
		return NULL;
	}

	virtual void EnterStandby() override
	{
	}

	virtual void DebugRequest( const char *pchRequest, char *pchResponseBuffer, uint32_t unResponseBufferSize ) override
	{
		if( unResponseBufferSize >= 1 )
			pchResponseBuffer[0] = 0;
	}

	virtual vr::DriverPose_t GetPose() override
	{
		vr::DriverPose_t pose = { 0 };
		pose.poseIsValid = true;
		pose.result = vr::TrackingResult_Running_OK;
		pose.deviceIsConnected = true;
		pose.qWorldFromDriverRotation.w = 1;
		pose.qWorldFromDriverRotation.x = 0;
		pose.qWorldFromDriverRotation.y = 0;
		pose.qWorldFromDriverRotation.z = 0;
		pose.qDriverFromHeadRotation.w = 1;
		pose.qDriverFromHeadRotation.x = 0;
		pose.qDriverFromHeadRotation.y = 0;
		pose.qDriverFromHeadRotation.z = 0;
		return pose;
	}

	std::string GetSerialNumber()
	{
		return m_rchSerialNumber;
	}

	// IVRVirtualDisplay

	virtual void Present( vr::SharedTextureHandle_t backbufferTextureHandle ) override
	{
		// Open and cache our shared textures to avoid re-opening every frame.
		ID3D11Texture2D *pTexture = m_pD3DRender->GetSharedTexture( ( HANDLE )backbufferTextureHandle );
		if ( pTexture == NULL )
		{
			EventWriteString( L"[VDispDvr] Texture is NULL!" );
		}
		else
		{
			EventWriteString( L"[VDispDvr] Waiting for previous encode to finish..." );

			// Wait for the encoder to be ready.  This is important because the encoder thread
			// blocks on transmit which uses our shared d3d context (which is not thread safe).
			m_pEncoder->WaitForEncode();

			EventWriteString( L"[VDispDvr] Done" );

			// Access to shared texture must be wrapped in AcquireSync/ReleaseSync
			// to ensure the compositor has finished rendering to it before it gets used.
			// This enforces scheduling of work on the gpu between processes.
			IDXGIKeyedMutex *pKeyedMutex = NULL;
			if ( SUCCEEDED( pTexture->QueryInterface( __uuidof( IDXGIKeyedMutex ), ( void ** )&pKeyedMutex ) ) )
			{
				if ( pKeyedMutex->AcquireSync( 0, 10 ) != S_OK )
				{
					pKeyedMutex->Release();
					EventWriteString( L"[VDispDvr] ACQUIRESYNC FAILED!!!" );
					return;
				}
			}

			EventWriteString( L"[VDispDvr] AcquiredSync" );

			if ( m_pFlushTexture == NULL )
			{
				D3D11_TEXTURE2D_DESC srcDesc;
				pTexture->GetDesc( &srcDesc );

				// Create a second small texture for copying and reading a single pixel from
				// in order to block on the cpu until rendering is finished.
				D3D11_TEXTURE2D_DESC flushTextureDesc;
				ZeroMemory( &flushTextureDesc, sizeof( flushTextureDesc ) );
				flushTextureDesc.Width = 32;
				flushTextureDesc.Height = 32;
				flushTextureDesc.MipLevels = 1;
				flushTextureDesc.ArraySize = 1;
				flushTextureDesc.Format = srcDesc.Format;
				flushTextureDesc.SampleDesc.Count = 1;
				flushTextureDesc.Usage = D3D11_USAGE_STAGING;
				flushTextureDesc.BindFlags = 0;
				flushTextureDesc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;

				if ( FAILED( m_pD3DRender->GetDevice()->CreateTexture2D( &flushTextureDesc, NULL, &m_pFlushTexture ) ) )
				{
					Log( "Failed to create flush texture!" );
					return;
				}
			}

			// Copy a single pixel so we can block until rendering is finished in WaitForPresent.
			D3D11_BOX box = { 0, 0, 0, 1, 1, 1 };
			m_pD3DRender->GetContext()->CopySubresourceRegion( m_pFlushTexture, 0, 0, 0, 0, pTexture, 0, &box );

			EventWriteString( L"[VDispDvr] Flush-Begin" );

			// This can go away, but is useful to see it as a separate packet on the gpu in traces.
			m_pD3DRender->GetContext()->Flush();

			EventWriteString( L"[VDispDvr] Flush-End" );

			// Copy entire texture to staging so we can read the pixels to send to remote device.
			m_pEncoder->CopyToStaging( pTexture );

			EventWriteString( L"[VDispDvr] Flush-Staging(begin)" );

			m_pD3DRender->GetContext()->Flush();

			EventWriteString( L"[VDispDvr] Flush-Staging(end)" );

			if ( pKeyedMutex )
			{
				pKeyedMutex->ReleaseSync( 0 );
				pKeyedMutex->Release();
			}

			EventWriteString( L"[VDispDvr] ReleasedSync" );
		}
	}

	virtual void WaitForPresent() override
	{
		EventWriteString( L"[VDispDvr] WaitForPresent(begin)" );

		// First wait for rendering to finish on the gpu.
		if ( m_pFlushTexture )
		{
			D3D11_MAPPED_SUBRESOURCE mapped = { 0 };
			if ( SUCCEEDED( m_pD3DRender->GetContext()->Map( m_pFlushTexture, 0, D3D11_MAP_READ, 0, &mapped ) ) )
			{
				EventWriteString( L"[VDispDvr] Mapped FlushTexture" );

				m_pD3DRender->GetContext()->Unmap( m_pFlushTexture, 0 );
			}
		}

		EventWriteString( L"[VDispDvr] RenderingFinished" );

		// Now that we know rendering is done, we can fire off our thread that reads the
		// backbuffer into system memory.  We also pass in the earliest time that this frame
		// should get presented.  This is the real vsync that starts our frame.
		m_pEncoder->NewFrameReady( m_flLastVsyncTimeInSeconds + m_flAdditionalLatencyInSeconds );

		// Get latest timing info to work with.  This gets us sync'd up with the hardware in
		// the first place, and also avoids any drifting over time.
		double flLastVsyncTimeInSeconds;
		uint32_t nVsyncCounter;
		m_pRemoteDevice->GetTimingInfo( &flLastVsyncTimeInSeconds, &nVsyncCounter );

		// Account for encoder/transmit latency.
		// This is where the conversion from real to virtual vsync happens.
		flLastVsyncTimeInSeconds -= m_flAdditionalLatencyInSeconds;

		float flFrameIntervalInSeconds = m_pRemoteDevice->GetFrameIntervalInSeconds();

		// Realign our last time interval given updated timing reference.
		int32_t nTimeRefToLastVsyncFrames =
			( int32_t )roundf( float( m_flLastVsyncTimeInSeconds - flLastVsyncTimeInSeconds ) / flFrameIntervalInSeconds );
		m_flLastVsyncTimeInSeconds = flLastVsyncTimeInSeconds + flFrameIntervalInSeconds * nTimeRefToLastVsyncFrames;

		// We could probably just use this instead, but it seems safer to go off the system timer calculation.
		assert( m_nVsyncCounter == nVsyncCounter + nTimeRefToLastVsyncFrames );

		double flNow = SystemTime::GetInSeconds();

		// Find the next frame interval (keeping in mind we may get here during running start).
		int32_t nLastVsyncToNextVsyncFrames =
			( int32_t )( float( flNow - m_flLastVsyncTimeInSeconds ) / flFrameIntervalInSeconds );
		nLastVsyncToNextVsyncFrames = max( nLastVsyncToNextVsyncFrames, 0 ) + 1;

		// And store it for use in GetTimeSinceLastVsync (below) and updating our next frame.
		m_flLastVsyncTimeInSeconds += flFrameIntervalInSeconds * nLastVsyncToNextVsyncFrames;
		m_nVsyncCounter = nVsyncCounter + nTimeRefToLastVsyncFrames + nLastVsyncToNextVsyncFrames;

		EventWriteString( L"[VDispDvr] WaitForPresent(end)" );
	}

	virtual bool GetTimeSinceLastVsync( float *pfSecondsSinceLastVsync, uint64_t *pulFrameCounter ) override
	{
		*pfSecondsSinceLastVsync = ( float )( SystemTime::GetInSeconds() - m_flLastVsyncTimeInSeconds );
		*pulFrameCounter = m_nVsyncCounter;
		return true;
	}

private:
	uint32_t m_unObjectId;
	char m_rchSerialNumber[ 1024 ];
	char m_rchModelNumber[ 1024 ];
	uint64_t m_nGraphicsAdapterLuid;
	float m_flAdditionalLatencyInSeconds;
	double m_flLastVsyncTimeInSeconds;
	uint32_t m_nVsyncCounter;

	CD3DRender *m_pD3DRender;
	ID3D11Texture2D *m_pFlushTexture;
	CRemoteDevice *m_pRemoteDevice;
	CEncoder *m_pEncoder;
};


//-----------------------------------------------------------------------------
// Purpose: Server interface implementation.
//-----------------------------------------------------------------------------
class CServerDriver_DisplayRedirect : public vr::IServerTrackedDeviceProvider
{
public:
	CServerDriver_DisplayRedirect()
		: m_pDisplayRedirectLatest( NULL )
	{}

	virtual vr::EVRInitError Init( vr::IVRDriverContext *pContext ) override;
	virtual void Cleanup() override;
	virtual const char * const *GetInterfaceVersions() override
		{ return vr::k_InterfaceVersions;  }
	virtual const char *GetTrackedDeviceDriverVersion()
		{ return vr::ITrackedDeviceServerDriver_Version; }
	virtual void RunFrame() override {}
	virtual bool ShouldBlockStandbyMode() override { return false; }
	virtual void EnterStandby() override {}
	virtual void LeaveStandby() override {}

private:
	CDisplayRedirectLatest *m_pDisplayRedirectLatest;
};

vr::EVRInitError CServerDriver_DisplayRedirect::Init( vr::IVRDriverContext *pContext )
{
	VR_INIT_SERVER_DRIVER_CONTEXT( pContext );

	m_pDisplayRedirectLatest = new CDisplayRedirectLatest();

	if ( m_pDisplayRedirectLatest->IsValid() )
	{
		vr::VRServerDriverHost()->TrackedDeviceAdded(
			m_pDisplayRedirectLatest->GetSerialNumber().c_str(),
			vr::TrackedDeviceClass_DisplayRedirect,
			m_pDisplayRedirectLatest );
	}

	return vr::VRInitError_None;
}

void CServerDriver_DisplayRedirect::Cleanup()
{
	delete m_pDisplayRedirectLatest;
	m_pDisplayRedirectLatest = NULL;

	VR_CLEANUP_SERVER_DRIVER_CONTEXT();
}

CServerDriver_DisplayRedirect g_serverDriverDisplayRedirect;

//-----------------------------------------------------------------------------
// Purpose: Entry point for vrserver when loading drivers.
//-----------------------------------------------------------------------------
extern "C" __declspec( dllexport )
void *HmdDriverFactory( const char *pInterfaceName, int *pReturnCode )
{
	if ( 0 == strcmp( vr::IServerTrackedDeviceProvider_Version, pInterfaceName ) )
	{
		return &g_serverDriverDisplayRedirect;
	}

	if( pReturnCode )
		*pReturnCode = vr::VRInitError_Init_InterfaceNotFound;

	return NULL;
}

