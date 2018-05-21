//===================== Copyright (c) Valve Corporation. All Rights Reserved. ======================
//
// Example OpenVR driver for demonstrating IVRVirtualDisplay interface.
//
//==================================================================================================
#define _WINSOCKAPI_
#include "openvr_driver.h"
#include "sharedstate.h"
#include "threadtools.h"
#include "systemtime.h"
#include "d3drender.h"

#include <winsock2.h>
#include <d3d11.h>
#include <wrl.h>
#include <map>
#include <d3d11_1.h>
#include <ScreenGrab.h>
#include <wincodec.h>
#include <wincodecsdk.h>

#include "NvEncoderD3D11.h"
#include "Logger.h"
#include "NvCodecUtils.h"
#include "UdpSender.h"
#include "nvencoderclioptions.h"
#include "Listener.h"
#include "Utils.h"
#include "FrameRender.h"

HINSTANCE g_hInstance;

std::string g_DebugOutputDir;

uint64_t g_DriverTestMode = 0;

namespace
{
	using Microsoft::WRL::ComPtr;
	
	void DrawDebugTimestamp(CD3DRender *m_pD3DRender, ID3D11Texture2D *pTexture)
	{
		D3D11_MAPPED_SUBRESOURCE mapped = { 0 };
		HRESULT hr = m_pD3DRender->GetContext()->Map(pTexture, 0, D3D11_MAP_READ, 0, &mapped);
		if (SUCCEEDED(hr))
		{
			int x = 10;
			int y = 10;

			FILETIME ft;
			SYSTEMTIME st2, st;

			GetSystemTimeAsFileTime(&ft);
			FileTimeToSystemTime(&ft, &st2);
			SystemTimeToTzSpecificLocalTime(NULL, &st2, &st);

			uint64_t q = (((uint64_t)ft.dwHighDateTime) << 32) | ft.dwLowDateTime;
			q /= 10;
			char buf[100];
			snprintf(buf, sizeof(buf),
				"%02d %02d %02d %03lld %03lld",
				st.wHour, st.wMinute, st.wSecond, q / 1000 % 1000, q % 1000);

			for (int i = 0; buf[i]; i++) {
				if (buf[i] != ' ') {
					DrawDigitPixels(mapped, x, y, buf[i] - '0');
				}
				x += 10;
			}

			m_pD3DRender->GetContext()->Unmap(pTexture, 0);
		}
		else {
			Log("DrawDebugTimestamp failed: %p %s", hr, GetDxErrorStr(hr).c_str());
		}
	}

	void SaveDebugOutput(CD3DRender *m_pD3DRender, std::vector<std::vector<uint8_t>> &vPacket, ID3D11Texture2D *texture, uint64_t frameIndex) {
		if (vPacket.size() == 0) {
			return;
		}
		if (vPacket[0].size() < 10) {
			return;
		}
		int type = vPacket[0][4] & 0x1F;
		if (type == 7) {
			// SPS, PPS, IDR
			char filename[1000];
			wchar_t filename2[1000];
			snprintf(filename, sizeof(filename), "%s\\%llu.h264", g_DebugOutputDir.c_str(), frameIndex);
			_snwprintf_s(filename2, sizeof(filename2), L"%hs\\%llu.dds", g_DebugOutputDir.c_str(), frameIndex);
			FILE *fp;
			fopen_s(&fp, filename, "wb");
			if (fp) {
				for (auto packet : vPacket) {
					fwrite(&packet[0], packet.size(), 1, fp);
				}
				fclose(fp);
			}
			DirectX::SaveDDSTextureToFile(m_pD3DRender->GetContext(), texture, filename2);
		}
	}



	inline vr::HmdQuaternion_t HmdQuaternion_Init(double w, double x, double y, double z)
	{
		vr::HmdQuaternion_t quat;
		quat.w = w;
		quat.x = x;
		quat.y = y;
		quat.z = z;
		return quat;
	}

	inline void HmdMatrix_SetIdentity(vr::HmdMatrix34_t *pMatrix)
	{
		pMatrix->m[0][0] = 1.f;
		pMatrix->m[0][1] = 0.f;
		pMatrix->m[0][2] = 0.f;
		pMatrix->m[0][3] = 0.f;
		pMatrix->m[1][0] = 0.f;
		pMatrix->m[1][1] = 1.f;
		pMatrix->m[1][2] = 0.f;
		pMatrix->m[1][3] = 0.f;
		pMatrix->m[2][0] = 0.f;
		pMatrix->m[2][1] = 0.f;
		pMatrix->m[2][2] = 1.f;
		pMatrix->m[2][3] = 0.f;
	}

		
	//-----------------------------------------------------------------------------
	// Settings
	//-----------------------------------------------------------------------------
	static const char * const k_pch_Settings_Section = "driver_remote_glass";
	static const char * const k_pch_Settings_SerialNumber_String = "serialNumber";
	static const char * const k_pch_Settings_ModelNumber_String = "modelNumber";
	static const char * const k_pch_Settings_RenderWidth_Int32 = "renderWidth";
	static const char * const k_pch_Settings_RenderHeight_Int32 = "renderHeight";
	static const char * const k_pch_Settings_IPD_Float = "IPD";
	static const char * const k_pch_Settings_SecondsFromVsyncToPhotons_Float = "secondsFromVsyncToPhotons";
	static const char * const k_pch_Settings_DisplayFrequency_Float = "displayFrequency";
	static const char * const k_pch_Settings_EncoderOptions_String = "nvencOptions";
	static const char * const k_pch_Settings_DebugLog_Bool = "debugLog";
	static const char * const k_pch_Settings_DebugTimestamp_Bool = "debugTimestamp";
	static const char * const k_pch_Settings_DebugFrameIndex_Bool = "debugFrameIndex";
	static const char * const k_pch_Settings_DebugFrameOutput_Bool = "debugFrameOutput";
	static const char * const k_pch_Settings_DebugCaptureOutput_Bool = "debugCaptureOutput";
	static const char * const k_pch_Settings_DebugOutputDir = "debugOutputDir";
	static const char * const k_pch_Settings_ListenHost_String = "listenHost";
	static const char * const k_pch_Settings_ListenPort_Int32 = "listenPort";
	static const char * const k_pch_Settings_ControlListenHost_String = "controlListenHost";
	static const char * const k_pch_Settings_ControlListenPort_Int32 = "controlListenPort";

	static const char * const k_pch_Settings_AdapterIndex_Int32 = "adapterIndex";

	static const char * const k_pch_Settings_SrtOptions_String = "srtOptions";
	static const char * const k_pch_Settings_SendingTimeslotUs_Int32 = "sendingTimeslotUs";
	static const char * const k_pch_Settings_LimitTimeslotPackets_Int32 = "limitTimeslotPackets";

	class CNvEncoder
	{
	public:
		CNvEncoder(CD3DRender *pD3DRender,
			bool DebugTimestamp, bool DebugFrameOutput, bool DebugCaptureOutput)
			: enc(NULL)
			, m_pD3DRender(pD3DRender)
			, m_nFrame(0)
			, m_Listener(NULL)
			, m_DebugTimestamp(DebugTimestamp)
			, m_DebugFrameOutput(DebugFrameOutput)
			, m_DebugCaptureOutput(DebugCaptureOutput)
		{
		}

		~CNvEncoder()
		{}

