//===================== Copyright (c) Valve Corporation. All Rights Reserved. ======================
//
// Example OpenVR driver for demonstrating IVRVirtualDisplay interface.
//
//==================================================================================================

#include "OpenVRDirectModeComponent.h"

#include <ScreenGrab.h>

extern uint64_t gDriverTestMode;

OpenVRDirectModeComponent::OpenVRDirectModeComponent(std::shared_ptr<CD3DRender> pD3DRender, std::shared_ptr<FrameEncoder> pEncoder, std::shared_ptr<Listener> Listener, std::shared_ptr<RecenterManager> recenterManager)
	: mD3DRender(pD3DRender)
	, mEncoder(pEncoder)
	, mListener(Listener)
	, mRecenterManager(recenterManager)
	, mPoseMutex(NULL)
	, mSubmitLayer(0)
	, mLastReferencedFrameIndex(0)
	, mLastReferencedClientTime(0) {
}

void OpenVRDirectModeComponent::OnPoseUpdated(TrackingInfo & info) {
	// Put pose history buffer
	TrackingHistoryFrame history;
	history.info = info;

	vr::HmdQuaternion_t recentered = mRecenterManager->GetRecenteredHMD();
	HmdMatrix_QuatToMat(recentered.w,
		recentered.x,
		recentered.y,
		recentered.z,
		&history.rotationMatrix);

	Log(L"Rotation Matrix=(%f, %f, %f, %f) (%f, %f, %f, %f) (%f, %f, %f, %f)"
		, history.rotationMatrix.m[0][0], history.rotationMatrix.m[0][1], history.rotationMatrix.m[0][2], history.rotationMatrix.m[0][3]
		, history.rotationMatrix.m[1][0], history.rotationMatrix.m[1][1], history.rotationMatrix.m[1][2], history.rotationMatrix.m[1][3]
		, history.rotationMatrix.m[2][0], history.rotationMatrix.m[2][1], history.rotationMatrix.m[2][2], history.rotationMatrix.m[2][3]);

	mPoseMutex.Wait(INFINITE);
	if (mPoseBuffer.size() == 0) {
		mPoseBuffer.push_back(history);
	}
	else {
		if (mPoseBuffer.back().info.FrameIndex != info.FrameIndex) {
			// New track info
			mPoseBuffer.push_back(history);
		}
	}
	if (mPoseBuffer.size() > 10) {
		mPoseBuffer.pop_front();
	}
	mPoseMutex.Release();

	mLastReferencedFrameIndex = info.FrameIndex;
	mLastReferencedClientTime = info.clientTime;
}

/** Specific to Oculus compositor support, textures supplied must be created using this method. */

void OpenVRDirectModeComponent::CreateSwapTextureSet(uint32_t unPid, const SwapTextureSetDesc_t * pSwapTextureSetDesc, vr::SharedTextureHandle_t(*pSharedTextureHandles)[3])
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
		HRESULT hr = mD3DRender->GetDevice()->CreateTexture2D(&SharedTextureDesc, NULL, &processResource->textures[i]);
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
void OpenVRDirectModeComponent::DestroySwapTextureSet(vr::SharedTextureHandle_t sharedTextureHandle)
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
void OpenVRDirectModeComponent::DestroyAllSwapTextureSets(uint32_t unPid)
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
void OpenVRDirectModeComponent::GetNextSwapTextureSetIndex(vr::SharedTextureHandle_t sharedTextureHandles[2], uint32_t(*pIndices)[2])
{
	Log(L"GetNextSwapTextureSetIndex %p %p %d %d", sharedTextureHandles[0], sharedTextureHandles[1], (*pIndices)[0], (*pIndices)[1]);
	(*pIndices)[0]++;
	(*pIndices)[0] %= 3;
	(*pIndices)[1]++;
	(*pIndices)[1] %= 3;
}

/** Call once per layer to draw for this frame.  One shared texture handle per eye.  Textures must be created
* using CreateSwapTextureSet and should be alternated per frame.  Call Present once all layers have been submitted. */
void OpenVRDirectModeComponent::SubmitLayer(const SubmitLayerPerEye_t(&perEye)[2], const vr::HmdMatrix34_t * pPose)
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

	if (mSubmitLayer == 0) {
		// Detect FrameIndex of submitted frame by pPose.
		// This is important part to achieve smooth headtracking.
		// We search for history of TrackingInfo and find the TrackingInfo which have nearest matrix value.

		mPoseMutex.Wait(INFINITE);
		float minDiff = 100000;
		int index = 0;
		int minIndex = 0;
		auto minIt = mPoseBuffer.begin();
		for (auto it = mPoseBuffer.begin(); it != mPoseBuffer.end(); it++, index++) {
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
		if (minIt != mPoseBuffer.end()) {
			// found the frameIndex
			mPrevSubmitFrameIndex = mSubmitFrameIndex;
			mPrevSubmitClientTime = mSubmitClientTime;
			mSubmitFrameIndex = minIt->info.FrameIndex;
			mSubmitClientTime = minIt->info.clientTime;

			mPrevFramePoseRotation = mFramePoseRotation;
			mFramePoseRotation.x = minIt->info.HeadPose_Pose_Orientation.x;
			mFramePoseRotation.y = minIt->info.HeadPose_Pose_Orientation.y;
			mFramePoseRotation.z = minIt->info.HeadPose_Pose_Orientation.z;
			mFramePoseRotation.w = minIt->info.HeadPose_Pose_Orientation.w;

			Log(L"Frame pose found. mPrevSubmitFrameIndex=%llu mSubmitFrameIndex=%llu minDiff=%f", mPrevSubmitFrameIndex, mSubmitFrameIndex, minDiff);
		}
		else {
			mSubmitFrameIndex = 0;
			mSubmitClientTime = 0;
			mFramePoseRotation = HmdQuaternion_Init(0.0, 0.0, 0.0, 0.0);
		}
		mPoseMutex.Release();
	}
	if (mSubmitLayer < MAX_LAYERS) {
		mSubmitLayers[mSubmitLayer][0] = perEye[0];
		mSubmitLayers[mSubmitLayer][1] = perEye[1];
		mSubmitLayer++;
	}
	else {
		Log(L"Too many layers submitted!");
	}

	if (gDriverTestMode & 8) {
		// Crash test
		*(char*)0 = 1;
	}

	//CopyTexture();
}

