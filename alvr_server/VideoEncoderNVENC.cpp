
#include "VideoEncoderNVENC.h"
#include "NvCodecUtils.h"
#include "nvencoderclioptions.h"

VideoEncoderNVENC::VideoEncoderNVENC(std::shared_ptr<CD3DRender> pD3DRender
	, std::shared_ptr<Listener> listener, bool useNV12)
	: m_pD3DRender(pD3DRender)
	, m_nFrame(0)
	, m_Listener(listener)
	, m_useNV12(useNV12)
	, m_codec(Settings::Instance().m_codec)
	, m_refreshRate(Settings::Instance().m_refreshRate)
	, m_renderWidth(Settings::Instance().m_renderWidth)
	, m_renderHeight(Settings::Instance().m_renderHeight)
	, m_bitrateInMBits(Settings::Instance().mEncodeBitrate.toMiBits())
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
	if (m_useNV12) {
		format = NV_ENC_BUFFER_FORMAT_NV12;
	}

	Log(L"Initializing CNvEncoder. Width=%d Height=%d Format=%d (useNV12:%d)", m_renderWidth, m_renderHeight
		, format, m_useNV12);

	if (m_useNV12) {
		if (!LoadCudaDLL()) {
			throw MakeException(L"Failed to load nvcuda.dll. Please check if NVIDIA graphic driver is installed.");
		}
		try {
			m_Converter = std::make_shared<CudaConverter>(m_pD3DRender->GetDevice(), m_renderWidth, m_renderHeight);
		}
		catch (Exception e) {
			throw MakeException(L"Exception:%s", e.what());
		}

		try {
			m_NvNecoder = std::make_shared<NvEncoderCuda>(m_Converter->GetContext(), m_renderWidth, m_renderHeight, format, 0);
		}
		catch (NVENCException e) {
			throw MakeException(L"NvEnc NvEncoderCuda failed. Code=%d %hs", e.getErrorCode(), e.what());
		}
	}
	else {
		try {
			m_NvNecoder = std::make_shared<NvEncoderD3D11>(m_pD3DRender->GetDevice(), m_renderWidth, m_renderHeight, format, 0);
		}
		catch (NVENCException e) {
			throw MakeException(L"NvEnc NvEncoderD3D11 failed. Code=%d %hs", e.getErrorCode(), e.what());
		}
	}

	NV_ENC_INITIALIZE_PARAMS initializeParams = { NV_ENC_INITIALIZE_PARAMS_VER };
	NV_ENC_CONFIG encodeConfig = { NV_ENC_CONFIG_VER };

	initializeParams.encodeConfig = &encodeConfig;
	GUID EncoderGUID = m_codec == ALVR_CODEC_H264 ? NV_ENC_CODEC_H264_GUID : NV_ENC_CODEC_HEVC_GUID;
	m_NvNecoder->CreateDefaultEncoderParams(&initializeParams, EncoderGUID, NV_ENC_PRESET_LOW_LATENCY_HQ_GUID);

	if (m_codec == ALVR_CODEC_H264) {
		initializeParams.encodeConfig->encodeCodecConfig.h264Config.repeatSPSPPS = 1;
	}
	else {
		initializeParams.encodeConfig->encodeCodecConfig.hevcConfig.repeatSPSPPS = 1;
	}

	initializeParams.encodeConfig->rcParams.rateControlMode = NV_ENC_PARAMS_RC_CBR_LOWDELAY_HQ;
	initializeParams.frameRateNum = m_refreshRate;
	initializeParams.encodeConfig->rcParams.maxBitRate =
		initializeParams.encodeConfig->rcParams.averageBitRate = m_bitrateInMBits * 1000 * 1000;
	// Disable automatic IDR insertion by NVENC. We need to manually insert IDR when packet is dropped.
	initializeParams.encodeConfig->gopLength = NVENC_INFINITE_GOPLENGTH;

	//initializeParams.maxEncodeWidth = 3840;
	//initializeParams.maxEncodeHeight = 2160;

	try {
		m_NvNecoder->CreateEncoder(&initializeParams);
	}
	catch (NVENCException e) {
		if (e.getErrorCode() == NV_ENC_ERR_INVALID_PARAM) {
			throw MakeException(L"This GPU does not support H.265 encoding. (NvEncoderCuda NV_ENC_ERR_INVALID_PARAM)");
		}
		throw MakeException(L"NvEnc CreateEncoder failed. Code=%d %hs", e.getErrorCode(), e.what());
	}

	//
	// Initialize debug video output
	//

	if (Settings::Instance().m_DebugCaptureOutput) {
		fpOut = std::ofstream(Settings::Instance().GetVideoOutput(), std::ios::out | std::ios::binary);
		if (!fpOut)
		{
			Log(L"unable to open output file %hs", Settings::Instance().GetVideoOutput().c_str());
		}
	}

	Log(L"CNvEncoder is successfully initialized.");
}