		bool Initialize(
			std::string encoderOptions, Listener *listener, int nWidth, int nHeight)
		{
			NvEncoderInitParam EncodeCLIOptions(encoderOptions.c_str());

			//m_pD3DRender->GetDevice()->CreateDeferredContext(0, &m_DeferredContext);
			
			//
			// Initialize Encoder
			//

			Log("Initializing CNvEncoder. Width=%d Height=%d", nWidth, nHeight);

			NV_ENC_BUFFER_FORMAT format = NV_ENC_BUFFER_FORMAT_ABGR;

			enc = new NvEncoderD3D11(m_pD3DRender->GetDevice(), nWidth, nHeight, format, 0);

			NV_ENC_INITIALIZE_PARAMS initializeParams = { NV_ENC_INITIALIZE_PARAMS_VER };
			NV_ENC_CONFIG encodeConfig = { NV_ENC_CONFIG_VER };

			initializeParams.encodeConfig = &encodeConfig;
			enc->CreateDefaultEncoderParams(&initializeParams, EncodeCLIOptions.GetEncodeGUID(), EncodeCLIOptions.GetPresetGUID());

			initializeParams.encodeConfig->encodeCodecConfig.h264Config.repeatSPSPPS = 1;

			EncodeCLIOptions.SetInitParams(&initializeParams, format);

			std::string parameterDesc = EncodeCLIOptions.FullParamToString(&initializeParams);
			Log("NvEnc Encoder Parameters:\n%s", parameterDesc.c_str());

			enc->CreateEncoder(&initializeParams);

			//
			// Initialize debug video output
			//

			if (g_DebugOutputDir != "" && m_DebugCaptureOutput) {
				std::string outputFile = g_DebugOutputDir + "\\capture.h264";
				fpOut = std::ofstream(outputFile, std::ios::out | std::ios::binary);
				if (!fpOut)
				{
					Log("unable to open output file %s", outputFile.c_str());
				}
			}

			m_Listener = listener;

			return true;
		}

		void Shutdown()
		{
			std::vector<std::vector<uint8_t>> vPacket;
			enc->EndEncode(vPacket);
			for (std::vector<uint8_t> &packet : vPacket)
			{
				if (fpOut) {
					fpOut.write(reinterpret_cast<char*>(packet.data()), packet.size());
				}
				m_Listener->Send(packet.data(), (int)packet.size(), GetTimestampUs(), 0);
			}

			enc->DestroyEncoder();
			delete enc;

			Log("CNvEncoder::Shutdown");

			if (fpOut) {
				fpOut.close();
			}
		}

		void Transmit(ID3D11Texture2D *pTexture, uint64_t presentationTime, uint64_t frameIndex, uint64_t frameIndex2, uint64_t clientTime)
		{
			std::vector<std::vector<uint8_t>> vPacket;
			D3D11_TEXTURE2D_DESC desc;

			pTexture->GetDesc(&desc);

			Log("[VDispDvr] Transmit(begin) FrameIndex=%llu", frameIndex);

			const NvEncInputFrame* encoderInputFrame = enc->GetNextInputFrame();

			if (m_DebugTimestamp) {
				DrawDebugTimestamp(m_pD3DRender, pTexture);
			}

			ID3D11Texture2D *pTexBgra = reinterpret_cast<ID3D11Texture2D*>(encoderInputFrame->inputPtr);
			Log("CopyResource start");
			m_pD3DRender->GetContext()->CopyResource(pTexBgra, pTexture);
			//m_DeferredContext->CopyResource(pTexBgra, pTexture);

			Log("EncodeFrame start");
			enc->EncodeFrame(vPacket);

			Log("Tracking info delay: %lld us FrameIndex=%llu", GetTimestampUs() - m_Listener->clientToServerTime(clientTime), frameIndex);
			Log("Encoding delay: %lld us FrameIndex=%llu", GetTimestampUs() - presentationTime, frameIndex);

			m_nFrame += (int)vPacket.size();
			for (std::vector<uint8_t> &packet : vPacket)
			{
				if (fpOut) {
					fpOut.write(reinterpret_cast<char*>(packet.data()), packet.size());
				}
				if (m_Listener) {
					m_Listener->Send(packet.data(), (int)packet.size(), presentationTime, frameIndex);
				}
			}

			if (m_DebugFrameOutput) {
				SaveDebugOutput(m_pD3DRender, vPacket, pTexBgra, frameIndex2);
			}

			{
				CSharedState::Ptr data(&m_sharedState);
				data->m_flLastVsyncTimeInSeconds = SystemTime::GetInSeconds();
				data->m_nVsyncCounter++;
			}

			Log("[VDispDvr] Transmit(end) (frame %d %d) FrameIndex=%llu", vPacket.size(), m_nFrame, frameIndex);
		}

		void GetTimingInfo(double *pflLastVsyncTimeInSeconds, uint32_t *pnVsyncCounter)
		{
			CSharedState::Ptr data(&m_sharedState);
			*pflLastVsyncTimeInSeconds = data->m_flLastVsyncTimeInSeconds;
			*pnVsyncCounter = data->m_nVsyncCounter;
		}

	private:
		CSharedState m_sharedState;
		std::ofstream fpOut;
		NvEncoderD3D11 *enc;

		CD3DRender *m_pD3DRender;
		int m_nFrame;

		Listener *m_Listener;
		bool m_DebugTimestamp;
		bool m_DebugFrameOutput;
		bool m_DebugCaptureOutput;

		//ComPtr<ID3D11DeviceContext> m_DeferredContext;
	};

	//----------------------------------------------------------------------------
	// Blocks on reading backbuffer from gpu, so WaitForPresent can return
	// as soon as we know rendering made it this frame.  This step of the pipeline
	// should run about 3ms per frame.
	//----------------------------------------------------------------------------
	class CEncoder : public CThread
	{
	public:
		CEncoder( CD3DRender *pD3DRender, CNvEncoder *pRemoteDevice, int renderWidth, int renderHeight, bool debugFrameIndex )
			: m_pRemoteDevice( pRemoteDevice )
			, m_bExiting( false )
			, m_frameIndex(0)
			, m_frameIndex2(0)
		{
			m_FrameRender = new FrameRender(renderWidth, renderHeight, debugFrameIndex, pD3DRender);
			m_encodeFinished.Set();
		}

		~CEncoder()
		{
		}

		bool CopyToStaging( ID3D11Texture2D *pTexture[][2], vr::VRTextureBounds_t bounds[][2], int layerCount, uint64_t presentationTime, uint64_t frameIndex, uint64_t clientTime, const std::string& debugText)
		{
			m_presentationTime = presentationTime;
			m_frameIndex = frameIndex;
			m_clientTime = clientTime;
			m_FrameRender->Startup();

			char buf[200];
			snprintf(buf, sizeof(buf), "\nindex2: %llu", m_frameIndex2);

			m_FrameRender->RenderFrame(pTexture, bounds, layerCount, debugText + buf);
			return true;
		}

		void Run() override
		{
			SetThreadPriority( GetCurrentThread(), THREAD_PRIORITY_MOST_URGENT );

			while ( !m_bExiting )
			{
				Log( "[VDispDvr] Encoder waiting for new frame..." );

				m_newFrameReady.Wait();
				if ( m_bExiting )
					break;

				if ( m_FrameRender->GetTexture() )
				{
					m_pRemoteDevice->Transmit(m_FrameRender->GetTexture().Get(), m_presentationTime, m_frameIndex, m_frameIndex2, m_clientTime);
				}

				m_frameIndex2++;

				m_encodeFinished.Set();
			}
		}

