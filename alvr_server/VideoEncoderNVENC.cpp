
#include "VideoEncoderNVENC.h"
#include "NvCodecUtils.h"
#include "nvencoderclioptions.h"

VideoEncoderNVENC::VideoEncoderNVENC(std::shared_ptr<CD3DRender> pD3DRender
	, std::shared_ptr<Listener> listener, bool useNV12)
	: m_pD3DRender(pD3DRender)
	, m_nFrame(0)
	, m_Listener(listener)
	, m_useNV12(useNV12)
{
}

VideoEncoderNVENC::~VideoEncoderNVENC()
{}

bool VideoEncoderNVENC::Initialize()
{
	NvEncoderInitParam EncodeCLIOptions(Settings::Instance().m_EncoderOptions.c_str());

	//
	// Initialize Encoder
	//

	NV_ENC_BUFFER_FORMAT format = NV_ENC_BUFFER_FORMAT_ABGR;
	if (m_useNV12) {
		format = NV_ENC_BUFFER_FORMAT_NV12;
	}

	Log("Initializing CNvEncoder. Width=%d Height=%d Format=%d (useNV12:%d)", Settings::Instance().m_renderWidth, Settings::Instance().m_renderHeight
		, format, m_useNV12);

	if (m_useNV12) {
		if (!LoadCudaDLL()) {
			FatalLog("Failed to load nvcuda.dll. Please check if NVIDIA graphic driver is installed.");
			return false;
		}
		try {
			m_Converter = std::make_shared<CudaConverter>(m_pD3DRender->GetDevice(), Settings::Instance().m_renderWidth, Settings::Instance().m_renderHeight);
		}
		catch (Exception e) {
			FatalLog("Exception:%s", e.what());
			return false;
		}

		try {
			m_NvNecoder = std::make_shared<NvEncoderCuda>(m_Converter->GetContext(), Settings::Instance().m_renderWidth, Settings::Instance().m_renderHeight, format, 0);
		}
		catch (NVENCException e) {
			if (e.getErrorCode() == NV_ENC_ERR_INVALID_PARAM) {
				FatalLog("This GPU does not port H.265 encoding. (NvEncoderCuda NV_ENC_ERR_INVALID_PARAM)");
				return false;
			}
			FatalLog("NvEnc NvEncoderCuda failed. Code=%d %s", e.getErrorCode(), e.what());
			return false;
		}
	}
	else {
		try {
			m_NvNecoder = std::make_shared<NvEncoderD3D11>(m_pD3DRender->GetDevice(), Settings::Instance().m_renderWidth, Settings::Instance().m_renderHeight, format, 0);
		}
		catch (NVENCException e) {
			if (e.getErrorCode() == NV_ENC_ERR_INVALID_PARAM) {
				FatalLog("This GPU does not port H.265 encoding. (NvEncoderD3D11 NV_ENC_ERR_INVALID_PARAM)");
				return false;
			}
			FatalLog("NvEnc NvEncoderD3D11 failed. Code=%d %s", e.getErrorCode(), e.what());
			return false;
		}
	}

	NV_ENC_INITIALIZE_PARAMS initializeParams = { NV_ENC_INITIALIZE_PARAMS_VER };
	NV_ENC_CONFIG encodeConfig = { NV_ENC_CONFIG_VER };

	initializeParams.encodeConfig = &encodeConfig;
	GUID EncoderGUID = Settings::Instance().m_codec == ALVR_CODEC_H264 ? NV_ENC_CODEC_H264_GUID : NV_ENC_CODEC_HEVC_GUID;
	m_NvNecoder->CreateDefaultEncoderParams(&initializeParams, EncoderGUID, EncodeCLIOptions.GetPresetGUID());

	if (Settings::Instance().m_codec == ALVR_CODEC_H264) {
		initializeParams.encodeConfig->encodeCodecConfig.h264Config.repeatSPSPPS = 1;
	}
	else {
		initializeParams.encodeConfig->encodeCodecConfig.hevcConfig.repeatSPSPPS = 1;
	}

	EncodeCLIOptions.SetInitParams(&initializeParams, format);

	std::string parameterDesc = EncodeCLIOptions.FullParamToString(&initializeParams);
	Log("NvEnc Encoder Parameters:\n%s", parameterDesc.c_str());

	try {
		m_NvNecoder->CreateEncoder(&initializeParams);
	}
	catch (NVENCException e) {
		FatalLog("NvEnc CreateEncoder failed. Code=%d %s", e.getErrorCode(), e.what());
		return false;
	}

	//
	// Initialize debug video output
	//

	if (Settings::Instance().m_DebugCaptureOutput) {
		fpOut = std::ofstream(Settings::Instance().GetVideoOutput(), std::ios::out | std::ios::binary);
		if (!fpOut)
		{
			Log("unable to open output file %s", Settings::Instance().GetVideoOutput().c_str());
		}
	}

	Log("CNvEncoder is successfully initialized.");

	return true;
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
		m_Listener->SendVideo(packet.data(), (int)packet.size(), 0);
	}

	m_NvNecoder->DestroyEncoder();
	m_NvNecoder.reset();

	Log("CNvEncoder::Shutdown");

	if (fpOut) {
		fpOut.close();
	}
}

void VideoEncoderNVENC::Transmit(ID3D11Texture2D *pTexture, uint64_t presentationTime, uint64_t frameIndex, uint64_t frameIndex2, uint64_t clientTime, bool insertIDR)
{
	std::vector<std::vector<uint8_t>> vPacket;
	D3D11_TEXTURE2D_DESC desc;

	pTexture->GetDesc(&desc);

	Log("[VDispDvr] Transmit(begin) FrameIndex=%llu", frameIndex);

	const NvEncInputFrame* encoderInputFrame = m_NvNecoder->GetNextInputFrame();

	if (m_useNV12)
	{
		try {
			Log("ConvertRGBToNV12 start");
			m_Converter->Convert(pTexture, encoderInputFrame);
			Log("ConvertRGBToNV12 end");
		}
		catch (NVENCException e) {
			FatalLog("Exception:%s", e.what());
			return;
		}
	}
	else {
		ID3D11Texture2D *pInputTexture = reinterpret_cast<ID3D11Texture2D*>(encoderInputFrame->inputPtr);
		Log("CopyResource start");
		m_pD3DRender->GetContext()->CopyResource(pInputTexture, pTexture);
		Log("CopyResource end");
	}

	NV_ENC_PIC_PARAMS picParams = {};
	if (insertIDR) {
		Log("Inserting IDR frame.");
		picParams.encodePicFlags = NV_ENC_PIC_FLAG_FORCEIDR;
	}
	m_NvNecoder->EncodeFrame(vPacket, &picParams);

	Log("Tracking info delay: %lld us FrameIndex=%llu", GetTimestampUs() - m_Listener->clientToServerTime(clientTime), frameIndex);
	Log("Encoding delay: %lld us FrameIndex=%llu", GetTimestampUs() - presentationTime, frameIndex);

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

	Log("[VDispDvr] Transmit(end) (frame %d %d) FrameIndex=%llu", vPacket.size(), m_nFrame, frameIndex);
}