void VideoEncoderNVENC::Reconfigure(int refreshRate, int renderWidth, int renderHeight, int bitrateInMBits)
{
	if ((refreshRate != 0 && refreshRate != m_refreshRate) ||
		(renderWidth != 0 && renderWidth != m_renderWidth) ||
		(renderHeight != 0 && renderHeight != m_renderHeight) ||
		(bitrateInMBits != 0 && bitrateInMBits != m_bitrateInMBits)) {
		NV_ENC_RECONFIGURE_PARAMS reconfigureParams = { NV_ENC_RECONFIGURE_PARAMS_VER };
		NV_ENC_CONFIG encodeConfig = { NV_ENC_CONFIG_VER };

		reconfigureParams.resetEncoder = 1; // Needed?
		reconfigureParams.forceIDR = 1;
		reconfigureParams.reInitEncodeParams.version = NV_ENC_INITIALIZE_PARAMS_VER;
		reconfigureParams.reInitEncodeParams.encodeConfig = &encodeConfig;

		GUID EncoderGUID = m_codec == ALVR_CODEC_H264 ? NV_ENC_CODEC_H264_GUID : NV_ENC_CODEC_HEVC_GUID;

		m_NvNecoder->CreateDefaultEncoderParams(&reconfigureParams.reInitEncodeParams, EncoderGUID, NV_ENC_PRESET_LOW_LATENCY_HQ_GUID);

		if (m_codec == ALVR_CODEC_H264) {
			reconfigureParams.reInitEncodeParams.encodeConfig->encodeCodecConfig.h264Config.repeatSPSPPS = 1;
		}
		else {
			reconfigureParams.reInitEncodeParams.encodeConfig->encodeCodecConfig.hevcConfig.repeatSPSPPS = 1;
		}

		reconfigureParams.reInitEncodeParams.encodeConfig->rcParams.rateControlMode = NV_ENC_PARAMS_RC_CBR_LOWDELAY_HQ;
		reconfigureParams.reInitEncodeParams.frameRateNum = refreshRate;
		reconfigureParams.reInitEncodeParams.encodeConfig->rcParams.maxBitRate =
			reconfigureParams.reInitEncodeParams.encodeConfig->rcParams.averageBitRate = bitrateInMBits * 1000 * 1000;
		// Disable automatic IDR insertion by NVENC. We need to manually insert IDR when packet is dropped.
		reconfigureParams.reInitEncodeParams.encodeConfig->gopLength = NVENC_INFINITE_GOPLENGTH;

		reconfigureParams.reInitEncodeParams.encodeWidth = reconfigureParams.reInitEncodeParams.darWidth =
			reconfigureParams.reInitEncodeParams.maxEncodeWidth = renderWidth;
		reconfigureParams.reInitEncodeParams.encodeHeight = reconfigureParams.reInitEncodeParams.darHeight =
			reconfigureParams.reInitEncodeParams.maxEncodeHeight = renderHeight;

		bool ret = false;
		try {
			ret = m_NvNecoder->Reconfigure(&reconfigureParams);
		}
		catch (NVENCException e) {
			FatalLog(L"NvEnc Reconfigure failed with exception. Code=%d %hs. (%dHz %dx%d %dMbits) -> (%dHz %dx%d %dMbits)", e.getErrorCode(), e.what()
				, m_refreshRate, m_renderWidth, m_renderHeight, m_bitrateInMBits
				, refreshRate, renderWidth, renderHeight, bitrateInMBits
			);
			return;
		}
		if (!ret) {
			FatalLog(L"NvEnc Reconfigure failed. Return code=%d. (%dHz %dx%d %dMbits) -> (%dHz %dx%d %dMbits)", ret
				, m_refreshRate, m_renderWidth, m_renderHeight, m_bitrateInMBits
				, refreshRate, renderWidth, renderHeight, bitrateInMBits
			);
			return;
		}
		Log(L"NvEnc Reconfigure succeeded. (%dHz %dx%d %dMbits) -> (%dHz %dx%d %dMbits)"
			, m_refreshRate, m_renderWidth, m_renderHeight, m_bitrateInMBits
			, refreshRate, renderWidth, renderHeight, bitrateInMBits
		);

		if (refreshRate != 0) {
			m_refreshRate = refreshRate;
		}
		if (renderWidth != 0) {
			m_renderWidth = renderWidth;
		}
		if (renderHeight != 0) {
			m_renderHeight = renderHeight;
		}
		if (bitrateInMBits != 0) {
			m_bitrateInMBits = bitrateInMBits;
		}
	}
}