		void Stop()
		{
			m_bExiting = true;
			m_newFrameReady.Set();
			Join();
			delete m_FrameRender;
		}

		void NewFrameReady()
		{
			Log("New Frame Ready");
			m_encodeFinished.Reset();
			m_newFrameReady.Set();
		}

		void WaitForEncode()
		{
			m_encodeFinished.Wait();
		}

	private:
		CThreadEvent m_newFrameReady, m_encodeFinished;
		CNvEncoder *m_pRemoteDevice;
		bool m_bExiting;
		uint64_t m_presentationTime;
		uint64_t m_frameIndex;
		uint64_t m_clientTime;

		uint64_t m_frameIndex2;

		FrameRender *m_FrameRender;
	};
}

// VSync Event Thread

class VSyncThread : public CThread
{
public:
	VSyncThread() : m_bExit(false) {}

	void Run()override {
		while (!m_bExit) {
			uint64_t current = GetTimestampUs();
			m_PreviousVsync = current;

			if (current - m_InsertedVsync < 16666) {
				int sleepTime = (int)((m_InsertedVsync + 16666) - current) / 1000;
				Log("Skip VSync Event. Sleep %llu ms", sleepTime);
				Sleep(sleepTime);
			}
			else {
				Log("Generate VSync Event");
				vr::VRServerDriverHost()->VsyncEvent(0);
				Sleep(((m_PreviousVsync + 16666) - GetTimestampUs()) / 1000);
			}
		}
	}

	void Shutdown() {
		m_bExit = true;
	}

	void InsertVsync() {
		Log("Insert VSync Event");
		vr::VRServerDriverHost()->VsyncEvent(0);
		m_InsertedVsync = GetTimestampUs();
	}
private:
	bool m_bExit;
	uint64_t m_PreviousVsync;
	uint64_t m_InsertedVsync;
};

//-----------------------------------------------------------------------------
// Purpose:
//-----------------------------------------------------------------------------
class CRemoteHmd : public vr::ITrackedDeviceServerDriver, public vr::IVRDisplayComponent, public vr::IVRDriverDirectModeComponent
{
public:
	CRemoteHmd()
		: m_unObjectId(vr::k_unTrackedDeviceIndexInvalid)
		, m_nGraphicsAdapterLuid(0)
		, m_nVsyncCounter(0)
		, m_pD3DRender(NULL)
		, m_pFlushTexture(NULL)
		, m_pRemoteDevice(NULL)
		, m_pEncoder(NULL)
		, m_EncoderOptions("")
		, m_Listener(NULL)
		, m_VSyncThread(NULL)
		, m_poseMutex(NULL)
		, m_captureDDSTrigger(false)
	{
		std::string host, control_host;
		int port, control_port;

		m_unObjectId = vr::k_unTrackedDeviceIndexInvalid;
		m_ulPropertyContainer = vr::k_ulInvalidPropertyContainer;

		Log("Using settings values");

		char buf[10240];
		vr::VRSettings()->GetString(k_pch_Settings_Section, k_pch_Settings_SerialNumber_String, buf, sizeof(buf));
		m_sSerialNumber = buf;

		vr::VRSettings()->GetString(k_pch_Settings_Section, k_pch_Settings_ModelNumber_String, buf, sizeof(buf));
		m_sModelNumber = buf;

		m_nRenderWidth = vr::VRSettings()->GetInt32(k_pch_Settings_Section, k_pch_Settings_RenderWidth_Int32);
		m_nRenderHeight = vr::VRSettings()->GetInt32(k_pch_Settings_Section, k_pch_Settings_RenderHeight_Int32);
		m_flSecondsFromVsyncToPhotons = vr::VRSettings()->GetFloat(k_pch_Settings_Section, k_pch_Settings_SecondsFromVsyncToPhotons_Float);
		m_flDisplayFrequency = vr::VRSettings()->GetFloat(k_pch_Settings_Section, k_pch_Settings_DisplayFrequency_Float);

		int32_t nAdapterIndex = vr::VRSettings()->GetInt32(k_pch_Settings_Section, k_pch_Settings_AdapterIndex_Int32);

		vr::VRSettings()->GetString(k_pch_Settings_Section, k_pch_Settings_EncoderOptions_String, buf, sizeof(buf));
		m_EncoderOptions = buf;
		vr::VRSettings()->GetString(k_pch_Settings_Section, k_pch_Settings_DebugOutputDir, buf, sizeof(buf));
		g_DebugOutputDir = buf;
		
		vr::VRSettings()->GetString(k_pch_Settings_Section, k_pch_Settings_SrtOptions_String, buf, sizeof(buf));
		std::string SrtOptions = buf;

		// Listener Parameters
		vr::VRSettings()->GetString(k_pch_Settings_Section, k_pch_Settings_ListenHost_String, buf, sizeof(buf));
		host = buf;
		port = vr::VRSettings()->GetInt32(k_pch_Settings_Section, k_pch_Settings_ListenPort_Int32);


		uint64_t sendingTimeslotUs = vr::VRSettings()->GetInt32(k_pch_Settings_Section, k_pch_Settings_SendingTimeslotUs_Int32);
		uint64_t limitTimeslotPackets = vr::VRSettings()->GetInt32(k_pch_Settings_Section, k_pch_Settings_LimitTimeslotPackets_Int32);
		
		vr::VRSettings()->GetString(k_pch_Settings_Section, k_pch_Settings_ControlListenHost_String, buf, sizeof(buf));
		control_host = buf;
		control_port = vr::VRSettings()->GetInt32(k_pch_Settings_Section, k_pch_Settings_ControlListenPort_Int32);

		bool DebugLog = vr::VRSettings()->GetBool(k_pch_Settings_Section, k_pch_Settings_DebugLog_Bool);
		bool DebugTimestamp = vr::VRSettings()->GetBool(k_pch_Settings_Section, k_pch_Settings_DebugTimestamp_Bool);
		bool DebugFrameIndex = vr::VRSettings()->GetBool(k_pch_Settings_Section, k_pch_Settings_DebugFrameIndex_Bool);
		bool DebugFrameOutput = vr::VRSettings()->GetBool(k_pch_Settings_Section, k_pch_Settings_DebugFrameOutput_Bool);
		bool DebugCaptureOutput = vr::VRSettings()->GetBool(k_pch_Settings_Section, k_pch_Settings_DebugCaptureOutput_Bool);

		if (DebugLog) {
			OpenLog((g_DebugOutputDir + "\\driver.log").c_str());
		}
		
		float originalIPD = vr::VRSettings()->GetFloat(vr::k_pch_SteamVR_Section, vr::k_pch_SteamVR_IPD_Float);

		m_flIPD = vr::VRSettings()->GetFloat(k_pch_Settings_Section, k_pch_Settings_IPD_Float);
		vr::VRSettings()->SetFloat(vr::k_pch_SteamVR_Section, vr::k_pch_SteamVR_IPD_Float, m_flIPD);

		Log("driver_null: Serial Number: %s", m_sSerialNumber.c_str());
		Log("driver_null: Model Number: %s", m_sModelNumber.c_str());
		Log("driver_null: Render Target: %d %d", m_nRenderWidth, m_nRenderHeight);
		Log("driver_null: Seconds from Vsync to Photons: %f", m_flSecondsFromVsyncToPhotons);
		Log("driver_null: Display Frequency: %f", m_flDisplayFrequency);
		Log("driver_null: IPD: %f", m_flIPD);

		Log("driver_null: EncoderOptions: %s%s", m_EncoderOptions.c_str(), m_EncoderOptions.size() == sizeof(buf) - 1 ? " (Maybe truncated)" : "");
		

		m_pD3DRender = new CD3DRender();
		
		// Store off the LUID of the primary gpu we want to use.
		if (!m_pD3DRender->GetAdapterLuid(nAdapterIndex, &m_nGraphicsAdapterLuid))
		{
			Log("Failed to get adapter index for graphics adapter!");
			return;
		}

		// Now reinitialize using the other graphics card.
		if (!m_pD3DRender->Initialize(nAdapterIndex))
		{
			Log("Could not create graphics device for adapter %d.  Requires a minimum of two graphics cards.", nAdapterIndex);
			return;
		}

		int32_t nDisplayAdapterIndex;
		wchar_t wchAdapterDescription[300];
		if (!m_pD3DRender->GetAdapterInfo(&nDisplayAdapterIndex, wchAdapterDescription, sizeof(wchAdapterDescription)))
		{
			Log("Failed to get primary adapter info!");
			return;
		}

		Log("Using %ls as primary graphics adapter.", wchAdapterDescription);

		std::function<void(std::string, std::string)> Callback = [&](std::string commandName, std::string args) { CommandCallback(commandName, args); };
		std::function<void()> poseCallback = [&]() { OnPoseUpdated(); };
		m_Listener = new Listener(host, port, control_host, control_port, SrtOptions, sendingTimeslotUs, limitTimeslotPackets, Callback, poseCallback);
		m_Listener->Start();

		// Spawn our separate process to manage headset presentation.
		m_pRemoteDevice = new CNvEncoder(m_pD3DRender, DebugTimestamp, DebugFrameOutput, DebugCaptureOutput);
		if (!m_pRemoteDevice->Initialize(
			m_EncoderOptions, m_Listener, m_nRenderWidth, m_nRenderHeight))
		{
			return;
		}

		// Spin up a separate thread to handle the overlapped encoding/transmit step.
		m_pEncoder = new CEncoder(m_pD3DRender, m_pRemoteDevice, m_nRenderWidth, m_nRenderHeight, DebugFrameIndex);
		m_pEncoder->Start();

		m_VSyncThread = new VSyncThread();
		m_VSyncThread->Start();
	}

