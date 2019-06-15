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

#include <d3d11.h>
#include <wrl.h>
#include <map>
#include <d3d11_1.h>
#include <ScreenGrab.h>
#include <wincodec.h>
#include <wincodecsdk.h>

#include "Logger.h"
#include "Listener.h"
#include "Utils.h"
#include "FrameRender.h"
#include "Settings.h"
#include "RemoteController.h"
#include "RecenterManager.h"
#include "packet_types.h"
#include "resource.h"
#include "Tracking.h"
#include "AudioCapture.h"
#include "VideoEncoder.h"
#include "VideoEncoderNVENC.h"
#include "VideoEncoderVCE.h"
#include "IDRScheduler.h"

HINSTANCE g_hInstance;

uint64_t g_DriverTestMode = 0;

namespace
{
	using Microsoft::WRL::ComPtr;

	//----------------------------------------------------------------------------
	// Blocks on reading backbuffer from gpu, so WaitForPresent can return
	// as soon as we know rendering made it this frame.  This step of the pipeline
	// should run about 3ms per frame.
	//----------------------------------------------------------------------------
	class CEncoder : public CThread
	{
	public:
		CEncoder()
			: m_bExiting( false )
			, m_frameIndex(0)
			, m_frameIndex2(0)
		{
			m_encodeFinished.Set();
		}

		~CEncoder()
		{
			if (m_videoEncoder)
			{
				m_videoEncoder->Shutdown();
				m_videoEncoder.reset();
			}
		}

		void Initialize(std::shared_ptr<CD3DRender> d3dRender, std::shared_ptr<Listener> listener) {
			m_FrameRender = std::make_shared<FrameRender>(d3dRender);

			Exception vceException;
			Exception nvencException;
			try {
				Log(L"Try to use VideoEncoderVCE.");
				m_videoEncoder = std::make_shared<VideoEncoderVCE>(d3dRender, listener);
				m_videoEncoder->Initialize();
				return;
			}
			catch (Exception e) {
				vceException = e;
			}
			try {
				Log(L"Try to use VideoEncoderNVENC.");
				m_videoEncoder = std::make_shared<VideoEncoderNVENC>(d3dRender, listener
					, ShouldUseNV12Texture());
				m_videoEncoder->Initialize();
				return;
			}
			catch (Exception e) {
				nvencException = e;
			}
			throw MakeException(L"All VideoEncoder are not available. VCE: %s, NVENC: %s", vceException.what(), nvencException.what());
		}

		bool CopyToStaging( ID3D11Texture2D *pTexture[][2], vr::VRTextureBounds_t bounds[][2], int layerCount, bool recentering
			, uint64_t presentationTime, uint64_t frameIndex, uint64_t clientTime, const std::string& message, const std::string& debugText)
		{
			m_presentationTime = presentationTime;
			m_frameIndex = frameIndex;
			m_clientTime = clientTime;
			m_FrameRender->Startup();

			char buf[200];
			snprintf(buf, sizeof(buf), "\nindex2: %llu", m_frameIndex2);

			m_FrameRender->RenderFrame(pTexture, bounds, layerCount, recentering, message, debugText + buf);
			return true;
		}

