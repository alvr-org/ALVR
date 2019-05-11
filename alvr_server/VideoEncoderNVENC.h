#pragma once

#include <memory>
#include "d3drender.h"
#include "Listener.h"
#include "VideoEncoder.h"
#include "NvEncoderD3D11.h"
#include "NvEncoderCuda.h"
#include "CudaConverter.h"
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
	std::ofstream fpOut;
	std::shared_ptr<NvEncoder> m_NvNecoder;

	std::shared_ptr<CD3DRender> m_pD3DRender;
	int m_nFrame;

	std::shared_ptr<Listener> m_Listener;

	const bool m_useNV12;
	std::shared_ptr<CudaConverter> m_Converter;

	int m_codec;
	int m_refreshRate;
	int m_renderWidth;
	int m_renderHeight;
	int m_bitrateInMBits;
};