	virtual ~CRemoteHmd()
	{
		if (m_pEncoder)
		{
			m_pEncoder->Stop();
			delete m_pEncoder;
		}

		if (m_pRemoteDevice)
		{
			m_pRemoteDevice->Shutdown();
			delete m_pRemoteDevice;
		}

		if (m_Listener)
		{
			m_Listener->Stop();
			delete m_Listener;
		}

		if (m_VSyncThread)
		{
			m_VSyncThread->Shutdown();
			delete m_VSyncThread;
		}

		if (m_pFlushTexture)
		{
			m_pFlushTexture->Release();
		}

		if (m_pD3DRender)
		{
			m_pD3DRender->Shutdown();
			delete m_pD3DRender;
		}
	}


	virtual vr::EVRInitError Activate(vr::TrackedDeviceIndex_t unObjectId)
	{
		Log("CRemoteHmd Activate %d", unObjectId);

		m_unObjectId = unObjectId;
		m_ulPropertyContainer = vr::VRProperties()->TrackedDeviceToPropertyContainer(m_unObjectId);


		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_ModelNumber_String, m_sModelNumber.c_str());
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_RenderModelName_String, m_sModelNumber.c_str());
		vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_UserIpdMeters_Float, m_flIPD);
		vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_UserHeadToEyeDepthMeters_Float, 0.f);
		vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_DisplayFrequency_Float, m_flDisplayFrequency);
		vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_SecondsFromVsyncToPhotons_Float, m_flSecondsFromVsyncToPhotons);
		vr::VRProperties()->SetUint64Property(m_ulPropertyContainer, vr::Prop_GraphicsAdapterLuid_Uint64, m_nGraphicsAdapterLuid);

		// return a constant that's not 0 (invalid) or 1 (reserved for Oculus)
		vr::VRProperties()->SetUint64Property(m_ulPropertyContainer, vr::Prop_CurrentUniverseId_Uint64, 2);

		// avoid "not fullscreen" warnings from vrmonitor
		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_IsOnDesktop_Bool, false);

		// Manually send VSync events on direct mode. ref:https://github.com/ValveSoftware/virtual_display/issues/1
		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_DriverDirectModeSendsVsyncEvents_Bool, true);

		// Icons can be configured in code or automatically configured by an external file "drivername\resources\driver.vrresources".
		// Icon properties NOT configured in code (post Activate) are then auto-configured by the optional presence of a driver's "drivername\resources\driver.vrresources".
		// In this manner a driver can configure their icons in a flexible data driven fashion by using an external file.
		//
		// The structure of the driver.vrresources file allows a driver to specialize their icons based on their HW.
		// Keys matching the value in "Prop_ModelNumber_String" are considered first, since the driver may have model specific icons.
		// An absence of a matching "Prop_ModelNumber_String" then considers the ETrackedDeviceClass ("HMD", "Controller", "GenericTracker", "TrackingReference")
		// since the driver may have specialized icons based on those device class names.
		//
		// An absence of either then falls back to the "system.vrresources" where generic device class icons are then supplied.
		//
		// Please refer to "bin\drivers\sample\resources\driver.vrresources" which contains this sample configuration.
		//
		// "Alias" is a reserved key and specifies chaining to another json block.
		//
		// In this sample configuration file (overly complex FOR EXAMPLE PURPOSES ONLY)....
		//
		// "Model-v2.0" chains through the alias to "Model-v1.0" which chains through the alias to "Model-v Defaults".
		//
		// Keys NOT found in "Model-v2.0" would then chase through the "Alias" to be resolved in "Model-v1.0" and either resolve their or continue through the alias.
		// Thus "Prop_NamedIconPathDeviceAlertLow_String" in each model's block represent a specialization specific for that "model".
		// Keys in "Model-v Defaults" are an example of mapping to the same states, and here all map to "Prop_NamedIconPathDeviceOff_String".
		//
		bool bSetupIconUsingExternalResourceFile = true;
		if (!bSetupIconUsingExternalResourceFile)
		{
			// Setup properties directly in code.
			// Path values are of the form {drivername}\icons\some_icon_filename.png
			vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceOff_String, "{virtual_display}/icons/headset_sample_status_off.png");
			vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceSearching_String, "{virtual_display}/icons/headset_sample_status_searching.gif");
			vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceSearchingAlert_String, "{virtual_display}/icons/headset_sample_status_searching_alert.gif");
			vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceReady_String, "{virtual_display}/icons/headset_sample_status_ready.png");
			vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceReadyAlert_String, "{virtual_display}/icons/headset_sample_status_ready_alert.png");
			vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceNotReady_String, "{virtual_display}/icons/headset_sample_status_error.png");
			vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceStandby_String, "{virtual_display}/icons/headset_sample_status_standby.png");
			vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceAlertLow_String, "{virtual_display}/icons/headset_sample_status_ready_low.png");
		}

		return vr::VRInitError_None;
	}

	virtual void Deactivate()
	{
		Log("CRemoteHmd Deactivate");
		m_unObjectId = vr::k_unTrackedDeviceIndexInvalid;
	}

	virtual void EnterStandby()
	{
	}

	void *GetComponent(const char *pchComponentNameAndVersion)
	{
		Log("GetComponent %s", pchComponentNameAndVersion);
		if (!_stricmp(pchComponentNameAndVersion, vr::IVRDisplayComponent_Version))
		{
			return (vr::IVRDisplayComponent*)this;
		}
		if (!_stricmp(pchComponentNameAndVersion, vr::IVRDriverDirectModeComponent_Version))
		{
			return static_cast< vr::IVRDriverDirectModeComponent * >(this);
		}

		// override this to add a component to a driver
		return NULL;
	}

	virtual void PowerOff()
	{
	}

	/** debug request from a client */
	virtual void DebugRequest(const char *pchRequest, char *pchResponseBuffer, uint32_t unResponseBufferSize)
	{
		if (unResponseBufferSize >= 1)
			pchResponseBuffer[0] = 0;
	}

	virtual void GetWindowBounds(int32_t *pnX, int32_t *pnY, uint32_t *pnWidth, uint32_t *pnHeight)
	{
		Log("GetWindowBounds %dx%d - %dx%d", 0, 0, m_nRenderWidth, m_nRenderHeight);
		*pnX = 0;
		*pnY = 0;
		*pnWidth = m_nRenderWidth;
		*pnHeight = m_nRenderHeight;
	}

	virtual bool IsDisplayOnDesktop()
	{
		return false;
	}

	virtual bool IsDisplayRealDisplay()
	{
		return false;
	}

	virtual void GetRecommendedRenderTargetSize(uint32_t *pnWidth, uint32_t *pnHeight)
	{
		*pnWidth = m_nRenderWidth / 2;
		*pnHeight = m_nRenderHeight;
		Log("GetRecommendedRenderTargetSize %dx%d", *pnWidth, *pnHeight);
	}

	virtual void GetEyeOutputViewport(vr::EVREye eEye, uint32_t *pnX, uint32_t *pnY, uint32_t *pnWidth, uint32_t *pnHeight)
	{
		*pnY = 0;
		*pnWidth = m_nRenderWidth / 2;
		*pnHeight = m_nRenderHeight;

		if (eEye == vr::Eye_Left)
		{
			*pnX = 0;
		}
		else
		{
			*pnX = m_nRenderWidth / 2;
		}
		Log("GetEyeOutputViewport %d %dx%d %dx%d", eEye, *pnX, *pnY, *pnWidth, *pnHeight);
	}

	virtual void GetProjectionRaw(vr::EVREye eEye, float *pfLeft, float *pfRight, float *pfTop, float *pfBottom)
	{
		*pfLeft = -1.0;
		*pfRight = 1.0;
		*pfTop = -1.0;
		*pfBottom = 1.0;
		
		Log("GetProjectionRaw %d", eEye);
	}

	virtual vr::DistortionCoordinates_t ComputeDistortion(vr::EVREye eEye, float fU, float fV)
	{
		vr::DistortionCoordinates_t coordinates;
		coordinates.rfBlue[0] = fU;
		coordinates.rfBlue[1] = fV;
		coordinates.rfGreen[0] = fU;
		coordinates.rfGreen[1] = fV;
		coordinates.rfRed[0] = fU;
		coordinates.rfRed[1] = fV;
		return coordinates;
	}

	// ITrackedDeviceServerDriver


	virtual vr::DriverPose_t GetPose()
	{
		vr::DriverPose_t pose = { 0 };
		pose.poseIsValid = true;
		pose.result = vr::TrackingResult_Running_OK;
		pose.deviceIsConnected = true;
		//pose.shouldApplyHeadModel = true;
		//pose.willDriftInYaw = true;

		pose.qWorldFromDriverRotation = HmdQuaternion_Init(1, 0, 0, 0);
		pose.qDriverFromHeadRotation = HmdQuaternion_Init(1, 0, 0, 0);
		pose.qRotation = HmdQuaternion_Init(1, 0, 0, 0);

		if (m_Listener->HasValidTrackingInfo()) {
			Listener::TrackingInfo info;
			m_Listener->GetTrackingInfo(info);
			uint64_t trackingDelay = GetTimestampUs() - m_Listener->clientToServerTime(info.clientTime);

			Log("Tracking elapsed:%lld us FrameIndex=%lld quot:%f,%f,%f,%f\nposition:%f,%f,%f\nView[0]:\n%sProj[0]:\n%sView[1]:\n%sProj[1]:\n%s",
				trackingDelay,
				info.FrameIndex,
				info.HeadPose_Pose_Orientation.x,
				info.HeadPose_Pose_Orientation.y,
				info.HeadPose_Pose_Orientation.z,
				info.HeadPose_Pose_Orientation.w,
				info.HeadPose_Pose_Position.x,
				info.HeadPose_Pose_Position.y,
				info.HeadPose_Pose_Position.z,
				DumpMatrix(info.Eye[0].ViewMatrix.M).c_str(),
				DumpMatrix(info.Eye[0].ProjectionMatrix.M).c_str(),
				DumpMatrix(info.Eye[1].ViewMatrix.M).c_str(),
				DumpMatrix(info.Eye[1].ProjectionMatrix.M).c_str()
			);

			pose.qRotation.x = info.HeadPose_Pose_Orientation.x;
			pose.qRotation.y = info.HeadPose_Pose_Orientation.y;
			pose.qRotation.z = info.HeadPose_Pose_Orientation.z;
			pose.qRotation.w = info.HeadPose_Pose_Orientation.w;

			pose.vecPosition[0] = info.HeadPose_Pose_Position.x;
			pose.vecPosition[1] = info.HeadPose_Pose_Position.y;
			pose.vecPosition[2] = info.HeadPose_Pose_Position.z;

			// To disable time warp (or pose prediction), we dont set (set to zero) velocity and acceleration.
			/*
			pose.vecVelocity[0] = info.HeadPose_LinearVelocity.x;
			pose.vecVelocity[1] = info.HeadPose_LinearVelocity.y;
			pose.vecVelocity[2] = info.HeadPose_LinearVelocity.z;

			pose.vecAcceleration[0] = info.HeadPose_LinearAcceleration.x;
			pose.vecAcceleration[1] = info.HeadPose_LinearAcceleration.y;
			pose.vecAcceleration[2] = info.HeadPose_LinearAcceleration.z;

			pose.vecAngularVelocity[0] = info.HeadPose_AngularVelocity.x;
			pose.vecAngularVelocity[1] = info.HeadPose_AngularVelocity.y;
			pose.vecAngularVelocity[2] = info.HeadPose_AngularVelocity.z;

			pose.vecAngularAcceleration[0] = info.HeadPose_AngularAcceleration.x;
			pose.vecAngularAcceleration[1] = info.HeadPose_AngularAcceleration.y;
			pose.vecAngularAcceleration[2] = info.HeadPose_AngularAcceleration.z;*/

			pose.poseTimeOffset = 0;

			m_LastReferencedFrameIndex = info.FrameIndex;
			m_LastReferencedClientTime = info.clientTime;

			// Put pose history buffer
			m_poseMutex.Wait(INFINITE);
			if (m_poseBuffer.size() != 0) {
				m_poseBuffer.push_back(info);
			}
			else {
				if (m_poseBuffer.back().FrameIndex != info.FrameIndex) {
					// New track info
					m_poseBuffer.push_back(info);
				}
			}
			if (m_poseBuffer.size() > 10) {
				m_poseBuffer.pop_front();
			}
			m_poseMutex.Release();
		}

		return pose;
	}


	void RunFrame()
	{
		// In a real driver, this should happen from some pose tracking thread.
		// The RunFrame interval is unspecified and can be very irregular if some other
		// driver blocks it for some periodic task.
		if (m_unObjectId != vr::k_unTrackedDeviceIndexInvalid)
		{
			//Log("RunFrame");
			//vr::VRServerDriverHost()->TrackedDevicePoseUpdated(m_unObjectId, GetPose(), sizeof(vr::DriverPose_t));
		}
	}

	std::string GetSerialNumber() const { return m_sSerialNumber; }

