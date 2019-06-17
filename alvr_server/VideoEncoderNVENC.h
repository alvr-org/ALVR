#pragma once

#include <memory>
#include "openvr-utils\d3drender.h"
#include "openvr-utils\ipctools.h"
#include "Listener.h"
#include "Bitrate.h"
#include "VideoEncoder.h"
#include "nvenc\NvTextureEncoderD3D11.h"
#include "..\CUDA\NvTextureEncoderCuda.h"

// Video encoder for NVIDIA NvEnc.
class VideoEncoderNVENC : public VideoEncoder
{
public:
	VideoEncoderNVENC(std::shared_ptr<CD3DRender> pD3DRender
		, std::shared_ptr<Listener> listener, bool useNV12);
	~VideoEncoderNVENC();

	bool SupportsReferenceFrameInvalidation() { return mSupportsReferenceFrameInvalidation; };

	void Initialize();
	void Reconfigure(int refreshRate, int renderWidth, int renderHeight, Bitrate bitrate);
	void Shutdown();

	void Transmit(ID3D11Texture2D *pTexture, uint64_t presentationTime, uint64_t videoFrameIndex, uint64_t trackingFrameIndex, uint64_t clientTime, bool insertIDR);
	void InvalidateReferenceFrame(uint64_t videoFrameIndex);
private:
	std::ofstream mOutput;
	std::shared_ptr<NvTextureEncoder> mEncoder;

	std::shared_ptr<CD3DRender> mD3DRender;
	int mFrame;

	std::shared_ptr<Listener> mListener;

	const bool mUseNV12;

	int mCodec;
	int mRefreshRate;
	int mRenderWidth;
	int mRenderHeight;
	Bitrate mBitrate;
	bool mSupportsReferenceFrameInvalidation = false;

	void FillEncodeConfig(NV_ENC_INITIALIZE_PARAMS &initializeParams, int refreshRate, int renderWidth, int renderHeight, Bitrate bitrate);
};