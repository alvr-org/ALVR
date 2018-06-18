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

#include <winsock2.h>
#include <d3d11.h>
#include <wrl.h>
#include <map>
#include <d3d11_1.h>
#include <ScreenGrab.h>
#include <wincodec.h>
#include <wincodecsdk.h>

#include "NvEncoderD3D11.h"
#include "NvEncoderCuda.h"
#include "Logger.h"
#include "NvCodecUtils.h"
#include "nvencoderclioptions.h"
#include "Listener.h"
#include "Utils.h"
#include "FrameRender.h"
#include "Settings.h"
#include "RemoteController.h"
#include "packet_types.h"
#include "resource.h"
#include "Tracking.h"
#include "CudaConverter.h"	   
#include "RGBToNV12.h" 
#include "AudioCapture.h"

HINSTANCE g_hInstance;

uint64_t g_DriverTestMode = 0;

namespace
{
	using Microsoft::WRL::ComPtr;
	
	void SaveDebugOutput(std::shared_ptr<CD3DRender> m_pD3DRender, std::vector<std::vector<uint8_t>> &vPacket, ID3D11Texture2D *texture, uint64_t frameIndex) {
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
			snprintf(filename, sizeof(filename), "%s\\%llu.h264", Settings::Instance().m_DebugOutputDir.c_str(), frameIndex);
			_snwprintf_s(filename2, sizeof(filename2), L"%hs\\%llu.dds", Settings::Instance().m_DebugOutputDir.c_str(), frameIndex);
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


	class CNvEncoder
	{
	public:
		CNvEncoder(std::shared_ptr<CD3DRender> pD3DRender
			, std::shared_ptr<Listener> listener, bool useNV12)
			: m_pD3DRender(pD3DRender)
			, m_nFrame(0)
			, m_Listener(listener)
			, m_insertIDR(false)
			, m_useNV12(useNV12)
		{
		}

		~CNvEncoder()
		{}

		bool Initialize()
		{
			NvEncoderInitParam EncodeCLIOptions(Settings::Instance().m_EncoderOptions.c_str());

			//m_pD3DRender->GetDevice()->CreateDeferredContext(0, &m_DeferredContext);
			
			//
			// Initialize Encoder
			//

			NV_ENC_BUFFER_FORMAT format = NV_ENC_BUFFER_FORMAT_ABGR;
			if (m_useNV12) {
				format = NV_ENC_BUFFER_FORMAT_NV12;
			}

			Log("Initializing CNvEncoder. Width=%d Height=%d Format=%d (useNV12:%d)", Settings::Instance().m_renderWidth, Settings::Instance().m_renderHeight
				, format, m_useNV12);

			if (m_useNV12) {
				try {
					m_Converter = std::make_shared<CudaConverter>(m_pD3DRender->GetDevice(), Settings::Instance().m_renderWidth, Settings::Instance().m_renderHeight);
				}
				catch (Exception e) {
					FatalLog("Exception:%s", e.what());
					return false;
				}

				m_NvNecoder = std::make_shared<NvEncoderCuda>(m_Converter->GetContext(), Settings::Instance().m_renderWidth, Settings::Instance().m_renderHeight, format, 0);
			}
			else {
				m_NvNecoder = std::make_shared<NvEncoderD3D11>(m_pD3DRender->GetDevice(), Settings::Instance().m_renderWidth, Settings::Instance().m_renderHeight, format, 0);
			}

			NV_ENC_INITIALIZE_PARAMS initializeParams = { NV_ENC_INITIALIZE_PARAMS_VER };
			NV_ENC_CONFIG encodeConfig = { NV_ENC_CONFIG_VER };

			initializeParams.encodeConfig = &encodeConfig;
			m_NvNecoder->CreateDefaultEncoderParams(&initializeParams, EncodeCLIOptions.GetEncodeGUID(), EncodeCLIOptions.GetPresetGUID());

			initializeParams.encodeConfig->encodeCodecConfig.h264Config.repeatSPSPPS = 1;

			EncodeCLIOptions.SetInitParams(&initializeParams, format);

			std::string parameterDesc = EncodeCLIOptions.FullParamToString(&initializeParams);
			Log("NvEnc Encoder Parameters:\n%s", parameterDesc.c_str());

			try {
				m_NvNecoder->CreateEncoder(&initializeParams);
			}
			catch (NVENCException e) {
				FatalLog("NvEnc CreateEncoder failed. Code=%d %s", e.getErrorCode(), e.what());
				return false;
			}

			//
			// Initialize debug video output
			//

			if (Settings::Instance().m_DebugCaptureOutput) {
				fpOut = std::ofstream(Settings::Instance().GetVideoOutput(), std::ios::out | std::ios::binary);
				if (!fpOut)
				{
					Log("unable to open output file %s", Settings::Instance().GetVideoOutput().c_str());
				}
			}

			return true;
		}

		void Shutdown()
		{
			std::vector<std::vector<uint8_t>> vPacket;
			m_NvNecoder->EndEncode(vPacket);
			for (std::vector<uint8_t> &packet : vPacket)
			{
				if (fpOut) {
					fpOut.write(reinterpret_cast<char*>(packet.data()), packet.size());
				}
				m_Listener->SendVideo(packet.data(), (int)packet.size(), GetTimestampUs(), 0);
			}

			m_NvNecoder->DestroyEncoder();
			m_NvNecoder.reset();

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

			const NvEncInputFrame* encoderInputFrame = m_NvNecoder->GetNextInputFrame();

			if (m_useNV12)
			{
				try {
					Log("ConvertRGBToNV12 start");
					m_Converter->Convert(pTexture, encoderInputFrame);
					Log("ConvertRGBToNV12 end");
				}
				catch (NVENCException e) {
					FatalLog("Exception:%s", e.what());
					return;
				}
			}
			else {
				ID3D11Texture2D *pInputTexture = reinterpret_cast<ID3D11Texture2D*>(encoderInputFrame->inputPtr);
				Log("CopyResource start");
				m_pD3DRender->GetContext()->CopyResource(pInputTexture, pTexture);
				//m_DeferredContext->CopyResource(pTexBgra, pTexture);
				Log("CopyResource end");
			}

			NV_ENC_PIC_PARAMS picParams = {};
			if (m_insertIDR) {
				m_insertIDR = false;
				picParams.encodePicFlags = NV_ENC_PIC_FLAG_FORCEIDR;
			}
			m_NvNecoder->EncodeFrame(vPacket, &picParams);

			Log("Tracking info delay: %lld us FrameIndex=%llu", GetTimestampUs() - m_Listener->clientToServerTime(clientTime), frameIndex);
			Log("Encoding delay: %lld us FrameIndex=%llu", GetTimestampUs() - presentationTime, frameIndex);

			m_nFrame += (int)vPacket.size();
			for (std::vector<uint8_t> &packet : vPacket)
			{
				if (fpOut) {
					fpOut.write(reinterpret_cast<char*>(packet.data()), packet.size());
				}
				if (m_Listener) {
					m_Listener->SendVideo(packet.data(), (int)packet.size(), presentationTime, frameIndex);
				}
			}

			if (Settings::Instance().m_DebugFrameOutput) {
				if (!m_useNV12) {
					SaveDebugOutput(m_pD3DRender, vPacket, reinterpret_cast<ID3D11Texture2D*>(encoderInputFrame->inputPtr), frameIndex2);
				}
			}

			Log("[VDispDvr] Transmit(end) (frame %d %d) FrameIndex=%llu", vPacket.size(), m_nFrame, frameIndex);
		}

		void InsertIDR()
		{
			m_insertIDR = true;
		}

	private:
		std::ofstream fpOut;
		std::shared_ptr<NvEncoder> m_NvNecoder;

		std::shared_ptr<CD3DRender> m_pD3DRender;
		int m_nFrame;

		std::shared_ptr<Listener> m_Listener;

		bool m_insertIDR;

		const bool m_useNV12;
		std::shared_ptr<CudaConverter> m_Converter;
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
		CEncoder( std::shared_ptr<CD3DRender> pD3DRender, std::shared_ptr<CNvEncoder> pRemoteDevice )
			: m_pRemoteDevice( pRemoteDevice )
			, m_bExiting( false )
			, m_frameIndex(0)
			, m_frameIndex2(0)
			, m_FrameRender(std::make_shared<FrameRender>(pD3DRender))
		{
			m_encodeFinished.Set();
		}

		~CEncoder()
		{
		}

		bool CopyToStaging( ID3D11Texture2D *pTexture[][2], vr::VRTextureBounds_t bounds[][2], int layerCount, bool recentering, uint64_t presentationTime, uint64_t frameIndex, uint64_t clientTime, const std::string& debugText)
		{
			m_presentationTime = presentationTime;
			m_frameIndex = frameIndex;
			m_clientTime = clientTime;
			m_FrameRender->Startup();

			char buf[200];
			snprintf(buf, sizeof(buf), "\nindex2: %llu", m_frameIndex2);

			m_FrameRender->RenderFrame(pTexture, bounds, layerCount, recentering, debugText + buf);
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
			m_FrameRender.reset();
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
		std::shared_ptr<CNvEncoder> m_pRemoteDevice;
		bool m_bExiting;
		uint64_t m_presentationTime;
		uint64_t m_frameIndex;
		uint64_t m_clientTime;

		uint64_t m_frameIndex2;

		std::shared_ptr<FrameRender> m_FrameRender;
	};
}

// VSync Event Thread

class VSyncThread : public CThread
{
public:
	VSyncThread() 
		: m_bExit(false) {}

	// Trigger VSync if elapsed time from previous VSync is larger than 30ms.
	void Run()override {
		static int INTERVAL = 30 * 1000;
		while (!m_bExit) {
			uint64_t current = GetTimestampUs();

			if (current - m_PreviousVsync < INTERVAL - 2000) {
				int sleepTime = (int)((m_PreviousVsync + INTERVAL) - current) / 1000;
				Log("Skip VSync Event. Sleep %llu ms", sleepTime);
				Sleep(sleepTime);
			}
			else {
				Log("Generate VSync Event");
				vr::VRServerDriverHost()->VsyncEvent(0);
				m_PreviousVsync = GetTimestampUs();
			}
		}
	}

	void Shutdown() {
		m_bExit = true;
	}

	void InsertVsync() {
		Log("Insert VSync Event");
		vr::VRServerDriverHost()->VsyncEvent(0);
		m_PreviousVsync = GetTimestampUs();
	}
private:
	bool m_bExit;
	uint64_t m_PreviousVsync;
};

class DisplayComponent : public vr::IVRDisplayComponent
{
public:
	DisplayComponent() {}
	virtual ~DisplayComponent() {}

	virtual void GetWindowBounds(int32_t *pnX, int32_t *pnY, uint32_t *pnWidth, uint32_t *pnHeight) override
	{
		Log("GetWindowBounds %dx%d - %dx%d", 0, 0, Settings::Instance().m_renderWidth, Settings::Instance().m_renderHeight);
		*pnX = 0;
		*pnY = 0;
		*pnWidth = Settings::Instance().m_renderWidth;
		*pnHeight = Settings::Instance().m_renderHeight;
	}

	virtual bool IsDisplayOnDesktop() override
	{
		return false;
	}

	virtual bool IsDisplayRealDisplay() override
	{
		return false;
	}

	virtual void GetRecommendedRenderTargetSize(uint32_t *pnWidth, uint32_t *pnHeight) override
	{
		*pnWidth = Settings::Instance().m_renderWidth / 2;
		*pnHeight = Settings::Instance().m_renderHeight;
		Log("GetRecommendedRenderTargetSize %dx%d", *pnWidth, *pnHeight);
	}

	virtual void GetEyeOutputViewport(vr::EVREye eEye, uint32_t *pnX, uint32_t *pnY, uint32_t *pnWidth, uint32_t *pnHeight) override
	{
		*pnY = 0;
		*pnWidth = Settings::Instance().m_renderWidth / 2;
		*pnHeight = Settings::Instance().m_renderHeight;

		if (eEye == vr::Eye_Left)
		{
			*pnX = 0;
		}
		else
		{
			*pnX = Settings::Instance().m_renderWidth / 2;
		}
		Log("GetEyeOutputViewport %d %dx%d %dx%d", eEye, *pnX, *pnY, *pnWidth, *pnHeight);
	}

	virtual void GetProjectionRaw(vr::EVREye eEye, float *pfLeft, float *pfRight, float *pfTop, float *pfBottom) override
	{
		*pfLeft = -1.0;
		*pfRight = 1.0;
		*pfTop = -1.0;
		*pfBottom = 1.0;

		Log("GetProjectionRaw %d", eEye);
	}

	virtual vr::DistortionCoordinates_t ComputeDistortion(vr::EVREye eEye, float fU, float fV) override
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
};

class DirectModeComponent : public vr::IVRDriverDirectModeComponent
{
public:
	DirectModeComponent(std::shared_ptr<CD3DRender> pD3DRender,
		std::shared_ptr<CEncoder> pEncoder,
		std::shared_ptr<Listener> Listener,
		std::shared_ptr<RecenterManager> recenterManager)
		: m_captureDDSTrigger(false)
		, m_pD3DRender(pD3DRender)
		, m_pEncoder(pEncoder)
		, m_Listener(Listener)
		, m_recenterManager(recenterManager)
		, m_poseMutex(NULL)
		, m_submitLayer(0)
		, m_LastReferencedFrameIndex(0) 
		, m_LastReferencedClientTime(0) {
	}

	bool CommandCallback(std::string commandName, std::string args)
	{
		if (commandName == "Capture") {
			m_captureDDSTrigger = true;
			m_Listener->SendCommandResponse("OK\n");
			return true;
		}
		return false;
	}

	void OnPoseUpdated(TrackingInfo &info) {
		// Put pose history buffer
		TrackingHistoryFrame history;
		history.info = info;

		vr::HmdQuaternion_t recentered = m_recenterManager->GetRecentered(info.HeadPose_Pose_Orientation);
		HmdMatrix_QuatToMat(recentered.w,
			recentered.x,
			recentered.y,
			recentered.z,
			&history.rotationMatrix);

		m_poseMutex.Wait(INFINITE);
		if (m_poseBuffer.size() == 0) {
			m_poseBuffer.push_back(history);
		}
		else {
			if (m_poseBuffer.back().info.FrameIndex != info.FrameIndex) {
				// New track info
				m_poseBuffer.push_back(history);
			}
		}
		if (m_poseBuffer.size() > 10) {
			m_poseBuffer.pop_front();
		}
		m_poseMutex.Release();

		m_LastReferencedFrameIndex = info.FrameIndex;
		m_LastReferencedClientTime = info.clientTime;
	}

	/** Specific to Oculus compositor support, textures supplied must be created using this method. */
	virtual void CreateSwapTextureSet(uint32_t unPid, const SwapTextureSetDesc_t *pSwapTextureSetDesc, vr::SharedTextureHandle_t(*pSharedTextureHandles)[3]) override
	{
		Log("CreateSwapTextureSet pid=%d Format=%d %dx%d SampleCount=%d", unPid, pSwapTextureSetDesc->nFormat
			, pSwapTextureSetDesc->nWidth, pSwapTextureSetDesc->nHeight, pSwapTextureSetDesc->nSampleCount);

		//HRESULT hr = D3D11CreateDevice(pAdapter, D3D_DRIVER_TYPE_HARDWARE, NULL, creationFlags, NULL, 0, D3D11_SDK_VERSION, &pDevice, &eFeatureLevel, &pContext);

		D3D11_TEXTURE2D_DESC SharedTextureDesc = {};
		SharedTextureDesc.ArraySize = 1;
		SharedTextureDesc.MipLevels = 1;
		SharedTextureDesc.SampleDesc.Count = pSwapTextureSetDesc->nSampleCount;
		SharedTextureDesc.SampleDesc.Quality = 0;
		SharedTextureDesc.Usage = D3D11_USAGE_DEFAULT;
		SharedTextureDesc.Format = (DXGI_FORMAT)pSwapTextureSetDesc->nFormat;

		// Some(or all?) applications request larger texture than we specified in GetRecommendedRenderTargetSize.
		// But, we must create textures in requested size to prevent cropped output. And then we must shrink texture to H.264 movie size.
		SharedTextureDesc.Width = pSwapTextureSetDesc->nWidth;
		SharedTextureDesc.Height = pSwapTextureSetDesc->nHeight;

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
	virtual void DestroySwapTextureSet(vr::SharedTextureHandle_t sharedTextureHandle) override
	{
		Log("DestroySwapTextureSet %p", sharedTextureHandle);

		auto it = m_handleMap.find((HANDLE)sharedTextureHandle);
		if (it != m_handleMap.end()) {
			// Release all reference (a bit forcible)
			ProcessResource *p = it->second.first;
			m_handleMap.erase(p->sharedHandles[0]);
			m_handleMap.erase(p->sharedHandles[1]);
			m_handleMap.erase(p->sharedHandles[2]);
			delete p;
		}
		else {
			Log("Requested to destroy not managing texture. handle:%p", sharedTextureHandle);
		}
	}

	/** Used to purge all texture sets for a given process. */
	virtual void DestroyAllSwapTextureSets(uint32_t unPid) override
	{
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
	virtual void GetNextSwapTextureSetIndex(vr::SharedTextureHandle_t sharedTextureHandles[2], uint32_t(*pIndices)[2]) override
	{
		Log("GetNextSwapTextureSetIndex %p %p %d %d", sharedTextureHandles[0], sharedTextureHandles[1], (*pIndices)[0], (*pIndices)[1]);
		(*pIndices)[0]++;
		(*pIndices)[0] %= 3;
		(*pIndices)[1]++;
		(*pIndices)[1] %= 3;
	}

	/** Call once per layer to draw for this frame.  One shared texture handle per eye.  Textures must be created
	* using CreateSwapTextureSet and should be alternated per frame.  Call Present once all layers have been submitted. */
	virtual void SubmitLayer(const SubmitLayerPerEye_t(&perEye)[2], const vr::HmdMatrix34_t *pPose) override
	{
		Log("SubmitLayer Handles=%p,%p DepthHandles=%p,%p %f-%f,%f-%f %f-%f,%f-%f\n%f,%f,%f,%f\n%f,%f,%f,%f\n%f,%f,%f,%f"
			, perEye[0].hTexture, perEye[1].hTexture, perEye[0].hDepthTexture, perEye[1].hDepthTexture
			, perEye[0].bounds.uMin, perEye[0].bounds.uMax, perEye[0].bounds.vMin, perEye[0].bounds.vMax
			, perEye[1].bounds.uMin, perEye[1].bounds.uMax, perEye[1].bounds.vMin, perEye[1].bounds.vMax
			, pPose->m[0][0], pPose->m[0][1], pPose->m[0][2], pPose->m[0][3]
			, pPose->m[1][0], pPose->m[1][1], pPose->m[1][2], pPose->m[1][3]
			, pPose->m[2][0], pPose->m[2][1], pPose->m[2][2], pPose->m[2][3]
		);
		// pPose is qRotation which is calculated by SteamVR using vr::DriverPose_t::qRotation.
		// pPose->m[0][0], pPose->m[0][1], pPose->m[0][2],
		// pPose->m[1][0], pPose->m[1][1], pPose->m[1][2], 
		// pPose->m[2][0], pPose->m[2][1], pPose->m[2][2], 
		// position
		// x = pPose->m[0][3], y = pPose->m[1][3], z = pPose->m[2][3]

		if (m_submitLayer == 0) {
			// Detect FrameIndex of submitted frame by pPose.
			// This is important part to achieve smooth headtracking.
			// We search for history of TrackingInfo and find the TrackingInfo which have nearest matrix value.

			m_poseMutex.Wait(INFINITE);
			float minDiff = 100000;
			int index = 0;
			int minIndex = 0;
			auto minIt = m_poseBuffer.begin();
			for (auto it = m_poseBuffer.begin(); it != m_poseBuffer.end(); it++, index++) {
				float distance = 0;
				// Rotation matrix composes a part of ViewMatrix of TrackingInfo.
				// Be carefull of transpose.
				// And bottom side and right side of matrix should not be compared, because pPose does not contain that part of matrix.
				for (int i = 0; i < 3; i++) {
					for (int j = 0; j < 3; j++) {
						distance += pow(it->rotationMatrix.m[j][i] - pPose->m[j][i], 2);
					}
				}
				//Log("diff %f %llu", distance, it->info.FrameIndex);
				if (minDiff > distance) {
					minIndex = index;
					minIt = it;
					minDiff = distance;
				}
			}
			if (minIt != m_poseBuffer.end()) {
				// found the frameIndex
				m_prevSubmitFrameIndex = m_submitFrameIndex;
				m_prevSubmitClientTime = m_submitClientTime;
				m_submitFrameIndex = minIt->info.FrameIndex;
				m_submitClientTime = minIt->info.clientTime;

				m_prevFramePoseRotation = m_framePoseRotation;
				m_framePoseRotation.x = minIt->info.HeadPose_Pose_Orientation.x;
				m_framePoseRotation.y = minIt->info.HeadPose_Pose_Orientation.y;
				m_framePoseRotation.z = minIt->info.HeadPose_Pose_Orientation.z;
				m_framePoseRotation.w = minIt->info.HeadPose_Pose_Orientation.w;

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
			m_submitLayers[m_submitLayer][0] = perEye[0];
			m_submitLayers[m_submitLayer][1] = perEye[1];
			m_submitLayer++;
		}
		else {
			Log("Too many layers submitted!");
		}

		//CopyTexture();
	}

	/** Submits queued layers for display. */
	virtual void Present(vr::SharedTextureHandle_t syncTexture) override
	{
		bool useMutex = Settings::Instance().m_UseKeyedMutex;
		Log("Present syncTexture=%p (use:%d) m_prevSubmitFrameIndex=%llu m_submitFrameIndex=%llu", syncTexture, useMutex, m_prevSubmitFrameIndex, m_submitFrameIndex);

		IDXGIKeyedMutex *pKeyedMutex = NULL;

		uint32_t layerCount = m_submitLayer;
		m_submitLayer = 0;

		if (m_prevSubmitFrameIndex == m_submitFrameIndex) {
			Log("Discard duplicated frame. FrameIndex=%llu", m_submitFrameIndex);
			return;
		}
		/*if (m_submitFrameIndex != m_LastReferencedFrameIndex) {
		// Discard old frames
		Log("Discarding old frame: m_submitFrameIndex=%llu m_LastReferencedFrameIndex=%llu", m_submitFrameIndex, m_LastReferencedFrameIndex);
		return;
		}*/

		ID3D11Texture2D *pSyncTexture = m_pD3DRender->GetSharedTexture((HANDLE)syncTexture);
		if (!pSyncTexture)
		{
			Log("[VDispDvr] SyncTexture is NULL!");
			return;
		}

		if (useMutex) {
			// Access to shared texture must be wrapped in AcquireSync/ReleaseSync
			// to ensure the compositor has finished rendering to it before it gets used.
			// This enforces scheduling of work on the gpu between processes.
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
		}

		CopyTexture(layerCount);

		if (useMutex) {
			if (pKeyedMutex)
			{
				pKeyedMutex->ReleaseSync(0);
				pKeyedMutex->Release();
			}
			Log("[VDispDvr] Mutex Released.");
		}

		m_pEncoder->NewFrameReady();
	}

	void CopyTexture(uint32_t layerCount) {

		uint64_t presentationTime = GetTimestampUs();

		ID3D11Texture2D *pTexture[MAX_LAYERS][2];
		ComPtr<ID3D11Texture2D> Texture[MAX_LAYERS][2];
		vr::VRTextureBounds_t bounds[MAX_LAYERS][2];

		for (uint32_t i = 0; i < layerCount; i++) {
			// Find left eye texture.
			HANDLE leftEyeTexture = (HANDLE)m_submitLayers[i][0].hTexture;
			auto it = m_handleMap.find(leftEyeTexture);
			if (it == m_handleMap.end()) {
				// Ignore this layer.
				Log("Submitted texture is not found on HandleMap. eye=right layer=%d/%d Texture Handle=%p", i, layerCount, leftEyeTexture);
			}
			else {
				Texture[i][0] = it->second.first->textures[it->second.second];
				D3D11_TEXTURE2D_DESC desc;
				Texture[i][0]->GetDesc(&desc);

				Log("CopyTexture: layer=%d/%d pid=%d Texture Size=%dx%d Format=%d", i, layerCount, it->second.first->pid, desc.Width, desc.Height, desc.Format);

				// Find right eye texture.
				HANDLE rightEyeTexture = (HANDLE)m_submitLayers[i][1].hTexture;
				it = m_handleMap.find(rightEyeTexture);
				if (it == m_handleMap.end()) {
					// Ignore this layer
					Log("Submitted texture is not found on HandleMap. eye=left layer=%d/%d Texture Handle=%p", i, layerCount, rightEyeTexture);
					Texture[i][0].Reset();
				}
				else {
					Texture[i][1] = it->second.first->textures[it->second.second];
				}
			}

			pTexture[i][0] = Texture[i][0].Get();
			pTexture[i][1] = Texture[i][1].Get();
			bounds[i][0] = m_submitLayers[i][0].bounds;
			bounds[i][1] = m_submitLayers[i][1].bounds;
		}

		// This can go away, but is useful to see it as a separate packet on the gpu in traces.
		m_pD3DRender->GetContext()->Flush();

		Log("Waiting for finish of previous encode.");

		if (m_captureDDSTrigger) {
			wchar_t buf[1000];

			for (uint32_t i = 0; i < layerCount; i++) {
				Log("Writing Debug DDS. m_LastReferencedFrameIndex=%llu layer=%d/%d", 0, i, layerCount);
				_snwprintf_s(buf, sizeof(buf), L"%hs\\debug-%llu-%d-%d.dds", Settings::Instance().m_DebugOutputDir.c_str(), m_submitFrameIndex, i, layerCount);
				HRESULT hr = DirectX::SaveDDSTextureToFile(m_pD3DRender->GetContext(), pTexture[i][0], buf);
				Log("Writing Debug DDS: End hr=%p %s", hr, GetDxErrorStr(hr).c_str());
			}
			m_captureDDSTrigger = false;
		}

		// Wait for the encoder to be ready.  This is important because the encoder thread
		// blocks on transmit which uses our shared d3d context (which is not thread safe).
		m_pEncoder->WaitForEncode();

		std::string debugText;

		if (Settings::Instance().m_DebugFrameIndex) {
			TrackingInfo info;
			m_Listener->GetTrackingInfo(info);

			char buf[2000];
			snprintf(buf, sizeof(buf), "%llu\n%f\n%f", m_prevSubmitFrameIndex, m_prevFramePoseRotation.x, info.HeadPose_Pose_Orientation.x);
			debugText = buf;
		}

		// Copy entire texture to staging so we can read the pixels to send to remote device.
		m_pEncoder->CopyToStaging(pTexture, bounds, layerCount, m_recenterManager->IsRecentering(), presentationTime, m_submitFrameIndex, m_submitClientTime, debugText);

		m_pD3DRender->GetContext()->Flush();
	}

private:
	std::shared_ptr<CD3DRender> m_pD3DRender;
	std::shared_ptr<CEncoder> m_pEncoder;
	std::shared_ptr<Listener> m_Listener;
	std::shared_ptr<RecenterManager> m_recenterManager;

	// Resource for each process
	struct ProcessResource {
		ComPtr<ID3D11Texture2D> textures[3];
		HANDLE sharedHandles[3];
		uint32_t pid;
	};
	std::map<HANDLE, std::pair<ProcessResource *, int> > m_handleMap;

	static const int MAX_LAYERS = 10;
	int m_submitLayer;
	SubmitLayerPerEye_t m_submitLayers[MAX_LAYERS][2];
	vr::HmdQuaternion_t m_prevFramePoseRotation;
	vr::HmdQuaternion_t m_framePoseRotation;
	uint64_t m_submitFrameIndex;
	uint64_t m_submitClientTime;
	uint64_t m_prevSubmitFrameIndex;
	uint64_t m_prevSubmitClientTime;

	uint64_t m_LastReferencedFrameIndex;
	uint64_t m_LastReferencedClientTime;

	IPCMutex m_poseMutex;
	struct TrackingHistoryFrame {
		TrackingInfo info;
		vr::HmdMatrix34_t rotationMatrix;
	};
	std::list<TrackingHistoryFrame> m_poseBuffer;

	bool m_captureDDSTrigger;
};

//-----------------------------------------------------------------------------
// Purpose:
//-----------------------------------------------------------------------------
class CRemoteHmd : public vr::ITrackedDeviceServerDriver
{
public:
	CRemoteHmd(std::shared_ptr<Listener> listener)
		: m_unObjectId(vr::k_unTrackedDeviceIndexInvalid)
		, m_nGraphicsAdapterLuid(0)
		, m_nVsyncCounter(0)
		, m_controllerDetected(false)
		, m_added(false)
		, m_Listener(listener)
	{
		m_unObjectId = vr::k_unTrackedDeviceIndexInvalid;
		m_ulPropertyContainer = vr::k_ulInvalidPropertyContainer;

		Log("Startup: %s %s", APP_MODULE_NAME, APP_VERSION_STRING);

		std::function<void()> launcherCallback = [&]() { Enable(); };
		std::function<void(std::string, std::string)> commandCallback = [&](std::string commandName, std::string args) { CommandCallback(commandName, args); };
		std::function<void()> poseCallback = [&]() { OnPoseUpdated(); };
		std::function<void(int)> newClientCallback = [&](int refreshRate) { OnNewClient(refreshRate); };

		m_Listener->SetLauncherCallback(launcherCallback);
		m_Listener->SetCommandCallback(commandCallback);
		m_Listener->SetPoseUpdatedCallback(poseCallback);
		m_Listener->SetNewClientCallback(newClientCallback);

		Log("CRemoteHmd successfully initialized.");
	}

	virtual ~CRemoteHmd()
	{
		if (m_encoder)
		{
			m_encoder->Stop();
			m_encoder.reset();
		}

		if (m_CNvEncoder)
		{
			m_CNvEncoder->Shutdown();
			m_CNvEncoder.reset();
		}

		if (m_audioCapture)
		{
			m_audioCapture->Shutdown();
			m_audioCapture.reset();
		}

		if (m_Listener)
		{
			m_Listener->Stop();
			m_Listener.reset();
		}

		if (m_VSyncThread)
		{
			m_VSyncThread->Shutdown();
			m_VSyncThread.reset();
		}

		if (m_D3DRender)
		{
			m_D3DRender->Shutdown();
			m_D3DRender.reset();
		}

		m_recenterManager.reset();
	}

	std::string GetSerialNumber() const { return Settings::Instance().m_sSerialNumber; }
	
	void Enable()
	{
		if (m_added) {
			return;
		}
		m_added = true;
		bool ret;
		ret = vr::VRServerDriverHost()->TrackedDeviceAdded(
			GetSerialNumber().c_str(),
			vr::TrackedDeviceClass_HMD,
			this);
		Log("TrackedDeviceAdded(HMD) Ret=%d SerialNumber=%s", ret, GetSerialNumber().c_str());
		if (Settings::Instance().m_useTrackingReference) {
			m_trackingReference = std::make_shared<TrackingReference>();
			ret = vr::VRServerDriverHost()->TrackedDeviceAdded(
				m_trackingReference->GetSerialNumber().c_str(),
				vr::TrackedDeviceClass_TrackingReference,
				m_trackingReference.get());
			Log("TrackedDeviceAdded(TrackingReference) Ret=%d SerialNumber=%s", ret, GetSerialNumber().c_str());
		}
		
	}

	virtual vr::EVRInitError Activate(vr::TrackedDeviceIndex_t unObjectId) override
	{
		Log("CRemoteHmd Activate %d", unObjectId);

		m_unObjectId = unObjectId;
		m_ulPropertyContainer = vr::VRProperties()->TrackedDeviceToPropertyContainer(m_unObjectId);

		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_ModelNumber_String, Settings::Instance().m_sModelNumber.c_str());
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_RenderModelName_String, Settings::Instance().m_sModelNumber.c_str());
		vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_UserIpdMeters_Float, Settings::Instance().m_flIPD);
		vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_UserHeadToEyeDepthMeters_Float, 0.f);
		vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_DisplayFrequency_Float, Settings::Instance().m_flDisplayFrequency);
		vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_SecondsFromVsyncToPhotons_Float, Settings::Instance().m_flSecondsFromVsyncToPhotons);
		vr::VRProperties()->SetUint64Property(m_ulPropertyContainer, vr::Prop_GraphicsAdapterLuid_Uint64, m_nGraphicsAdapterLuid);