		void Run() override
		{
			Log(L"CEncoder: Start thread. Id=%d", GetCurrentThreadId());
			SetThreadPriority( GetCurrentThread(), THREAD_PRIORITY_MOST_URGENT );

			while ( !m_bExiting )
			{
				Log(L"CEncoder: Waiting for new frame...");

				m_newFrameReady.Wait();
				if ( m_bExiting )
					break;

				if ( m_FrameRender->GetTexture() )
				{
					m_videoEncoder->Transmit(m_FrameRender->GetTexture().Get(), m_presentationTime, m_frameIndex, m_frameIndex2, m_clientTime, m_scheduler.CheckIDRInsertion());
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
			Log(L"New Frame Ready");
			m_encodeFinished.Reset();
			m_newFrameReady.Set();
		}

		void WaitForEncode()
		{
			m_encodeFinished.Wait();
		}

		void OnStreamStart() {
			m_scheduler.OnStreamStart();
		}

		void OnPacketLoss() {
			m_scheduler.OnPacketLoss();
		}

		void Reconfigure(int refreshRate, int renderWidth, int renderHeight, int bitrateInMBits) {
			m_videoEncoder->Reconfigure(refreshRate, renderWidth, renderHeight, bitrateInMBits);
		}
	private:
		CThreadEvent m_newFrameReady, m_encodeFinished;
		std::shared_ptr<VideoEncoder> m_videoEncoder;
		bool m_bExiting;
		uint64_t m_presentationTime;
		uint64_t m_frameIndex;
		uint64_t m_clientTime;

		uint64_t m_frameIndex2;

		std::shared_ptr<FrameRender> m_FrameRender;

		IDRScheduler m_scheduler;
	};
}

// VSync Event Thread

class VSyncThread : public CThread
{
public:
	VSyncThread(int refreshRate) 
		: m_bExit(false)
		, m_refreshRate(refreshRate) {}

	// Trigger VSync if elapsed time from previous VSync is larger than 30ms.
	void Run()override {
		m_PreviousVsync = 0;

		while (!m_bExit) {
			uint64_t current = GetTimestampUs();
			uint64_t interval = 1000 * 1000 / m_refreshRate;

			if (m_PreviousVsync + interval > current) {
				uint64_t sleepTimeMs = (m_PreviousVsync + interval - current) / 1000;

				if (sleepTimeMs > 0) {
					Log(L"Sleep %llu ms for next VSync.", sleepTimeMs);
					Sleep(static_cast<DWORD>(sleepTimeMs));
				}

				m_PreviousVsync += interval;
			}
			else {
				m_PreviousVsync = current;
			}
			Log(L"Generate VSync Event by VSyncThread");
			vr::VRServerDriverHost()->VsyncEvent(0);
		}
	}

	void Shutdown() {
		m_bExit = true;
	}

	void SetRefreshRate(int refreshRate) {
		m_refreshRate = refreshRate;
	}
private:
	bool m_bExit;
	uint64_t m_PreviousVsync;
	int m_refreshRate = 60;
};

class DisplayComponent : public vr::IVRDisplayComponent
{
public:
	DisplayComponent() {}
	virtual ~DisplayComponent() {}

	virtual void GetWindowBounds(int32_t *pnX, int32_t *pnY, uint32_t *pnWidth, uint32_t *pnHeight) override
	{
		Log(L"GetWindowBounds %dx%d - %dx%d", 0, 0, Settings::Instance().m_renderWidth, Settings::Instance().m_renderHeight);
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
		Log(L"GetRecommendedRenderTargetSize %dx%d", *pnWidth, *pnHeight);
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
		Log(L"GetEyeOutputViewport Eye=%d %dx%d %dx%d", eEye, *pnX, *pnY, *pnWidth, *pnHeight);
	}