private:
	vr::TrackedDeviceIndex_t m_unObjectId;
	vr::PropertyContainerHandle_t m_ulPropertyContainer;

	std::string m_sSerialNumber;
	std::string m_sModelNumber;

	int32_t m_nRenderWidth;
	int32_t m_nRenderHeight;
	float m_flSecondsFromVsyncToPhotons;
	float m_flDisplayFrequency;
	float m_flIPD;

	std::string m_EncoderOptions;

	uint64_t m_LastReferencedFrameIndex;
	uint64_t m_LastReferencedClientTime;

	IPCMutex m_poseMutex;
	std::list<Listener::TrackingInfo> m_poseBuffer;

public:
	bool IsValid() const
	{
		return m_pEncoder != NULL;
	}

	void CommandCallback(std::string commandName, std::string args)
	{
		if (commandName == "Capture") {
			m_captureDDSTrigger = true;
		}
		else if (commandName == "EnableDriverTestMode") {
			g_DriverTestMode = strtoull(args.c_str(), NULL, 0);
		}
	}

	void OnPoseUpdated() {
		if (m_unObjectId != vr::k_unTrackedDeviceIndexInvalid)
		{
			Log("OnPoseUpdated");
			vr::VRServerDriverHost()->TrackedDevicePoseUpdated(m_unObjectId, GetPose(), sizeof(vr::DriverPose_t));

			Log("Generate VSync Event by OnPoseUpdated");
			m_VSyncThread->InsertVsync();
			//m_VSyncThread->InsertVsync();
		}
	}