/** Submits queued layers for display. */
void OpenVRDirectModeComponent::Present(vr::SharedTextureHandle_t syncTexture)
{
	bool useMutex = Settings::Instance().mUseKeyedMutex;
	Log(L"Present syncTexture=%p (use:%d) mPrevSubmitFrameIndex=%llu mSubmitFrameIndex=%llu", syncTexture, useMutex, mPrevSubmitFrameIndex, mSubmitFrameIndex);

	IDXGIKeyedMutex *pKeyedMutex = NULL;

	uint32_t layerCount = mSubmitLayer;
	mSubmitLayer = 0;

	if (mPrevSubmitFrameIndex == mSubmitFrameIndex) {
		Log(L"Discard duplicated frame. FrameIndex=%llu (Ignoring)", mSubmitFrameIndex);
		//return;
	}

	if (!mListener->IsStreaming()) {
		Log(L"Discard frame because isStreaming=false. FrameIndex=%llu", mSubmitFrameIndex);
		return;
	}

	ID3D11Texture2D *pSyncTexture = mD3DRender->GetSharedTexture((HANDLE)syncTexture);
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

	mEncoder->NewFrameReady();
}

void OpenVRDirectModeComponent::CopyTexture(uint32_t layerCount) {

	uint64_t presentationTime = GetTimestampUs();

	ID3D11Texture2D *pTexture[MAX_LAYERS][2];
	ComPtr<ID3D11Texture2D> Texture[MAX_LAYERS][2];
	vr::VRTextureBounds_t bounds[MAX_LAYERS][2];

	for (uint32_t i = 0; i < layerCount; i++) {
		// Find left eye texture.
		HANDLE leftEyeTexture = (HANDLE)mSubmitLayers[i][0].hTexture;
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
			HANDLE rightEyeTexture = (HANDLE)mSubmitLayers[i][1].hTexture;
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
		bounds[i][0] = mSubmitLayers[i][0].bounds;
		bounds[i][1] = mSubmitLayers[i][1].bounds;
	}

	// This can go away, but is useful to see it as a separate packet on the gpu in traces.
	mD3DRender->GetContext()->Flush();

	Log(L"Waiting for finish of previous encode.");

	if (Settings::Instance().mCaptureLayerDDSTrigger) {
		wchar_t buf[1000];

		for (uint32_t i = 0; i < layerCount; i++) {
			Log(L"Writing Debug DDS. mLastReferencedFrameIndex=%llu layer=%d/%d", 0, i, layerCount);
			_snwprintf_s(buf, sizeof(buf), L"%hs\\debug-%llu-%d-%d.dds", Settings::Instance().mDebugOutputDir.c_str(), mSubmitFrameIndex, i, layerCount);
			HRESULT hr = DirectX::SaveDDSTextureToFile(mD3DRender->GetContext(), pTexture[i][0], buf);
			Log(L"Writing Debug DDS: End hr=%p %s", hr, GetErrorStr(hr).c_str());
		}
		Settings::Instance().mCaptureLayerDDSTrigger = false;
	}

	// Wait for the encoder to be ready.  This is important because the encoder thread
	// blocks on transmit which uses our shared d3d context (which is not thread safe).
	mEncoder->WaitForEncode();

	std::string debugText;

	if (Settings::Instance().mDebugFrameIndex) {
		TrackingInfo info;
		mListener->GetTrackingInfo(info);

		char buf[2000];
		snprintf(buf, sizeof(buf), "%llu\n%f\n%f", mPrevSubmitFrameIndex, mPrevFramePoseRotation.x, info.HeadPose_Pose_Orientation.x);
		debugText = buf;
	}

	uint64_t submitFrameIndex = mSubmitFrameIndex + Settings::Instance().mTrackingFrameOffset;
	Log(L"Fix frame index. FrameIndex=%llu Offset=%d New FrameIndex=%llu"
		, mSubmitFrameIndex, Settings::Instance().mTrackingFrameOffset, submitFrameIndex);

	// Copy entire texture to staging so we can read the pixels to send to remote device.
	mEncoder->CopyToStaging(pTexture, bounds, layerCount, mRecenterManager->IsRecentering(), presentationTime, submitFrameIndex, mSubmitClientTime, mRecenterManager->GetFreePIEMessage(), debugText);

	mD3DRender->GetContext()->Flush();
}