void VideoEncoderNVENC::Shutdown()
{
	std::vector<std::vector<uint8_t>> vPacket;
	m_NvNecoder->EndEncode(vPacket);

	for (std::vector<uint8_t> &packet : vPacket)
	{
		if (fpOut) {
			fpOut.write(reinterpret_cast<char*>(packet.data()), packet.size());
		}
	}
	m_NvNecoder->DestroyEncoder();
	m_NvNecoder.reset();

	Log(L"CNvEncoder::Shutdown");

	if (fpOut) {
		fpOut.close();
	}
}

void VideoEncoderNVENC::Transmit(ID3D11Texture2D *pTexture, uint64_t presentationTime, uint64_t frameIndex, uint64_t frameIndex2, uint64_t clientTime, bool insertIDR)
{
	std::vector<std::vector<uint8_t>> vPacket;

	const NvEncInputFrame* encoderInputFrame = m_NvNecoder->GetNextInputFrame();

	if (m_useNV12)
	{
		try {
			m_Converter->Convert(pTexture, encoderInputFrame);
		}
		catch (NVENCException e) {
			FatalLog(L"Exception:%hs", e.what());
			return;
		}
	}
	else {
		ID3D11Texture2D *pInputTexture = reinterpret_cast<ID3D11Texture2D*>(encoderInputFrame->inputPtr);
		m_pD3DRender->GetContext()->CopyResource(pInputTexture, pTexture);
	}

	NV_ENC_PIC_PARAMS picParams = {};
	if (insertIDR) {
		Log(L"Inserting IDR frame.");
		picParams.encodePicFlags = NV_ENC_PIC_FLAG_FORCEIDR;
	}
	m_NvNecoder->EncodeFrame(vPacket, &picParams);

	Log(L"Tracking info delay: %lld us FrameIndex=%llu", GetTimestampUs() - m_Listener->clientToServerTime(clientTime), frameIndex);
	Log(L"Encoding delay: %lld us FrameIndex=%llu", GetTimestampUs() - presentationTime, frameIndex);

	if (m_Listener) {
		m_Listener->GetStatistics()->EncodeOutput(GetTimestampUs() - presentationTime);
	}

	m_nFrame += (int)vPacket.size();
	for (std::vector<uint8_t> &packet : vPacket)
	{
		if (fpOut) {
			fpOut.write(reinterpret_cast<char*>(packet.data()), packet.size());
		}
		if (m_Listener) {
			m_Listener->SendVideo(packet.data(), (int)packet.size(), frameIndex);
		}
	}

	if (Settings::Instance().m_DebugFrameOutput) {
		if (!m_useNV12) {
			SaveDebugOutput(m_pD3DRender, vPacket, reinterpret_cast<ID3D11Texture2D*>(encoderInputFrame->inputPtr), frameIndex2);
		}
	}
}