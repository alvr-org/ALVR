//===================== Copyright (c) Valve Corporation. All Rights Reserved. ======================
//
// Example OpenVR driver for demonstrating IVRVirtualDisplay interface.
//
//==================================================================================================

#include "FrameEncoder.h"
#include "VideoEncoderNVENC.h"
#include "VideoEncoderVCE.h"

FrameEncoder::FrameEncoder()
	: mExiting(false)
	, mTrackingFrameIndex(0)
	, mVideoFrameIndex(0)
{
	mEncodeFinished.Set();
}


FrameEncoder::~FrameEncoder()
{
	if (mVideoEncoder)
	{
		mVideoEncoder->Shutdown();
		mVideoEncoder.reset();
	}
}

void FrameEncoder::Initialize(std::shared_ptr<CD3DRender> d3dRender, std::shared_ptr<Listener> listener) {
	mFrameRender = std::make_shared<FrameRender>(d3dRender);
	mListener = listener;

	Exception vceException;
	Exception nvencException;
	try {
		Log(L"Try to use VideoEncoderVCE.");
		mVideoEncoder = std::make_shared<VideoEncoderVCE>(d3dRender, listener);
		mVideoEncoder->Initialize();
		return;
	}
	catch (Exception e) {
		vceException = e;
	}
	try {
		Log(L"Try to use VideoEncoderNVENC.");
		mVideoEncoder = std::make_shared<VideoEncoderNVENC>(d3dRender, listener
			, ShouldUseNV12Texture());
		mVideoEncoder->Initialize();
		return;
	}
	catch (Exception e) {
		nvencException = e;
	}
	throw MakeException(L"All VideoEncoder are not available. VCE: %s, NVENC: %s", vceException.what(), nvencException.what());
}

bool FrameEncoder::CopyToStaging(ID3D11Texture2D * pTexture[][2], vr::VRTextureBounds_t bounds[][2], int layerCount, bool recentering, uint64_t presentationTime, uint64_t frameIndex, uint64_t clientTime, const std::string & message, const std::string & debugText)
{
	uint64_t firstVideoFrameInBuffer;
	// We can buffer only 2-frames in throttling buffer.
	if (mListener->GetFirstBufferedFrame(&firstVideoFrameInBuffer) && firstVideoFrameInBuffer + 2 <= mVideoFrameIndex) {
		Log(L"Drop frame because of large throttling buffer. firstVideoFrameInBuffer=%llu Current=%llu", firstVideoFrameInBuffer, mVideoFrameIndex);
		return true;
	}
	if (!mScheduler.CanEncodeFrame()) {
		Log(L"Skipping encode because of sending IDR or not streaming.");
		return true;
	}
	mPresentationTime = presentationTime;
	mTrackingFrameIndex = frameIndex;
	mClientTime = clientTime;
	mFrameRender->Startup();

	char buf[200];
	snprintf(buf, sizeof(buf), "\nvfindex: %llu", mVideoFrameIndex);

	mFrameRender->RenderFrame(pTexture, bounds, layerCount, recentering, message, debugText + buf);
	return true;
}

void FrameEncoder::Run()
{
	Log(L"CEncoder: Start thread. Id=%d", GetCurrentThreadId());
	SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_MOST_URGENT);

	while (!mExiting)
	{
		Log(L"CEncoder: Waiting for new frame...");

		mNewFrameReady.Wait();
		if (mExiting)
			break;

		if (mFrameRender->GetTexture())
		{
			mVideoEncoder->Transmit(mFrameRender->GetTexture().Get(), mPresentationTime, mVideoFrameIndex, mTrackingFrameIndex, mClientTime, mScheduler.CheckIDRInsertion());
			mVideoFrameIndex++;
		}

		mEncodeFinished.Set();
	}
}

void FrameEncoder::Stop()
{
	mExiting = true;
	mNewFrameReady.Set();
	Join();
	mFrameRender.reset();
}

void FrameEncoder::NewFrameReady()
{
	Log(L"New Frame Ready");
	mEncodeFinished.Reset();
	mNewFrameReady.Set();
}

void FrameEncoder::WaitForEncode()
{
	mEncodeFinished.Wait();
}

void FrameEncoder::OnStreamStart() {
	mScheduler.OnStreamStart();
}

void FrameEncoder::OnFrameAck(bool result, bool isIDR, uint64_t startFrame, uint64_t endFrame) {
	Log(L"OnFrameAck: result=%d isIDR=%d VideoFrame=%llu-%llu", result, isIDR, startFrame, endFrame);
	mScheduler.OnFrameAck(result, isIDR);
	if (!result && !isIDR) {
		if (mVideoEncoder->SupportsReferenceFrameInvalidation()) {
			if (startFrame + 16 < mVideoFrameIndex || mVideoFrameIndex < endFrame) {
				Log(L"Invalid reference frame for invalidation. %llu - %llu CurrentVideoFrameIndex=%llu",
					startFrame, endFrame, mVideoFrameIndex);
				// Fallback to IDR frame insertion.
				mScheduler.OnPacketLoss();
				return;
			}
			for (uint64_t videoFrameIndex = startFrame;
				videoFrameIndex <= endFrame; videoFrameIndex++) {
				mVideoEncoder->InvalidateReferenceFrame(videoFrameIndex);
			}
		}
		else {
			mScheduler.OnPacketLoss();
		}
	}
}

void FrameEncoder::Reconfigure(int refreshRate, int renderWidth, int renderHeight, Bitrate bitrate) {
	mVideoEncoder->Reconfigure(refreshRate, renderWidth, renderHeight, bitrate);
}
