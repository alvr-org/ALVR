//===================== Copyright (c) Valve Corporation. All Rights Reserved. ======================
//
// Example OpenVR driver for demonstrating IVRVirtualDisplay interface.
//
//==================================================================================================

#pragma once

#include <memory>
#include "openvr-utils/threadtools.h"
#include "openvr-utils/d3drender.h"
#include "Listener.h"
#include "VideoEncoder.h"
#include "FrameRender.h"
#include "IDRScheduler.h"

class FrameEncoder : public CThread
{
public:
	FrameEncoder();
	~FrameEncoder();

	void Initialize(std::shared_ptr<CD3DRender> d3dRender, std::shared_ptr<Listener> listener);
	bool CopyToStaging(ID3D11Texture2D *pTexture[][2], vr::VRTextureBounds_t bounds[][2], int layerCount, bool recentering
		, uint64_t presentationTime, uint64_t frameIndex, uint64_t clientTime, const std::string& message, const std::string& debugText);
	void Run() override;
	void Stop();
	void NewFrameReady();
	void WaitForEncode();
	void OnStreamStart();
	void OnFrameAck(bool result, bool isIDR, uint64_t startFrame, uint64_t endFrame);
	void Reconfigure(int refreshRate, int renderWidth, int renderHeight, Bitrate bitrate);

private:
	CThreadEvent mNewFrameReady, mEncodeFinished;
	std::shared_ptr<VideoEncoder> mVideoEncoder;
	std::shared_ptr<Listener> mListener;
	bool mExiting;
	uint64_t mPresentationTime;
	uint64_t mTrackingFrameIndex;
	uint64_t mClientTime;

	uint64_t mVideoFrameIndex;

	std::shared_ptr<FrameRender> mFrameRender;

	IDRScheduler mScheduler;
};