private:
	uint64_t m_nGraphicsAdapterLuid;
	uint32_t m_nVsyncCounter;

	CD3DRender *m_pD3DRender;
	ID3D11Texture2D *m_pFlushTexture;
	CNvEncoder *m_pRemoteDevice;
	CEncoder *m_pEncoder;
	Listener *m_Listener;
	VSyncThread *m_VSyncThread;
public:
	// -----------------------------------
	// Direct mode methods
	// -----------------------------------

	/** Specific to Oculus compositor support, textures supplied must be created using this method. */
	virtual void CreateSwapTextureSet(uint32_t unPid, uint32_t unFormat, uint32_t unWidth, uint32_t unHeight, vr::SharedTextureHandle_t(*pSharedTextureHandles)[3]) {
		Log("CreateSwapTextureSet pid=%d Format=%d %dx%d", unPid, unFormat, unWidth, unHeight);

		//HRESULT hr = D3D11CreateDevice(pAdapter, D3D_DRIVER_TYPE_HARDWARE, NULL, creationFlags, NULL, 0, D3D11_SDK_VERSION, &pDevice, &eFeatureLevel, &pContext);

		D3D11_TEXTURE2D_DESC SharedTextureDesc = {};
		SharedTextureDesc.ArraySize = 1;
		SharedTextureDesc.MipLevels = 1;
		SharedTextureDesc.SampleDesc.Count = 1;
		SharedTextureDesc.SampleDesc.Quality = 0;
		SharedTextureDesc.Usage = D3D11_USAGE_DEFAULT;
		SharedTextureDesc.Format = (DXGI_FORMAT)unFormat;

		// Some(or all?) applications request larger texture than we specified in GetRecommendedRenderTargetSize.
		// But, we must create textures in requested size to prevent cropped output. And then we must shrink texture to H.264 movie size.
		SharedTextureDesc.Width = unWidth;
		SharedTextureDesc.Height = unHeight;

		SharedTextureDesc.BindFlags = D3D11_BIND_SHADER_RESOURCE | D3D11_BIND_RENDER_TARGET;
		//SharedTextureDesc.MiscFlags = D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX | D3D11_RESOURCE_MISC_SHARED_NTHANDLE;
		SharedTextureDesc.MiscFlags = D3D11_RESOURCE_MISC_SHARED;
		
		ProcessResource *processResource = new ProcessResource();
		processResource->pid = unPid;

		for (int i = 0; i < 3; i++) {
			HRESULT hr = m_pD3DRender->GetDevice()->CreateTexture2D(&SharedTextureDesc, NULL, &processResource->textures[i]);
			//Log("texture%d %p res:%d %s", i, texture[i], hr, GetDxErrorStr(hr).c_str());

			IDXGIResource* pResource;
			hr = processResource->textures[i]->QueryInterface(__uuidof(IDXGIResource), (void**)&pResource);
			//Log("QueryInterface %p res:%d %s", pResource, hr, GetDxErrorStr(hr).c_str());

			hr = pResource->GetSharedHandle(&processResource->sharedHandles[i]);
			//Log("GetSharedHandle %p res:%d %s", processResource->sharedHandles[i], hr, GetDxErrorStr(hr).c_str());

			m_handleMap.insert(std::make_pair(processResource->sharedHandles[i], std::make_pair(processResource, i)));

			(*pSharedTextureHandles)[i] = (vr::SharedTextureHandle_t)processResource->sharedHandles[i];

			pResource->Release();

			Log("Created Texture %d %p", i, processResource->sharedHandles[i]);
		}
		//m_processMap.insert(std::pair<uint32_t, ProcessResource *>(unPid, processResource));
	}

	/** Used to textures created using CreateSwapTextureSet.  Only one of the set's handles needs to be used to destroy the entire set. */
	virtual void DestroySwapTextureSet(vr::SharedTextureHandle_t sharedTextureHandle) {
		Log("DestroySwapTextureSet");

		auto it = m_handleMap.find((HANDLE)sharedTextureHandle);
		if (it != m_handleMap.end()) {
			// Release all reference (a bit forcible)
			it->second.first->textures[it->second.second].Reset();
		}
		else {
			Log("Requested to destroy not managing texture. handle:%p", sharedTextureHandle);
		}
	}

	/** Used to purge all texture sets for a given process. */
	virtual void DestroyAllSwapTextureSets(uint32_t unPid) {
		Log("DestroyAllSwapTextureSets pid=%d", unPid);

		for (auto it = m_handleMap.begin(); it != m_handleMap.end();) {
			if (it->second.first->pid == unPid) {
				if (it->second.second == 0) {
					delete it->second.first;
				}
				m_handleMap.erase(it++);
			}
			else {
				++it;
			}
		}
	}

	/** After Present returns, calls this to get the next index to use for rendering. */
	virtual void GetNextSwapTextureSetIndex(vr::SharedTextureHandle_t sharedTextureHandles[2], uint32_t(*pIndices)[2]) {
		Log("GetNextSwapTextureSetIndex %p %p %d %d", sharedTextureHandles[0], sharedTextureHandles[1], (*pIndices)[0], (*pIndices)[1]);
		(*pIndices)[0]++;
		(*pIndices)[0] %= 3;
		(*pIndices)[1]++;
		(*pIndices)[1] %= 3;
	}

	/** Call once per layer to draw for this frame.  One shared texture handle per eye.  Textures must be created
	* using CreateSwapTextureSet and should be alternated per frame.  Call Present once all layers have been submitted. */
	virtual void SubmitLayer(vr::SharedTextureHandle_t sharedTextureHandles[2], const vr::VRTextureBounds_t(&bounds)[2], const vr::HmdMatrix34_t *pPose) {
		Log("SubmitLayer Handle0=%p Handle1=%p %f-%f,%f-%f %f-%f,%f-%f  \n%f,%f,%f,%f\n%f,%f,%f,%f\n%f,%f,%f,%f", sharedTextureHandles[0], sharedTextureHandles[1]
			, bounds[0].uMin, bounds[0].uMax, bounds[0].vMin, bounds[0].vMax
			, bounds[1].uMin, bounds[1].uMax, bounds[1].vMin, bounds[1].vMax
			, pPose->m[0][0], pPose->m[0][1], pPose->m[0][2], pPose->m[0][3]
			, pPose->m[1][0], pPose->m[1][1], pPose->m[1][2], pPose->m[1][3]
			, pPose->m[2][0], pPose->m[2][1], pPose->m[2][2], pPose->m[2][3]
		);
		// 3x3 rotation matrix
		//pPose->m[0][0], pPose->m[0][1], pPose->m[0][2],
		//pPose->m[1][0], pPose->m[1][1], pPose->m[1][2], 
		//pPose->m[2][0], pPose->m[2][1], pPose->m[2][2], 
		// position
		// x = pPose->m[0][3], y = pPose->m[1][3], z = pPose->m[2][3]
		m_framePose = *pPose;

		if (m_submitLayer == 0) {
			m_poseMutex.Wait(INFINITE);
			float diff = 100000;
			int index = 0;
			int minIndex = 0;
			auto minIt = m_poseBuffer.begin();
			for (auto it = m_poseBuffer.begin(); it != m_poseBuffer.end(); it++, index++) {
				float distance = 0;
				// rotation matrix composes parts of ViewMatrix
				for (int i = 0; i < 3; i++) {
					for (int j = 0; j < 3; j++) {
						distance += pow(it->Eye[0].ViewMatrix.M[j * 3 + i] - pPose->m[i][j], 2);
					}
				}
				if (diff > distance) {
					minIndex = index;
					minIt = it;
				}
			}
			if (minIt != m_poseBuffer.end()) {
				// found the frameIndex
				m_prevSubmitFrameIndex = m_submitFrameIndex;
				m_prevSubmitClientTime = m_submitClientTime;
				m_submitFrameIndex = minIt->FrameIndex;
				m_submitClientTime = minIt->clientTime;

				m_prevFramePoseRotation = m_framePoseRotation;
				m_framePoseRotation.x = minIt->HeadPose_Pose_Orientation.x;
				m_framePoseRotation.y = minIt->HeadPose_Pose_Orientation.y;
				m_framePoseRotation.z = minIt->HeadPose_Pose_Orientation.z;
				m_framePoseRotation.w = minIt->HeadPose_Pose_Orientation.w;

				Log("Frame pose found. m_prevSubmitFrameIndex=%llu m_submitFrameIndex=%llu", m_prevSubmitFrameIndex, m_submitFrameIndex);
			}
			else {
				m_submitFrameIndex = 0;
				m_submitClientTime = 0;
				m_framePoseRotation = HmdQuaternion_Init(0.0, 0.0, 0.0, 0.0);
			}
			m_poseMutex.Release();
		}
		/*Listener::TrackingInfo info;
		m_Listener->GetTrackingInfo(info);
		m_submitFrameIndex = info.FrameIndex;
		m_submitClientTime = info.clientTime;
		m_framePoseRotation.x = info.HeadPose_Pose_Orientation.x;
		*/
		if (m_submitLayer < MAX_LAYERS) {
			m_submitTextures[m_submitLayer][0] = sharedTextureHandles[0];
			m_submitTextures[m_submitLayer][1] = sharedTextureHandles[1];
			m_submitBounds[m_submitLayer][0] = bounds[0];
			m_submitBounds[m_submitLayer][1] = bounds[1];
			m_submitLayer++;
		}
		else {
			Log("Too many layers submitted!");
		}

		//CopyTexture();
	}

	/** Submits queued layers for display. */
	virtual void Present(vr::SharedTextureHandle_t syncTexture) {
		Log("Present syncTexture=%p m_prevSubmitFrameIndex=%llu m_submitFrameIndex=%llu", syncTexture, m_prevSubmitFrameIndex, m_submitFrameIndex);

		uint32_t layerCount = m_submitLayer;
		m_submitLayer = 0;

		if (m_submitFrameIndex != m_LastReferencedFrameIndex) {
			// Discard old frames
			Log("Discarding old frame: m_submitFrameIndex=%llu m_LastReferencedFrameIndex=%llu", m_submitFrameIndex, m_LastReferencedFrameIndex);
			return;
		}


		ID3D11Texture2D *pSyncTexture = m_pD3DRender->GetSharedTexture((HANDLE)syncTexture);
		if (!pSyncTexture)
		{
			Log("[VDispDvr] SyncTexture is NULL!");
			return;
		}

		// Access to shared texture must be wrapped in AcquireSync/ReleaseSync
		// to ensure the compositor has finished rendering to it before it gets used.
		// This enforces scheduling of work on the gpu between processes.
		IDXGIKeyedMutex *pKeyedMutex = NULL;
		if (SUCCEEDED(pSyncTexture->QueryInterface(__uuidof(IDXGIKeyedMutex), (void **)&pKeyedMutex)))
		{
			Log("[VDispDvr] Wait for SyncTexture Mutex.");
			// TODO: Reasonable timeout and timeout handling
			HRESULT hr = pKeyedMutex->AcquireSync(0, 10);
			if (hr != S_OK)
			{
				Log("[VDispDvr] ACQUIRESYNC FAILED!!! hr=%d %p %s", hr, hr, GetDxErrorStr(hr).c_str());
				pKeyedMutex->Release();
				return;
			}
		}

		Log("[VDispDvr] Mutex Acquired.");

		CopyTexture(layerCount);

		if (pKeyedMutex)
		{
			pKeyedMutex->ReleaseSync(0);
			pKeyedMutex->Release();
		}

		Log("[VDispDvr] Mutex Released.");
		m_pEncoder->NewFrameReady();
	}

	void CopyTexture(uint32_t layerCount) {

		uint64_t presentationTime = GetTimestampUs();

		ID3D11Texture2D *pTexture[MAX_LAYERS][2];
		ComPtr<ID3D11Texture2D> Texture[MAX_LAYERS][2];

		for (uint32_t i = 0; i < layerCount; i++) {
			// Find left eye texture.
			auto it = m_handleMap.find((HANDLE)m_submitTextures[i][0]);
			if (it == m_handleMap.end()) {
				// Ignore this layer.
				Log("Submitted texture is not found on HandleMap. eye=right layer=%d/%d Texture Handle=%p", i, layerCount, (HANDLE)m_submitTextures[i][0]);
			}
			else {
				Texture[i][0] = it->second.first->textures[it->second.second];
				D3D11_TEXTURE2D_DESC desc;
				Texture[i][0]->GetDesc(&desc);

				Log("CopyTexture: layer=%d/%d pid=%d Texture Size=%dx%d Format=%d", i, layerCount, it->second.first->pid, desc.Width, desc.Height, desc.Format);

				// Find right eye texture.
				it = m_handleMap.find((HANDLE)m_submitTextures[i][1]);
				if (it == m_handleMap.end()) {
					// Ignore this layer
					Log("Submitted texture is not found on HandleMap. eye=left layer=%d/%d Texture Handle=%p", i, layerCount, (HANDLE)m_submitTextures[i][1]);
					Texture[i][0].Reset();
				}
				else {
					Texture[i][1] = it->second.first->textures[it->second.second];
				}
			}

			pTexture[i][0] = Texture[i][0].Get();
			pTexture[i][1] = Texture[i][1].Get();
		}

		// This can go away, but is useful to see it as a separate packet on the gpu in traces.
		m_pD3DRender->GetContext()->Flush();

		Log("Waiting for finish of previous encode.");

		if (m_captureDDSTrigger) {
			wchar_t buf[1000];

			for (uint32_t i = 0; i < layerCount; i++) {
				Log("Writing Debug DDS. m_LastReferencedFrameIndex=%llu layer=%d/%d", m_LastReferencedFrameIndex, i, layerCount);
				_snwprintf_s(buf, sizeof(buf), L"%hs\\debug-%llu-%d-%d.dds", g_DebugOutputDir.c_str(), m_LastReferencedFrameIndex, i, layerCount);
				HRESULT hr = DirectX::SaveDDSTextureToFile(m_pD3DRender->GetContext(), pTexture[i][0], buf);
				Log("Writing Debug DDS: End hr=%p %s", hr, GetDxErrorStr(hr).c_str());
			}
			m_Listener->SendCommandResponse("Capture OK");
			m_captureDDSTrigger = false;
		}

		// Wait for the encoder to be ready.  This is important because the encoder thread
		// blocks on transmit which uses our shared d3d context (which is not thread safe).
		m_pEncoder->WaitForEncode();

		// Copy entire texture to staging so we can read the pixels to send to remote device.
		Log("FrameIndex diff LastRef: %llu render:%llu  diff:%llu", m_LastReferencedFrameIndex, m_submitFrameIndex, m_LastReferencedFrameIndex - m_submitFrameIndex);

		Listener::TrackingInfo info;
		m_Listener->GetTrackingInfo(info);

		char buf[2000];
		snprintf(buf, sizeof(buf), "%llu\n%f\n%f", m_prevSubmitFrameIndex, m_prevFramePoseRotation.x, info.HeadPose_Pose_Orientation.x);
		m_pEncoder->CopyToStaging(pTexture, m_submitBounds, layerCount, presentationTime, m_prevSubmitFrameIndex, m_prevSubmitClientTime, std::string(buf));

		//m_pEncoder->CopyToStaging(pTexture, m_submitBounds, presentationTime, m_submitFrameIndex, m_submitClientTime, std::string(buf));

		m_pD3DRender->GetContext()->Flush();
	}

