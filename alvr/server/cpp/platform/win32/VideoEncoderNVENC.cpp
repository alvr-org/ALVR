
#include "VideoEncoderNVENC.h"
#include "NvCodecUtils.h"
#include "alvr_server/nvencoderclioptions.h"

#include "alvr_server/Statistics.h"
#include "alvr_server/Logger.h"
#include "alvr_server/Settings.h"
#include "alvr_server/Utils.h"

VideoEncoderNVENC::VideoEncoderNVENC(std::shared_ptr<CD3DRender> pD3DRender
	, std::shared_ptr<ClientConnection> listener
	, int width, int height)
	: m_pD3DRender(pD3DRender)
	, m_nFrame(0)
	, m_Listener(listener)
	, m_codec(Settings::Instance().m_codec)
	, m_refreshRate(Settings::Instance().m_refreshRate)
	, m_renderWidth(width)
	, m_renderHeight(height)
	, m_bitrateInMBits(Settings::Instance().mEncodeBitrateMBs)
{
	
}

VideoEncoderNVENC::~VideoEncoderNVENC()
{}

void VideoEncoderNVENC::Initialize()
{
	//
	// Initialize Encoder
	//

	NV_ENC_BUFFER_FORMAT format = NV_ENC_BUFFER_FORMAT_ABGR;
	
	if (Settings::Instance().m_use10bitEncoder) {
		format = NV_ENC_BUFFER_FORMAT_ABGR10;
	}

	Debug("Initializing CNvEncoder. Width=%d Height=%d Format=%d\n", m_renderWidth, m_renderHeight, format);

	try {
		m_NvNecoder = std::make_shared<NvEncoderD3D11>(m_pD3DRender->GetDevice(), m_renderWidth, m_renderHeight, format, 0);
	}
	catch (NVENCException e) {
		throw MakeException("NvEnc NvEncoderD3D11 failed. Code=%d %hs\n", e.getErrorCode(), e.what());
	}

	NV_ENC_INITIALIZE_PARAMS initializeParams = { NV_ENC_INITIALIZE_PARAMS_VER };
	NV_ENC_CONFIG encodeConfig = { NV_ENC_CONFIG_VER };
	initializeParams.encodeConfig = &encodeConfig;

	FillEncodeConfig(initializeParams, m_refreshRate, m_renderWidth, m_renderHeight, m_bitrateInMBits * 1'000'000);
	   

	try {
		m_NvNecoder->CreateEncoder(&initializeParams);
	}
	catch (NVENCException e) {
		if (e.getErrorCode() == NV_ENC_ERR_INVALID_PARAM) {
			throw MakeException("This GPU does not support H.265 encoding. (NvEncoderCuda NV_ENC_ERR_INVALID_PARAM)");
		}
		throw MakeException("NvEnc CreateEncoder failed. Code=%d %hs", e.getErrorCode(), e.what());
	}

	Debug("CNvEncoder is successfully initialized.\n");
}

void VideoEncoderNVENC::Shutdown()
{
	std::vector<std::vector<uint8_t>> vPacket;
	if(m_NvNecoder)
		m_NvNecoder->EndEncode(vPacket);

	for (std::vector<uint8_t> &packet : vPacket)
	{
		if (fpOut) {
			fpOut.write(reinterpret_cast<char*>(packet.data()), packet.size());
		}
	}
	if (m_NvNecoder) {
		m_NvNecoder->DestroyEncoder();
		m_NvNecoder.reset();
	}

	Debug("CNvEncoder::Shutdown\n");

	if (fpOut) {
		fpOut.close();
	}
}

void VideoEncoderNVENC::Transmit(ID3D11Texture2D *pTexture, uint64_t presentationTime, uint64_t targetTimestampNs, bool insertIDR)
{
	if (m_Listener) {
		if (m_Listener->GetStatistics()->CheckBitrateUpdated()) {
			m_bitrateInMBits = m_Listener->GetStatistics()->GetBitrate();
			NV_ENC_INITIALIZE_PARAMS initializeParams = { NV_ENC_INITIALIZE_PARAMS_VER };
			NV_ENC_CONFIG encodeConfig = { NV_ENC_CONFIG_VER };
			initializeParams.encodeConfig = &encodeConfig;
			FillEncodeConfig(initializeParams, m_refreshRate, m_renderWidth, m_renderHeight, m_bitrateInMBits * 1'000'000);
			NV_ENC_RECONFIGURE_PARAMS reconfigureParams = { NV_ENC_RECONFIGURE_PARAMS_VER };
			reconfigureParams.reInitEncodeParams = initializeParams;
			m_NvNecoder->Reconfigure(&reconfigureParams);
		}
	}

	std::vector<std::vector<uint8_t>> vPacket;

	const NvEncInputFrame* encoderInputFrame = m_NvNecoder->GetNextInputFrame();

	ID3D11Texture2D *pInputTexture = reinterpret_cast<ID3D11Texture2D*>(encoderInputFrame->inputPtr);
	m_pD3DRender->GetContext()->CopyResource(pInputTexture, pTexture);

	NV_ENC_PIC_PARAMS picParams = {};
	if (insertIDR) {
		Debug("Inserting IDR frame.\n");
		picParams.encodePicFlags = NV_ENC_PIC_FLAG_FORCEIDR;
	}
	m_NvNecoder->EncodeFrame(vPacket, &picParams);

	if (m_Listener) {
		m_Listener->GetStatistics()->EncodeOutput();
	}

	m_nFrame += (int)vPacket.size();
	for (std::vector<uint8_t> &packet : vPacket)
	{
		if (fpOut) {
			fpOut.write(reinterpret_cast<char*>(packet.data()), packet.size());
		}
		if (m_Listener) {
			m_Listener->SendVideo(packet.data(), (int)packet.size(), targetTimestampNs);
		}
	}
}