	virtual void GetProjectionRaw(vr::EVREye eEye, float *pfLeft, float *pfRight, float *pfTop, float *pfBottom) override
	{
		auto eyeFov = Settings::Instance().m_eyeFov[eEye];
		*pfLeft = -tanf(static_cast<float>(eyeFov.left / 180.0 * M_PI));
		*pfRight = tanf(static_cast<float>(eyeFov.right / 180.0 * M_PI));
		*pfTop = -tanf(static_cast<float>(eyeFov.top / 180.0 * M_PI));
		*pfBottom = tanf(static_cast<float>(eyeFov.bottom / 180.0 * M_PI));

		Log(L"GetProjectionRaw Eye=%d (l,r,t,b)=(%f,%f,%f,%f)", eEye, eyeFov.left, eyeFov.right, eyeFov.top, eyeFov.bottom);
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
		Log(L"ComputeDistortion %f,%f", fU, fV);
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
		: m_pD3DRender(pD3DRender)
		, m_pEncoder(pEncoder)
		, m_Listener(Listener)
		, m_recenterManager(recenterManager)
		, m_poseMutex(NULL)
		, m_submitLayer(0)
		, m_LastReferencedFrameIndex(0) 
		, m_LastReferencedClientTime(0) {
	}

	void OnPoseUpdated(TrackingInfo &info) {
		// Put pose history buffer
		TrackingHistoryFrame history;
		history.info = info;

		vr::HmdQuaternion_t recentered = m_recenterManager->GetRecenteredHMD();
		HmdMatrix_QuatToMat(recentered.w,
			recentered.x,
			recentered.y,
			recentered.z,
			&history.rotationMatrix);

		Log(L"Rotation Matrix=(%f, %f, %f, %f) (%f, %f, %f, %f) (%f, %f, %f, %f)"
			, history.rotationMatrix.m[0][0], history.rotationMatrix.m[0][1], history.rotationMatrix.m[0][2], history.rotationMatrix.m[0][3]
			, history.rotationMatrix.m[1][0], history.rotationMatrix.m[1][1], history.rotationMatrix.m[1][2], history.rotationMatrix.m[1][3]
			, history.rotationMatrix.m[2][0], history.rotationMatrix.m[2][1], history.rotationMatrix.m[2][2], history.rotationMatrix.m[2][3]);

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
		Log(L"CreateSwapTextureSet pid=%d Format=%d %dx%d SampleCount=%d", unPid, pSwapTextureSetDesc->nFormat
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
			//Log(L"texture%d %p res:%d %s", i, texture[i], hr, GetDxErrorStr(hr).c_str());

			IDXGIResource* pResource;
			hr = processResource->textures[i]->QueryInterface(__uuidof(IDXGIResource), (void**)&pResource);
			//Log(L"QueryInterface %p res:%d %s", pResource, hr, GetDxErrorStr(hr).c_str());

			hr = pResource->GetSharedHandle(&processResource->sharedHandles[i]);
			//Log(L"GetSharedHandle %p res:%d %s", processResource->sharedHandles[i], hr, GetDxErrorStr(hr).c_str());

			m_handleMap.insert(std::make_pair(processResource->sharedHandles[i], std::make_pair(processResource, i)));

			(*pSharedTextureHandles)[i] = (vr::SharedTextureHandle_t)processResource->sharedHandles[i];

			pResource->Release();

			Log(L"Created Texture %d %p", i, processResource->sharedHandles[i]);
		}
		//m_processMap.insert(std::pair<uint32_t, ProcessResource *>(unPid, processResource));
	}

	/** Used to textures created using CreateSwapTextureSet.  Only one of the set's handles needs to be used to destroy the entire set. */
	virtual void DestroySwapTextureSet(vr::SharedTextureHandle_t sharedTextureHandle) override
	{
		Log(L"DestroySwapTextureSet %p", sharedTextureHandle);

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
			Log(L"Requested to destroy not managing texture. handle:%p", sharedTextureHandle);
		}
	}

	/** Used to purge all texture sets for a given process. */
	virtual void DestroyAllSwapTextureSets(uint32_t unPid) override
	{
		Log(L"DestroyAllSwapTextureSets pid=%d", unPid);

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
		Log(L"GetNextSwapTextureSetIndex %p %p %d %d", sharedTextureHandles[0], sharedTextureHandles[1], (*pIndices)[0], (*pIndices)[1]);
		(*pIndices)[0]++;
		(*pIndices)[0] %= 3;
		(*pIndices)[1]++;
		(*pIndices)[1] %= 3;
	}