private:
	// Resource for each process
	struct ProcessResource {
		ComPtr<ID3D11Texture2D> textures[3];
		HANDLE sharedHandles[3];
		uint32_t pid;
	};
	//std::unordered_multimap<uint32_t, ProcessResource *> m_processMap;
	std::map<HANDLE, std::pair<ProcessResource *, int> > m_handleMap;

	static const int MAX_LAYERS = 10;
	int m_submitLayer;
	vr::SharedTextureHandle_t m_submitTextures[MAX_LAYERS][2];
	vr::VRTextureBounds_t m_submitBounds[MAX_LAYERS][2];
	vr::HmdMatrix34_t m_framePose;
	vr::HmdQuaternion_t m_prevFramePoseRotation;
	vr::HmdQuaternion_t m_framePoseRotation;
	uint64_t m_submitFrameIndex;
	uint64_t m_submitClientTime;
	uint64_t m_prevSubmitFrameIndex;
	uint64_t m_prevSubmitClientTime;

	bool m_captureDDSTrigger;
};

//-----------------------------------------------------------------------------
// Purpose: Server interface implementation.
//-----------------------------------------------------------------------------
class CServerDriver_DisplayRedirect : public vr::IServerTrackedDeviceProvider
{
public:
	CServerDriver_DisplayRedirect()
		: m_pRemoteHmd( NULL )
	{}

