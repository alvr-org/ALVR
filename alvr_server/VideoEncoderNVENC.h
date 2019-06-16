#pragma once

#include <memory>
#include "d3drender.h"
#include "Listener.h"
#include "VideoEncoder.h"
#include "nvenc\NvTextureEncoderD3D11.h"
#include "..\CUDA\NvTextureEncoderCuda.h"
#include "ipctools.h"

// Video encoder for NVIDIA NvEnc.
class VideoEncoderNVENC : public VideoEncoder
{
public:
	VideoEncoderNVENC(std::shared_ptr<CD3DRender> pD3DRender
		, std::shared_ptr<Listener> listener, bool useNV12);
	~VideoEncoderNVENC();

	void Initialize();
	void Reconfigure(int refreshRate, int renderWidth, int renderHeight, int bitrateInMBit);
	void Shutdown();

	void Transmit(ID3D11Texture2D *pTexture, uint64_t presentationTime, uint64_t frameIndex, uint64_t frameIndex2, uint64_t clientTime, bool insertIDR);
private:
	std::ofstream mOutput;
	std::shared_ptr<INvTextureEncoder> mEncoder;

	std::shared_ptr<CD3DRender> mD3DRender;
	int mFrame;

	std::shared_ptr<Listener> mListener;

	const bool mUseNV12;

	int mCodec;
	int mRefreshRate;
	int mRenderWidth;
	int mRenderHeight;
	int mBitrateInMBits;
};