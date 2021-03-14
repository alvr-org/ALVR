#pragma once

#include <memory>
#include "shared/d3drender.h"
#include "alvr_server/ClientConnection.h"
#include "VideoEncoder.h"
#include "NvEncoderD3D11.h"
#include "shared/ipctools.h"

// Video encoder for NVIDIA NvEnc.
class VideoEncoderNVENC : public VideoEncoder
{
public:
	VideoEncoderNVENC(std::shared_ptr<CD3DRender> pD3DRender
		, std::shared_ptr<ClientConnection> listener
		, int width, int height);
	~VideoEncoderNVENC();

	void Initialize();
	void Shutdown();

	void Transmit(ID3D11Texture2D *pTexture, uint64_t presentationTime, uint64_t frameIndex, uint64_t frameIndex2, uint64_t clientTime, bool insertIDR);
private:
	void FillEncodeConfig(NV_ENC_INITIALIZE_PARAMS &initializeParams, int refreshRate, int renderWidth, int renderHeight, uint64_t bitrateBits);


	std::ofstream fpOut;
	std::shared_ptr<NvEncoder> m_NvNecoder;

	std::shared_ptr<CD3DRender> m_pD3DRender;
	int m_nFrame;

	std::shared_ptr<ClientConnection> m_Listener;

	bool mSupportsReferenceFrameInvalidation = false;

	int m_codec;
	int m_refreshRate;
	int m_renderWidth;
	int m_renderHeight;
	int m_bitrateInMBits;
};