	/** Call once per layer to draw for this frame.  One shared texture handle per eye.  Textures must be created
	* using CreateSwapTextureSet and should be alternated per frame.  Call Present once all layers have been submitted. */
	virtual void SubmitLayer(const SubmitLayerPerEye_t(&perEye)[2], const vr::HmdMatrix34_t *pPose) override
	{
		Log(L"SubmitLayer Handles=%p,%p DepthHandles=%p,%p %f-%f,%f-%f %f-%f,%f-%f\n%f,%f,%f,%f\n%f,%f,%f,%f\n%f,%f,%f,%f"
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
				//Log(L"diff %f %llu", distance, it->info.FrameIndex);
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

				Log(L"Frame pose found. m_prevSubmitFrameIndex=%llu m_submitFrameIndex=%llu minDiff=%f", m_prevSubmitFrameIndex, m_submitFrameIndex, minDiff);
			}
			else {
				m_submitFrameIndex = 0;
				m_submitClientTime = 0;
				m_framePoseRotation = HmdQuaternion_Init(0.0, 0.0, 0.0, 0.0);
			}
			m_poseMutex.Release();
		}
		if (m_submitLayer < MAX_LAYERS) {
			m_submitLayers[m_submitLayer][0] = perEye[0];
			m_submitLayers[m_submitLayer][1] = perEye[1];
			m_submitLayer++;
		}
		else {
			Log(L"Too many layers submitted!");
		}

		if (g_DriverTestMode & 8) {
			// Crash test
			*(char*)0 = 1;
		}

		//CopyTexture();
	}

