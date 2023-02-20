#pragma once

#include <memory>
#include "shared/d3drender.h"
#include "VideoEncoder.h"
#include "NvEncoderD3D11.h"

enum AdaptiveQuantizationMode {
	SpatialAQ = 1,
	TemporalAQ = 2
};

// Video encoder for NVIDIA NvEnc.
class VideoEncoderNVENC : public VideoEncoder
{
public:
	VideoEncoderNVENC(std::shared_ptr<CD3DRender> pD3DRender
		, int width, int height);
	~VideoEncoderNVENC();

	void Initialize();
	void Shutdown();

	void Transmit(ID3D11Texture2D *pTexture, uint64_t presentationTime, uint64_t targetTimestampNs, bool insertIDR);
private:
	void FillEncodeConfig(NV_ENC_INITIALIZE_PARAMS &initializeParams, int refreshRate, int renderWidth, int renderHeight, uint64_t bitrate_bps);


	std::ofstream fpOut;
	std::shared_ptr<NvEncoder> m_NvNecoder;

	std::shared_ptr<CD3DRender> m_pD3DRender;

	int m_codec;
	int m_refreshRate;
	int m_renderWidth;
	int m_renderHeight;
	int m_bitrateInMBits;
};
