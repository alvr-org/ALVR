#ifdef ALVR_GPL

#pragma once

#include <wrl.h>

#include "shared/d3drender.h"
#include "VideoEncoder.h"
#include "ALVR-common/packet_types.h"

extern "C" {
	#include <libavutil/avutil.h>
	#include <libavcodec/avcodec.h>
	#include <libavformat/avformat.h>
	#include <libswscale/swscale.h>
}

using Microsoft::WRL::ComPtr;

// Software video encoder using FFMPEG
class VideoEncoderSW : public VideoEncoder
{
public:
	VideoEncoderSW(std::shared_ptr<CD3DRender> pD3DRender
		, int width, int height);
	~VideoEncoderSW();

	void Initialize();
	void Shutdown();

	static void LibVALog(void*, int level, const char* data, va_list va);

	AVCodecID ToFFMPEGCodec(ALVR_CODEC codec);

	void Transmit(ID3D11Texture2D *pTexture, uint64_t presentationTime, uint64_t targetTimestampNs, bool insertIDR);
	HRESULT SetupStagingTexture(ID3D11Texture2D *pTexture);
	HRESULT CopyTexture(ID3D11Texture2D *pTexture);
private:
    std::shared_ptr<CD3DRender> m_d3dRender;

	AVCodecContext *m_codecContext;
	AVFrame *m_transferredFrame, *m_encoderFrame;
	SwsContext *m_scalerContext = nullptr;

	ComPtr<ID3D11Texture2D> m_stagingTex;
	D3D11_TEXTURE2D_DESC m_stagingTexDesc;
	D3D11_MAPPED_SUBRESOURCE m_stagingTexMap;

    ALVR_CODEC m_codec;
	int m_refreshRate;
	int m_renderWidth;
	int m_renderHeight;
	int m_bitrateInMBits;
};

#endif // ALVR_GPL