		// return a constant that's not 0 (invalid) or 1 (reserved for Oculus)
		vr::VRProperties()->SetUint64Property(m_ulPropertyContainer, vr::Prop_CurrentUniverseId_Uint64, 2);

		// avoid "not fullscreen" warnings from vrmonitor
		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_IsOnDesktop_Bool, false);

		// Manually send VSync events on direct mode. ref:https://github.com/ValveSoftware/virtual_display/issues/1
		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_DriverDirectModeSendsVsyncEvents_Bool, true);

		float originalIPD = vr::VRSettings()->GetFloat(vr::k_pch_SteamVR_Section, vr::k_pch_SteamVR_IPD_Float);
		vr::VRSettings()->SetFloat(vr::k_pch_SteamVR_Section, vr::k_pch_SteamVR_IPD_Float, Settings::Instance().m_flIPD);


		m_D3DRender = std::make_shared<CD3DRender>();

		// Store off the LUID of the primary gpu we want to use.
		if (!m_D3DRender->GetAdapterLuid(Settings::Instance().m_nAdapterIndex, &m_nGraphicsAdapterLuid))
		{
			FatalLog("Failed to get adapter index for graphics adapter!");
			return vr::VRInitError_Driver_Failed;
		}

		// Now reinitialize using the other graphics card.
		if (!m_D3DRender->Initialize(Settings::Instance().m_nAdapterIndex))
		{
			FatalLog("Could not create graphics device for adapter %d.  Requires a minimum of two graphics cards.", Settings::Instance().m_nAdapterIndex);
			return vr::VRInitError_Driver_Failed;
		}

		int32_t nDisplayAdapterIndex;
		wchar_t wchAdapterDescription[300];
		if (!m_D3DRender->GetAdapterInfo(&nDisplayAdapterIndex, wchAdapterDescription, sizeof(wchAdapterDescription) / sizeof(wchar_t)))
		{
			FatalLog("Failed to get primary adapter info!");
			return vr::VRInitError_Driver_Failed;
		}

		Log("Using %ls as primary graphics adapter.", wchAdapterDescription);

		// Spawn our separate process to manage headset presentation.
		m_CNvEncoder = std::make_shared<CNvEncoder>(m_D3DRender, m_Listener, ShouldUseNV12Texture());
		if (!m_CNvEncoder->Initialize())
		{
			return vr::VRInitError_Driver_Failed;
		}

		// Spin up a separate thread to handle the overlapped encoding/transmit step.
		m_encoder = std::make_shared<CEncoder>(m_D3DRender, m_CNvEncoder);
		m_encoder->Start();

		m_audioCapture = std::make_shared<AudioCapture>(m_Listener);
		try {
			std::vector<std::wstring> audioDevices;
			AudioCapture::list_devices(audioDevices);
			if (audioDevices.size() == 0) {
				Log("Could not find any audio devices.");
			}
			else {
				m_audioCapture->Start(audioDevices[0]);
			}
		}
		catch (Exception e) {
			FatalLog("Failed to start audio capture. %s", e.what());
			Sleep(5 * 1000);
			return vr::VRInitError_Driver_Failed;
		}

		m_VSyncThread = std::make_shared<VSyncThread>();
		m_VSyncThread->Start();

		m_recenterManager = std::make_shared<RecenterManager>();

		m_displayComponent = std::make_shared<DisplayComponent>();
		m_directModeComponent = std::make_shared<DirectModeComponent>(m_D3DRender, m_encoder, m_Listener, m_recenterManager);

		return vr::VRInitError_None;
	}

	virtual void Deactivate() override
	{
		Log("CRemoteHmd Deactivate");
		m_unObjectId = vr::k_unTrackedDeviceIndexInvalid;
	}

	virtual void EnterStandby() override
	{
	}

	void *GetComponent(const char *pchComponentNameAndVersion) override
	{
		Log("GetComponent %s", pchComponentNameAndVersion);
		if (!_stricmp(pchComponentNameAndVersion, vr::IVRDisplayComponent_Version))
		{
			return m_displayComponent.get();
		}
		if (!_stricmp(pchComponentNameAndVersion, vr::IVRDriverDirectModeComponent_Version))
		{
			return m_directModeComponent.get();
		}

		// override this to add a component to a driver
		return NULL;
	}

	/** debug request from a client */
	virtual void DebugRequest(const char *pchRequest, char *pchResponseBuffer, uint32_t unResponseBufferSize) override
	{
		if (unResponseBufferSize >= 1)
			pchResponseBuffer[0] = 0;
	}

	virtual vr::DriverPose_t GetPose() override
	{
		vr::DriverPose_t pose = { 0 };
		pose.poseIsValid = true;
		pose.result = vr::TrackingResult_Running_OK;
		pose.deviceIsConnected = true;

		pose.qWorldFromDriverRotation = HmdQuaternion_Init(1, 0, 0, 0);
		pose.qDriverFromHeadRotation = HmdQuaternion_Init(1, 0, 0, 0);
		pose.qRotation = HmdQuaternion_Init(1, 0, 0, 0);

		if (m_Listener->HasValidTrackingInfo()) {
			TrackingInfo info;
			m_Listener->GetTrackingInfo(info);
			uint64_t trackingDelay = GetTimestampUs() - m_Listener->clientToServerTime(info.clientTime);

			Log("Tracking elapsed:%lld us FrameIndex=%lld quot:%f,%f,%f,%f\nposition:%f,%f,%f\narcore:%f,%f,%f",
				trackingDelay,
				info.FrameIndex,
				info.HeadPose_Pose_Orientation.x,
				info.HeadPose_Pose_Orientation.y,
				info.HeadPose_Pose_Orientation.z,
				info.HeadPose_Pose_Orientation.w,
				info.HeadPose_Pose_Position.x,
				info.HeadPose_Pose_Position.y,
				info.HeadPose_Pose_Position.z,
				info.Other_Tracking_Source_Position.x,
				info.Other_Tracking_Source_Position.y,
				info.Other_Tracking_Source_Position.z
			);

			pose.qRotation = m_recenterManager->GetRecentered(info.HeadPose_Pose_Orientation);

			TrackingVector3 position = m_recenterManager->GetRecenteredVector(info.HeadPose_Pose_Position);
			pose.vecPosition[0] = position.x;
			pose.vecPosition[1] = position.y;
			pose.vecPosition[2] = position.z;

			if (info.flags & TrackingInfo::FLAG_OTHER_TRACKING_SOURCE) {
				pose.vecPosition[0] += info.Other_Tracking_Source_Position.x;
				pose.vecPosition[1] += info.Other_Tracking_Source_Position.y;
				pose.vecPosition[2] += info.Other_Tracking_Source_Position.z;
			}

			if (Settings::Instance().m_EnableOffsetPos) {
				Log("Provide fake position(offset) for debug. Coords=(%f, %f, %f)"
					, Settings::Instance().m_OffsetPos[0], Settings::Instance().m_OffsetPos[1], Settings::Instance().m_OffsetPos[2]);
				pose.vecPosition[0] += Settings::Instance().m_OffsetPos[0];
				pose.vecPosition[1] += Settings::Instance().m_OffsetPos[1];
				pose.vecPosition[2] += Settings::Instance().m_OffsetPos[2];
			}

			// To disable time warp (or pose prediction), we dont set (set to zero) velocity and acceleration.

			pose.poseTimeOffset = 0;
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


	void CommandCallback(std::string commandName, std::string args)
	{
		if (commandName == "EnableDriverTestMode") {
			g_DriverTestMode = strtoull(args.c_str(), NULL, 0);
			m_Listener->SendCommandResponse("OK\n");
		}
		else if (commandName == "GetConfig") {
			char buf[1000];
			snprintf(buf, sizeof(buf)
				, "%s"
				"%s %d\n"
				"%s %d\n"
				"%s %d\n"
				"%s %d\n"
				"%s %d\n"
				"%s %d\n"
				"%s %d\n"
				"%s %d\n"
				"%s %d"
				, m_Listener->DumpConfig().c_str()
				, k_pch_Settings_DebugLog_Bool, Settings::Instance().m_DebugLog
				, k_pch_Settings_DebugFrameIndex_Bool, Settings::Instance().m_DebugFrameIndex
				, k_pch_Settings_DebugFrameOutput_Bool, Settings::Instance().m_DebugFrameOutput
				, k_pch_Settings_DebugCaptureOutput_Bool, Settings::Instance().m_DebugCaptureOutput
				, k_pch_Settings_UseKeyedMutex_Bool, Settings::Instance().m_UseKeyedMutex
				, k_pch_Settings_ControllerTriggerMode_Int32, Settings::Instance().m_controllerTriggerMode
				, k_pch_Settings_ControllerTrackpadClickMode_Int32, Settings::Instance().m_controllerTrackpadClickMode
				, k_pch_Settings_ControllerTrackpadTouchMode_Int32, Settings::Instance().m_controllerTrackpadTouchMode
				, k_pch_Settings_ControllerRecenterButton_Int32, Settings::Instance().m_controllerRecenterButton
			);
			m_Listener->SendCommandResponse(buf);
		}else if(commandName == "SetConfig"){
			auto index = args.find(" ");
			if (index == std::string::npos) {
				m_Listener->SendCommandResponse("NG\n");
			}
			else {
				auto name = args.substr(0, index);
				if (name == k_pch_Settings_DebugFrameIndex_Bool) {
					Settings::Instance().m_DebugFrameIndex = atoi(args.substr(index + 1).c_str());
				}else if(name == k_pch_Settings_DebugFrameOutput_Bool){
					Settings::Instance().m_DebugFrameOutput = atoi(args.substr(index + 1).c_str());
				}
				else if (name == k_pch_Settings_DebugCaptureOutput_Bool) {
					Settings::Instance().m_DebugCaptureOutput = atoi(args.substr(index + 1).c_str());
				}
				else if (name == k_pch_Settings_UseKeyedMutex_Bool) {
					Settings::Instance().m_UseKeyedMutex = atoi(args.substr(index + 1).c_str());
				}
				else if (name == k_pch_Settings_ControllerTriggerMode_Int32) {
					Settings::Instance().m_controllerTriggerMode = atoi(args.substr(index + 1).c_str());
				}
				else if (name == k_pch_Settings_ControllerTrackpadClickMode_Int32) {
					Settings::Instance().m_controllerTrackpadClickMode = atoi(args.substr(index + 1).c_str());
				}
				else if (name == k_pch_Settings_ControllerTrackpadTouchMode_Int32) {
					Settings::Instance().m_controllerTrackpadTouchMode = atoi(args.substr(index + 1).c_str());
				}
				else if (name == k_pch_Settings_ControllerRecenterButton_Int32) {
					Settings::Instance().m_controllerRecenterButton = atoi(args.substr(index + 1).c_str());
				}
				else if (name == "causePacketLoss") {
					Settings::Instance().m_causePacketLoss = atoi(args.substr(index + 1).c_str());
				}
				else {
					m_Listener->SendCommandResponse("NG\n");
					return;
				}
				m_Listener->SendCommandResponse("OK\n");
			}
		}
		else if (commandName == "SetOffsetPos") {
			std::string enabled = GetNextToken(args, " ");
			std::string x = GetNextToken(args, " ");
			std::string y = GetNextToken(args, " ");
			std::string z = GetNextToken(args, " ");
			Settings::Instance().m_OffsetPos[0] = (float)atof(x.c_str());
			Settings::Instance().m_OffsetPos[1] = (float)atof(y.c_str());
			Settings::Instance().m_OffsetPos[2] = (float)atof(z.c_str());

			Settings::Instance().m_EnableOffsetPos = atoi(enabled.c_str()) != 0;

			m_Listener->SendCommandResponse("OK\n");
		}else {
			if (!m_directModeComponent->CommandCallback(commandName, args)) {
				Log("Invalid control command: %s", commandName.c_str());
				m_Listener->SendCommandResponse("NG\n");
			}
		}
		
	}

	void OnPoseUpdated() {
		if (m_unObjectId != vr::k_unTrackedDeviceIndexInvalid)
		{
			if (!m_Listener->HasValidTrackingInfo()) {
				return;
			}

			TrackingInfo info;
			m_Listener->GetTrackingInfo(info);

			m_directModeComponent->OnPoseUpdated(info);
			m_recenterManager->OnPoseUpdated(info);
			
			vr::VRServerDriverHost()->TrackedDevicePoseUpdated(m_unObjectId, GetPose(), sizeof(vr::DriverPose_t));

			Log("Generate VSync Event by OnPoseUpdated");
			m_VSyncThread->InsertVsync();
			
			UpdateControllerState(info);

			if (m_trackingReference) {
				m_trackingReference->OnPoseUpdated();
			}
		}
	}

	void UpdateControllerState(const TrackingInfo& info) {
		if (!Settings::Instance().m_enableController) {
			return;
		}
		if (!m_controllerDetected) {
			if (info.flags & TrackingInfo::FLAG_CONTROLLER_ENABLE) {
				Log("New controller is detected.");
				m_controllerDetected = true;

				// false: right hand, true: left hand
				bool handed = false;
				if (info.flags & TrackingInfo::FLAG_CONTROLLER_LEFTHAND) {
					handed = true;
				}
				m_remoteController = std::make_shared<RemoteControllerServerDriver>(handed, m_recenterManager);

				bool ret;
				ret = vr::VRServerDriverHost()->TrackedDeviceAdded(
					m_remoteController->GetSerialNumber().c_str(),
					vr::TrackedDeviceClass_Controller,
					m_remoteController.get());
				Log("TrackedDeviceAdded Ret=%d SerialNumber=%s", ret, m_remoteController->GetSerialNumber().c_str());
			}
		}
		if (info.flags & TrackingInfo::FLAG_CONTROLLER_ENABLE) {
			bool recenterRequested = m_remoteController->ReportControllerState(info);
			if (recenterRequested) {
				m_recenterManager->BeginRecenter();
			}
		}
	}

	void OnNewClient(int refreshRate) {
		m_refreshRate = refreshRate;

		vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_DisplayFrequency_Float, (float)m_refreshRate);
		// Insert IDR frame for faster startup of decoding.
		m_CNvEncoder->InsertIDR();
	}

private:
	bool m_added;
	vr::TrackedDeviceIndex_t m_unObjectId;
	vr::PropertyContainerHandle_t m_ulPropertyContainer;

	uint64_t m_nGraphicsAdapterLuid;
	uint32_t m_nVsyncCounter;

	std::shared_ptr<CD3DRender> m_D3DRender;
	std::shared_ptr<CNvEncoder> m_CNvEncoder;
	std::shared_ptr<CEncoder> m_encoder;
	std::shared_ptr<AudioCapture> m_audioCapture;
	std::shared_ptr<Listener> m_Listener;
	std::shared_ptr<VSyncThread> m_VSyncThread;
	std::shared_ptr<RecenterManager> m_recenterManager;

	bool m_controllerDetected;
	std::shared_ptr<RemoteControllerServerDriver> m_remoteController;

	std::shared_ptr<DisplayComponent> m_displayComponent;
	std::shared_ptr<DirectModeComponent> m_directModeComponent;

	std::shared_ptr<TrackingReference> m_trackingReference;

	int m_refreshRate;
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
	std::shared_ptr<CRemoteHmd> m_pRemoteHmd;
	std::shared_ptr<Listener> m_Listener;
	std::shared_ptr<IPCMutex> m_mutex;
};