void VideoEncoderNVENC::FillEncodeConfig(NV_ENC_INITIALIZE_PARAMS &initializeParams, int refreshRate, int renderWidth, int renderHeight, uint64_t bitrateBits)
{
	auto &encodeConfig = *initializeParams.encodeConfig;
	GUID EncoderGUID = m_codec == ALVR_CODEC_H264 ? NV_ENC_CODEC_H264_GUID : NV_ENC_CODEC_HEVC_GUID;

	// According to the docment, NVIDIA Video Encoder (NVENC) Interface 8.1,
	// following configrations are recommended for low latency application:
	// 1. Low-latency high quality preset
	// 2. Rate control mode = CBR
	// 3. Very low VBV buffer size(single frame)
	// 4. No B Frames
	// 5. Infinite GOP length
	// 6. Long term reference pictures
	// 7. Intra refresh
	// 8. Adaptive quantization(AQ) enabled

	GUID preset = NV_ENC_PRESET_LOW_LATENCY_HQ_GUID;
	if (Settings::Instance().m_nvencPreset == 0) {
		preset = NV_ENC_PRESET_LOW_LATENCY_DEFAULT_GUID;
	} else if (Settings::Instance().m_nvencPreset == 1) {
		preset = NV_ENC_PRESET_LOW_LATENCY_HQ_GUID;
	} else if (Settings::Instance().m_nvencPreset == 2) {
		preset = NV_ENC_PRESET_LOW_LATENCY_HP_GUID;
	}
	m_NvNecoder->CreateDefaultEncoderParams(&initializeParams, EncoderGUID, preset);

	initializeParams.encodeWidth = initializeParams.darWidth = renderWidth;
	initializeParams.encodeHeight = initializeParams.darHeight = renderHeight;
	initializeParams.frameRateNum = refreshRate;
	initializeParams.frameRateDen = 1;

	if (Settings::Instance().m_nvencRefreshRate != -1) {
		initializeParams.frameRateNum = Settings::Instance().m_nvencRefreshRate;
	}

	// Use reference frame invalidation to faster recovery from frame loss if supported.
	mSupportsReferenceFrameInvalidation = m_NvNecoder->GetCapabilityValue(EncoderGUID, NV_ENC_CAPS_SUPPORT_REF_PIC_INVALIDATION);
	bool supportsIntraRefresh = m_NvNecoder->GetCapabilityValue(EncoderGUID, NV_ENC_CAPS_SUPPORT_INTRA_REFRESH);
	Debug("VideoEncoderNVENC: SupportsReferenceFrameInvalidation: %d\n", mSupportsReferenceFrameInvalidation);
	Debug("VideoEncoderNVENC: SupportsIntraRefresh: %d\n", supportsIntraRefresh);

	// 16 is recommended when using reference frame invalidation. But it has caused bad visual quality.
	// Now, use 0 (use default).
	uint32_t maxNumRefFrames = 0;
	uint32_t gopLength = NVENC_INFINITE_GOPLENGTH;

	if (Settings::Instance().m_nvencMaxNumRefFrames != -1) {
		maxNumRefFrames = Settings::Instance().m_nvencMaxNumRefFrames;
	}
	if (Settings::Instance().m_nvencGopLength != -1) {
		gopLength = Settings::Instance().m_nvencGopLength;
	}

	if (m_codec == ALVR_CODEC_H264) {
		auto &config = encodeConfig.encodeCodecConfig.h264Config;
		config.repeatSPSPPS = 1;
		//if (supportsIntraRefresh) {
		//	config.enableIntraRefresh = 1;
		//	// Do intra refresh every 10sec.
		//	config.intraRefreshPeriod = refreshRate * 10;
		//	config.intraRefreshCnt = refreshRate;
		//}
		if (Settings::Instance().m_nvencEnableIntraRefresh != -1) {
			config.enableIntraRefresh = Settings::Instance().m_nvencEnableIntraRefresh;
		}
		if (Settings::Instance().m_nvencIntraRefreshPeriod != -1) {
			config.intraRefreshPeriod = Settings::Instance().m_nvencIntraRefreshPeriod;
		}
		if (Settings::Instance().m_nvencIntraRefreshCount != -1) {
			config.intraRefreshCnt = Settings::Instance().m_nvencIntraRefreshCount;
		}

		switch (Settings::Instance().m_entropyCoding) {
			case ALVR_CABAC:
				config.entropyCodingMode = NV_ENC_H264_ENTROPY_CODING_MODE_CABAC;
				break;
			case ALVR_CAVLC:
				config.entropyCodingMode = NV_ENC_H264_ENTROPY_CODING_MODE_CAVLC;
				break;
		}

		config.maxNumRefFrames = maxNumRefFrames;
		config.idrPeriod = gopLength;
	}
	else {
		auto &config = encodeConfig.encodeCodecConfig.hevcConfig;
		config.repeatSPSPPS = 1;
		//if (supportsIntraRefresh) {
		//	config.enableIntraRefresh = 1;
		//	// Do intra refresh every 10sec.
		//	config.intraRefreshPeriod = refreshRate * 10;
		//	config.intraRefreshCnt = refreshRate;
		//}
		if (Settings::Instance().m_nvencEnableIntraRefresh != -1) {
			config.enableIntraRefresh = Settings::Instance().m_nvencEnableIntraRefresh;
		}
		if (Settings::Instance().m_nvencIntraRefreshPeriod != -1) {
			config.intraRefreshPeriod = Settings::Instance().m_nvencIntraRefreshPeriod;
		}
		if (Settings::Instance().m_nvencIntraRefreshCount != -1) {
			config.intraRefreshCnt = Settings::Instance().m_nvencIntraRefreshCount;
		}

		config.maxNumRefFramesInDPB = maxNumRefFrames;
		config.idrPeriod = gopLength;
	}

	// According to the document, NVIDIA Video Encoder Interface 5.0,
	// following configrations are recommended for low latency application:
	// 1. NV_ENC_PARAMS_RC_2_PASS_FRAMESIZE_CAP rate control mode.
	// 2. Set vbvBufferSize and vbvInitialDelay to maxFrameSize.
	// 3. Inifinite GOP length.
	// NV_ENC_PARAMS_RC_2_PASS_FRAMESIZE_CAP also assures maximum frame size,
	// which introduces lower transport latency and fewer packet losses.

	// Disable automatic IDR insertion by NVENC. We need to manually insert IDR when packet is dropped
	// if don't use reference frame invalidation.
	encodeConfig.gopLength = gopLength;
	encodeConfig.frameIntervalP = 1;

	if (Settings::Instance().m_nvencPFrameStrategy != -1) {
		encodeConfig.frameIntervalP = Settings::Instance().m_nvencPFrameStrategy;
	}

	// NV_ENC_PARAMS_RC_CBR_HQ is equivalent to NV_ENC_PARAMS_RC_2_PASS_FRAMESIZE_CAP.
	//encodeConfig.rcParams.rateControlMode = NV_ENC_PARAMS_RC_CBR_LOWDELAY_HQ;// NV_ENC_PARAMS_RC_CBR_HQ;
	switch (Settings::Instance().m_rateControlMode) {
		case ALVR_CBR:
			encodeConfig.rcParams.rateControlMode = NV_ENC_PARAMS_RC_CBR_LOWDELAY_HQ;
			break;
		case ALVR_VBR:
			encodeConfig.rcParams.rateControlMode = NV_ENC_PARAMS_RC_VBR;
			break;
	}
	
	uint32_t maxFrameSize = static_cast<uint32_t>(bitrateBits / refreshRate);
	Debug("VideoEncoderNVENC: maxFrameSize=%d bits\n", maxFrameSize);
	encodeConfig.rcParams.vbvBufferSize = maxFrameSize;
	encodeConfig.rcParams.vbvInitialDelay = maxFrameSize;
	encodeConfig.rcParams.maxBitRate = static_cast<uint32_t>(bitrateBits);
	encodeConfig.rcParams.averageBitRate = static_cast<uint32_t>(bitrateBits);

	if (Settings::Instance().m_nvencRateControlMode != -1) {
		encodeConfig.rcParams.rateControlMode = (NV_ENC_PARAMS_RC_MODE)Settings::Instance().m_nvencRateControlMode;
	}
	if (Settings::Instance().m_nvencRcBufferSize != -1) {
		encodeConfig.rcParams.vbvBufferSize = Settings::Instance().m_nvencRcBufferSize;
	}
	if (Settings::Instance().m_nvencRcInitialDelay != -1) {
		encodeConfig.rcParams.vbvInitialDelay = Settings::Instance().m_nvencRcInitialDelay;
	}
	if (Settings::Instance().m_nvencRcMaxBitrate != -1) {
		encodeConfig.rcParams.maxBitRate = Settings::Instance().m_nvencRcMaxBitrate;
	}
	if (Settings::Instance().m_nvencRcAverageBitrate != -1) {
		encodeConfig.rcParams.averageBitRate = Settings::Instance().m_nvencRcAverageBitrate;
	}

	if (Settings::Instance().m_use10bitEncoder) {
		encodeConfig.rcParams.enableAQ = 1;
		encodeConfig.encodeCodecConfig.hevcConfig.pixelBitDepthMinus8 = 2;
	}

	if (Settings::Instance().m_nvencEnableAQ != -1) {
		encodeConfig.rcParams.enableAQ = Settings::Instance().m_nvencEnableAQ;
	}
}