	/** Submits queued layers for display. */
	virtual void Present(vr::SharedTextureHandle_t syncTexture) override
	{
		bool useMutex = Settings::Instance().m_UseKeyedMutex;
		Log(L"Present syncTexture=%p (use:%d) m_prevSubmitFrameIndex=%llu m_submitFrameIndex=%llu", syncTexture, useMutex, m_prevSubmitFrameIndex, m_submitFrameIndex);

		IDXGIKeyedMutex *pKeyedMutex = NULL;

		uint32_t layerCount = m_submitLayer;
		m_submitLayer = 0;

		if (m_prevSubmitFrameIndex == m_submitFrameIndex) {
			Log(L"Discard duplicated frame. FrameIndex=%llu (Ignoring)", m_submitFrameIndex);
			//return;
		}

		if (!m_Listener->IsStreaming()) {
			Log(L"Discard frame because isStreaming=false. FrameIndex=%llu", m_submitFrameIndex);
			return;
		}

		ID3D11Texture2D *pSyncTexture = m_pD3DRender->GetSharedTexture((HANDLE)syncTexture);
		if (!pSyncTexture)
		{
			Log(L"[VDispDvr] SyncTexture is NULL!");
			return;
		}

		if (useMutex) {
			// Access to shared texture must be wrapped in AcquireSync/ReleaseSync
			// to ensure the compositor has finished rendering to it before it gets used.
			// This enforces scheduling of work on the gpu between processes.
			if (SUCCEEDED(pSyncTexture->QueryInterface(__uuidof(IDXGIKeyedMutex), (void **)&pKeyedMutex)))
			{
				Log(L"[VDispDvr] Wait for SyncTexture Mutex.");
				// TODO: Reasonable timeout and timeout handling
				HRESULT hr = pKeyedMutex->AcquireSync(0, 10);
				if (hr != S_OK)
				{
					Log(L"[VDispDvr] ACQUIRESYNC FAILED!!! hr=%d %p %s", hr, hr, GetErrorStr(hr).c_str());
					pKeyedMutex->Release();
					return;
				}
			}

			Log(L"[VDispDvr] Mutex Acquired.");
		}

		CopyTexture(layerCount);

		if (useMutex) {
			if (pKeyedMutex)
			{
				pKeyedMutex->ReleaseSync(0);
				pKeyedMutex->Release();
			}
			Log(L"[VDispDvr] Mutex Released.");
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
				Log(L"Submitted texture is not found on HandleMap. eye=right layer=%d/%d Texture Handle=%p", i, layerCount, leftEyeTexture);
			}
			else {
				Texture[i][0] = it->second.first->textures[it->second.second];
				D3D11_TEXTURE2D_DESC desc;
				Texture[i][0]->GetDesc(&desc);

				Log(L"CopyTexture: layer=%d/%d pid=%d Texture Size=%dx%d Format=%d", i, layerCount, it->second.first->pid, desc.Width, desc.Height, desc.Format);

				// Find right eye texture.
				HANDLE rightEyeTexture = (HANDLE)m_submitLayers[i][1].hTexture;
				it = m_handleMap.find(rightEyeTexture);
				if (it == m_handleMap.end()) {
					// Ignore this layer
					Log(L"Submitted texture is not found on HandleMap. eye=left layer=%d/%d Texture Handle=%p", i, layerCount, rightEyeTexture);
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

		Log(L"Waiting for finish of previous encode.");

		if (Settings::Instance().m_captureLayerDDSTrigger) {
			wchar_t buf[1000];

			for (uint32_t i = 0; i < layerCount; i++) {
				Log(L"Writing Debug DDS. m_LastReferencedFrameIndex=%llu layer=%d/%d", 0, i, layerCount);
				_snwprintf_s(buf, sizeof(buf), L"%hs\\debug-%llu-%d-%d.dds", Settings::Instance().m_DebugOutputDir.c_str(), m_submitFrameIndex, i, layerCount);
				HRESULT hr = DirectX::SaveDDSTextureToFile(m_pD3DRender->GetContext(), pTexture[i][0], buf);
				Log(L"Writing Debug DDS: End hr=%p %s", hr, GetErrorStr(hr).c_str());
			}
			Settings::Instance().m_captureLayerDDSTrigger = false;
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

		uint64_t submitFrameIndex = m_submitFrameIndex + Settings::Instance().m_trackingFrameOffset;
		Log(L"Fix frame index. FrameIndex=%llu Offset=%d New FrameIndex=%llu"
			, m_submitFrameIndex, Settings::Instance().m_trackingFrameOffset, submitFrameIndex);

		// Copy entire texture to staging so we can read the pixels to send to remote device.
		m_pEncoder->CopyToStaging(pTexture, bounds, layerCount, m_recenterManager->IsRecentering(), presentationTime, submitFrameIndex, m_submitClientTime, m_recenterManager->GetFreePIEMessage(), debugText);

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
};

//-----------------------------------------------------------------------------
// Purpose:
//-----------------------------------------------------------------------------
class CRemoteHmd : public vr::ITrackedDeviceServerDriver
{
public:
	CRemoteHmd(std::shared_ptr<Listener> listener)
		: m_unObjectId(vr::k_unTrackedDeviceIndexInvalid)
		, m_added(false)
		, mActivated(false)
		, m_Listener(listener)
	{
		m_unObjectId = vr::k_unTrackedDeviceIndexInvalid;
		m_ulPropertyContainer = vr::k_ulInvalidPropertyContainer;

		Log(L"Startup: %hs %hs", APP_MODULE_NAME, APP_VERSION_STRING);

		std::function<void()> launcherCallback = [&]() { Enable(); };
		std::function<void(std::string, std::string)> commandCallback = [&](std::string commandName, std::string args) { CommandCallback(commandName, args); };
		std::function<void()> poseCallback = [&]() { OnPoseUpdated(); };
		std::function<void()> newClientCallback = [&]() { OnNewClient(); };
		std::function<void()> streamStartCallback = [&]() { OnStreamStart(); };
		std::function<void()> packetLossCallback = [&]() { OnPacketLoss(); };
		std::function<void()> shutdownCallback = [&]() { OnShutdown(); };

		m_Listener->SetLauncherCallback(launcherCallback);
		m_Listener->SetCommandCallback(commandCallback);
		m_Listener->SetPoseUpdatedCallback(poseCallback);
		m_Listener->SetNewClientCallback(newClientCallback);
		m_Listener->SetStreamStartCallback(streamStartCallback);
		m_Listener->SetPacketLossCallback(packetLossCallback);
		m_Listener->SetShutdownCallback(shutdownCallback);

		Log(L"CRemoteHmd successfully initialized.");
	}

	virtual ~CRemoteHmd()
	{
		if (m_encoder)
		{
			m_encoder->Stop();
			m_encoder.reset();
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

	std::string GetSerialNumber() const { return Settings::Instance().mSerialNumber; }
	
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
		Log(L"TrackedDeviceAdded(HMD) Ret=%d SerialNumber=%hs", ret, GetSerialNumber().c_str());
		if (Settings::Instance().m_useTrackingReference) {
			m_trackingReference = std::make_shared<TrackingReference>();
			ret = vr::VRServerDriverHost()->TrackedDeviceAdded(
				m_trackingReference->GetSerialNumber().c_str(),
				vr::TrackedDeviceClass_TrackingReference,
				m_trackingReference.get());
			Log(L"TrackedDeviceAdded(TrackingReference) Ret=%d SerialNumber=%hs", ret, GetSerialNumber().c_str());
		}
		
	}

	virtual vr::EVRInitError Activate(vr::TrackedDeviceIndex_t unObjectId) override
	{
		Log(L"CRemoteHmd Activate %d", unObjectId);

		m_unObjectId = unObjectId;
		m_ulPropertyContainer = vr::VRProperties()->TrackedDeviceToPropertyContainer(m_unObjectId);

		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_TrackingSystemName_String, Settings::Instance().mTrackingSystemName.c_str());
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_ModelNumber_String, Settings::Instance().mModelNumber.c_str());
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_ManufacturerName_String, Settings::Instance().mManufacturerName.c_str());
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_RenderModelName_String, Settings::Instance().mRenderModelName.c_str());
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_RegisteredDeviceType_String, Settings::Instance().mRegisteredDeviceType.c_str());
		vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_UserIpdMeters_Float, Settings::Instance().m_flIPD);
		vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_UserHeadToEyeDepthMeters_Float, 0.f);
		vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_DisplayFrequency_Float, static_cast<float>(Settings::Instance().m_refreshRate));
		vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_SecondsFromVsyncToPhotons_Float, Settings::Instance().m_flSecondsFromVsyncToPhotons);

		// return a constant that's not 0 (invalid) or 1 (reserved for Oculus)
		vr::VRProperties()->SetUint64Property(m_ulPropertyContainer, vr::Prop_CurrentUniverseId_Uint64, 2);

		// avoid "not fullscreen" warnings from vrmonitor
		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_IsOnDesktop_Bool, false);

		// Manually send VSync events on direct mode. ref:https://github.com/ValveSoftware/virtual_display/issues/1
		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_DriverDirectModeSendsVsyncEvents_Bool, true);

		float originalIPD = vr::VRSettings()->GetFloat(vr::k_pch_SteamVR_Section, vr::k_pch_SteamVR_IPD_Float);
		vr::VRSettings()->SetFloat(vr::k_pch_SteamVR_Section, vr::k_pch_SteamVR_IPD_Float, Settings::Instance().m_flIPD);

		m_D3DRender = std::make_shared<CD3DRender>();

		// Use the same adapter as vrcompositor uses. If another adapter is used, vrcompositor says "failed to open shared texture" and then crashes.
		// It seems vrcompositor selects always(?) first adapter. vrcompositor may use Intel iGPU when user sets it as primary adapter. I don't know what happens on laptop which support optimus.
		// Prop_GraphicsAdapterLuid_Uint64 is only for redirect display and is ignored on direct mode driver. So we can't specify an adapter for vrcompositor.
		// m_nAdapterIndex is set 0 on the launcher.
		if (!m_D3DRender->Initialize(Settings::Instance().m_nAdapterIndex))
		{
			FatalLog(L"Could not create graphics device for adapter %d.  Requires a minimum of two graphics cards.", Settings::Instance().m_nAdapterIndex);
			return vr::VRInitError_Driver_Failed;
		}

		int32_t nDisplayAdapterIndex;
		if (!m_D3DRender->GetAdapterInfo(&nDisplayAdapterIndex, m_adapterName))
		{
			FatalLog(L"Failed to get primary adapter info!");
			return vr::VRInitError_Driver_Failed;
		}

		Log(L"Using %s as primary graphics adapter.", m_adapterName.c_str());
		Log(L"OSVer: %s", GetWindowsOSVersion().c_str());

		// Spin up a separate thread to handle the overlapped encoding/transmit step.
		m_encoder = std::make_shared<CEncoder>();
		try {
			m_encoder->Initialize(m_D3DRender, m_Listener);
		}
		catch (Exception e) {
			FatalLog(L"Failed to initialize CEncoder. %s", e.what());
			return vr::VRInitError_Driver_Failed;
		}
		m_encoder->Start();

		if (Settings::Instance().m_enableSound) {
			m_audioCapture = std::make_shared<AudioCapture>(m_Listener);
			try {
				m_audioCapture->Start(ToWstring(Settings::Instance().m_soundDevice));
			}
			catch (Exception e) {
				FatalLog(L"Failed to start audio capture. %s", e.what());
				return vr::VRInitError_Driver_Failed;
			}
		}

		m_VSyncThread = std::make_shared<VSyncThread>(Settings::Instance().m_refreshRate);
		m_VSyncThread->Start();

		m_recenterManager = std::make_shared<RecenterManager>();

		m_displayComponent = std::make_shared<DisplayComponent>();
		m_directModeComponent = std::make_shared<DirectModeComponent>(m_D3DRender, m_encoder, m_Listener, m_recenterManager);

		mActivated = true;

		return vr::VRInitError_None;
	}

	virtual void Deactivate() override
	{
		Log(L"CRemoteHmd Deactivate");
		mActivated = false;
		m_unObjectId = vr::k_unTrackedDeviceIndexInvalid;
	}

	virtual void EnterStandby() override
	{
	}

	void *GetComponent(const char *pchComponentNameAndVersion) override
	{
		Log(L"GetComponent %hs", pchComponentNameAndVersion);
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

		if (m_recenterManager->HasValidTrackingInfo()) {
			pose.qRotation = m_recenterManager->GetRecenteredHMD();

			TrackingVector3 position = m_recenterManager->GetRecenteredPositionHMD();
			pose.vecPosition[0] = position.x;
			pose.vecPosition[1] = position.y;
			pose.vecPosition[2] = position.z;

			Log(L"GetPose: Rotation=(%f, %f, %f, %f) Position=(%f, %f, %f)",
				pose.qRotation.x,
				pose.qRotation.y,
				pose.qRotation.z,
				pose.qRotation.w,
				pose.vecPosition[0],
				pose.vecPosition[1],
				pose.vecPosition[2]
			);

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
			//Log(L"RunFrame");
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
			char buf[4000];
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
				"%s %d\n"
				"%s %d\n"
				"GPU %s\n"
				"Codec %d\n"
				"Bitrate %lluMbps\n"
				"Resolution %dx%d\n"
				"RefreshRate %d\n"
				, m_Listener->DumpConfig().c_str()
				, k_pch_Settings_DebugLog_Bool, Settings::Instance().m_DebugLog
				, k_pch_Settings_DebugFrameIndex_Bool, Settings::Instance().m_DebugFrameIndex
				, k_pch_Settings_DebugFrameOutput_Bool, Settings::Instance().m_DebugFrameOutput
				, k_pch_Settings_DebugCaptureOutput_Bool, Settings::Instance().m_DebugCaptureOutput
				, k_pch_Settings_UseKeyedMutex_Bool, Settings::Instance().m_UseKeyedMutex
				, k_pch_Settings_ControllerTriggerMode_Int32, Settings::Instance().m_controllerTriggerMode
				, k_pch_Settings_ControllerTrackpadClickMode_Int32, Settings::Instance().m_controllerTrackpadClickMode
				, k_pch_Settings_ControllerTrackpadTouchMode_Int32, Settings::Instance().m_controllerTrackpadTouchMode
				, k_pch_Settings_ControllerBackMode_Int32, Settings::Instance().m_controllerBackMode
				, k_pch_Settings_ControllerRecenterButton_Int32, Settings::Instance().m_controllerRecenterButton
				, ToUTF8(m_adapterName).c_str() // TODO: Proper treatment of UNICODE. Sanitizing.
				, Settings::Instance().m_codec
				, Settings::Instance().mEncodeBitrate.toMiBits()
				, Settings::Instance().m_renderWidth, Settings::Instance().m_renderHeight
				, Settings::Instance().m_refreshRate
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
				else if (name == k_pch_Settings_ControllerBackMode_Int32) {
					Settings::Instance().m_controllerBackMode = atoi(args.substr(index + 1).c_str());
				}
				else if (name == k_pch_Settings_ControllerRecenterButton_Int32) {
					Settings::Instance().m_controllerRecenterButton = atoi(args.substr(index + 1).c_str());
				}
				else if (name == "causePacketLoss") {
					Settings::Instance().m_causePacketLoss = atoi(args.substr(index + 1).c_str());
				}
				else if (name == "trackingFrameOffset") {
					Settings::Instance().m_trackingFrameOffset = atoi(args.substr(index + 1).c_str());
				}
				else if (name == "captureLayerDDS") {
					Settings::Instance().m_captureLayerDDSTrigger = atoi(args.substr(index + 1).c_str());
				}
				else if (name == "captureComposedDDS") {
					Settings::Instance().m_captureComposedDDSTrigger = atoi(args.substr(index + 1).c_str());
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
			Log(L"Invalid control command: %hs", commandName.c_str());
			m_Listener->SendCommandResponse("NG\n");
		}
		
	}

	void OnPoseUpdated() {
		if (m_unObjectId != vr::k_unTrackedDeviceIndexInvalid)
		{
			if (!m_Listener->HasValidTrackingInfo()) {
				return;
			}
			if (!m_added || !mActivated) {
				return;
			}

			TrackingInfo info;
			m_Listener->GetTrackingInfo(info);

			m_recenterManager->OnPoseUpdated(info, m_Listener.get());
			m_directModeComponent->OnPoseUpdated(info);
			
			vr::VRServerDriverHost()->TrackedDevicePoseUpdated(m_unObjectId, GetPose(), sizeof(vr::DriverPose_t));

			if (m_trackingReference) {
				m_trackingReference->OnPoseUpdated();
			}
		}
	}

	void OnNewClient() {
	}

	void OnStreamStart() {
		if (!m_added || !mActivated) {
			return;
		}
		Log(L"OnStreamStart()");
		// Insert IDR frame for faster startup of decoding.
		m_encoder->OnStreamStart();
	}

	void OnPacketLoss() {
		if (!m_added || !mActivated) {
			return;
		}
		Log(L"OnPacketLoss()");
		m_encoder->OnPacketLoss();
	}

	void OnShutdown() {
		if (!m_added || !mActivated) {
			return;
		}
		Log(L"Sending shutdown signal to vrserver.");
		vr::VREvent_Reserved_t data = { 0, 0 };
		vr::VRServerDriverHost()->VendorSpecificEvent(m_unObjectId, vr::VREvent_DriverRequestedQuit, (vr::VREvent_Data_t&)data, 0);
	}
private:
	bool m_added;
	bool mActivated;
	vr::TrackedDeviceIndex_t m_unObjectId;
	vr::PropertyContainerHandle_t m_ulPropertyContainer;

	std::wstring m_adapterName;

	std::shared_ptr<CD3DRender> m_D3DRender;
	std::shared_ptr<CEncoder> m_encoder;
	std::shared_ptr<AudioCapture> m_audioCapture;
	std::shared_ptr<Listener> m_Listener;
	std::shared_ptr<VSyncThread> m_VSyncThread;
	std::shared_ptr<RecenterManager> m_recenterManager;

	std::shared_ptr<DisplayComponent> m_displayComponent;
	std::shared_ptr<DirectModeComponent> m_directModeComponent;

	std::shared_ptr<TrackingReference> m_trackingReference;
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
		FatalLog(L"ALVR Server driver is installed on multiple locations. This causes some issues.\r\n"
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
	InitCrashHandler();

	Log(L"HmdDriverFactory %hs (%hs)", pInterfaceName, vr::IServerTrackedDeviceProvider_Version);
	if ( 0 == strcmp( vr::IServerTrackedDeviceProvider_Version, pInterfaceName ) )
	{
		Log(L"HmdDriverFactory server return");
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