vr::EVRInitError CServerDriver_DisplayRedirect::Init( vr::IVRDriverContext *pContext )
{
	VR_INIT_SERVER_DRIVER_CONTEXT( pContext );

	m_mutex = std::make_shared<IPCMutex>(APP_MUTEX_NAME, true);
	if (m_mutex->AlreadyExist()) {
		// Duplicate driver installation.
		FatalLog("ALVR Server driver is installed on multiple locations. This causes some issues.\r\n"
			"Please check the installed driver list on About tab and uninstall old drivers.");
		return vr::VRInitError_Driver_Failed;
	}

	Settings::Instance().Load();

	m_Listener = std::make_shared<Listener>();
	if (!m_Listener->Startup())
	{
		return vr::VRInitError_Driver_Failed;
	}

	m_pRemoteHmd = std::make_shared<CRemoteHmd>(m_Listener);

	if (Settings::Instance().IsLoaded()) {
		// Launcher is running. Enable driver.
		m_pRemoteHmd->Enable();
	}

	return vr::VRInitError_None;
}

void CServerDriver_DisplayRedirect::Cleanup()
{
	m_Listener.reset();
	m_pRemoteHmd.reset();
	m_mutex.reset();

	VR_CLEANUP_SERVER_DRIVER_CONTEXT();
}

void CServerDriver_DisplayRedirect::RunFrame()
{
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