	virtual vr::EVRInitError Init( vr::IVRDriverContext *pContext ) override;
	virtual void Cleanup() override;
	virtual const char * const *GetInterfaceVersions() override
		{ return vr::k_InterfaceVersions;  }
	virtual const char *GetTrackedDeviceDriverVersion()
		{ return vr::ITrackedDeviceServerDriver_Version; }
	virtual void RunFrame();
	virtual bool ShouldBlockStandbyMode() override { return false; }
	virtual void EnterStandby() override {}
	virtual void LeaveStandby() override {}

private:
	CRemoteHmd *m_pRemoteHmd;
};

vr::EVRInitError CServerDriver_DisplayRedirect::Init( vr::IVRDriverContext *pContext )
{
	VR_INIT_SERVER_DRIVER_CONTEXT( pContext );

	m_pRemoteHmd = new CRemoteHmd();

	if (m_pRemoteHmd->IsValid() )
	{
		bool ret;
		ret = vr::VRServerDriverHost()->TrackedDeviceAdded(
			m_pRemoteHmd->GetSerialNumber().c_str(),
			vr::TrackedDeviceClass_HMD,
			//vr::TrackedDeviceClass_DisplayRedirect,
			m_pRemoteHmd);
		Log("TrackedDeviceAdded %d %s", ret, m_pRemoteHmd->GetSerialNumber().c_str());
	}

	return vr::VRInitError_None;
}

void CServerDriver_DisplayRedirect::Cleanup()
{
	delete m_pRemoteHmd;
	m_pRemoteHmd = NULL;

	VR_CLEANUP_SERVER_DRIVER_CONTEXT();
}

void CServerDriver_DisplayRedirect::RunFrame()
{
	if (m_pRemoteHmd)
	{
		m_pRemoteHmd->RunFrame();
	}
}

CServerDriver_DisplayRedirect g_serverDriverDisplayRedirect;

//-----------------------------------------------------------------------------
// Purpose: Entry point for vrserver when loading drivers.
//-----------------------------------------------------------------------------
extern "C" __declspec( dllexport )
void *HmdDriverFactory( const char *pInterfaceName, int *pReturnCode )
{
	Log("HmdDriverFactory %s (%s)", pInterfaceName, vr::IServerTrackedDeviceProvider_Version);
	if ( 0 == strcmp( vr::IServerTrackedDeviceProvider_Version, pInterfaceName ) )
	{
		Log("HmdDriverFactory server return");
		return &g_serverDriverDisplayRedirect;
	}

	if( pReturnCode )
		*pReturnCode = vr::VRInitError_Init_InterfaceNotFound;

	return NULL;
}

BOOL WINAPI DllMain(HINSTANCE hInstance, DWORD dwReason, LPVOID lpReserved)
{
	switch (dwReason) {
	case DLL_PROCESS_ATTACH:
		g_hInstance = hInstance;
	}

	return TRUE;
}