#include "VideoEncoderVCE.h"

VideoEncoderVCE::VideoEncoderVCE(std::shared_ptr<CD3DRender> d3dRender
	, std::shared_ptr<Listener> listener, int width, int height, bool useNV12)
	: m_d3dRender(d3dRender)
	, m_nFrame(0)
	, m_Listener(listener)
	, m_useNV12(useNV12)
	, m_width(width)
	, m_height(height)
{
}

VideoEncoderVCE::~VideoEncoderVCE()
{}

bool VideoEncoderVCE::Initialize()
{
	const wchar_t *pCodec;

	switch (Settings::Instance().m_codec){
	case ALVR_CODEC_H264:
		pCodec = AMFVideoEncoderVCE_AVC;
		break;
	case ALVR_CODEC_H265:
		pCodec = AMFVideoEncoder_HEVC;
		break;
	default:
		FatalLog("Unsupported video encoding %d", Settings::Instance().m_codec);
		return false;
	}

	AMF_RESULT res = AMF_OK; // error checking can be added later
	res = g_AMFFactory.Init();
	if (res != AMF_OK)
	{
		FatalLog("AMF Failed to initialize");
		return false;
	}

	::amf_increase_timer_precision();

	amf_int32 frameRateIn = 60;
	amf_int64 bitRateIn = 5000000L; // in bits, 25MBit
	amf_int32 rectSize = 50;
	amf_int32 frameCount = 500;
	bool bMaximumSpeed = true;

	// context
	res = g_AMFFactory.GetFactory()->CreateContext(&m_amfContext);
	if (res != AMF_OK)
	{
		FatalLog("AMF Failed on CreateContext");
		return false;
	}
	res = m_amfContext->InitDX11(m_d3dRender->GetDevice()); // can be DX11 device
	if (res != AMF_OK)
	{
		FatalLog("AMF Failed on InitDX11");
		return false;
	}

	// Create encoder component.
	res = g_AMFFactory.GetFactory()->CreateComponent(m_amfContext, pCodec, &m_amfEncoder);
	if (res != AMF_OK)
	{
		FatalLog("AMF Failed on CreateComponent");
		return false;
	}

	if (amf_wstring(pCodec) == amf_wstring(AMFVideoEncoderVCE_AVC))
	{
		res = m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_USAGE, AMF_VIDEO_ENCODER_USAGE_ULTRA_LOW_LATENCY);

		if (bMaximumSpeed)
		{
			res = m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_B_PIC_PATTERN, 0);
			res = m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_QUALITY_PRESET, AMF_VIDEO_ENCODER_QUALITY_PRESET_SPEED);
		}

		res = m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_TARGET_BITRATE, bitRateIn);
		res = m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_FRAMESIZE, ::AMFConstructSize(m_width, m_height));
		res = m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_FRAMERATE, ::AMFConstructRate(frameRateIn, 1));
#if defined(ENABLE_4K)
		m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_PROFILE, AMF_VIDEO_ENCODER_PROFILE_HIGH);
		res = m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_PROFILE_LEVEL, 51);
		m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_B_PIC_PATTERN, 0);
#endif
	}
	else
	{
		res = m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_USAGE, AMF_VIDEO_ENCODER_HEVC_USAGE_ULTRA_LOW_LATENCY);

		if (bMaximumSpeed)
		{
			res = m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET, AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET_SPEED);
		}

		res = m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_TARGET_BITRATE, bitRateIn);
		res = m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_FRAMESIZE, ::AMFConstructSize(Settings::Instance().m_renderWidth, Settings::Instance().m_renderHeight));
		res = m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_FRAMERATE, ::AMFConstructRate(frameRateIn, 1));

#if defined(ENABLE_4K)
		m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_TIER, AMF_VIDEO_ENCODER_HEVC_TIER_HIGH);
		res = m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_PROFILE_LEVEL, AMF_LEVEL_5_1);
#endif
	}
	res = m_amfEncoder->Init(ENCODER_INPUT_FORMAT, Settings::Instance().m_renderWidth, Settings::Instance().m_renderHeight);
	if (res != AMF_OK)
	{
		FatalLog("AMF Failed on Encoder Init");
		return false;
	}

	//
	// Initialize debug video output
	//

	if (Settings::Instance().m_DebugCaptureOutput) {
		fpOut = std::ofstream(Settings::Instance().GetVideoOutput(), std::ios::out | std::ios::binary);
		if (!fpOut)
		{
			Log("Unable to open output file %s", Settings::Instance().GetVideoOutput().c_str());
		}
	}

	Log("Successfully initialized VideoEncoderVCE.");
	return true;
}

void VideoEncoderVCE::Shutdown()
{
	Log("VideoEncoderVCE::Shutdown");

	amf_restore_timer_precision();

	if (fpOut) {
		fpOut.close();
	}
}

void VideoEncoderVCE::Transmit(ID3D11Texture2D *pTexture, uint64_t presentationTime, uint64_t frameIndex, uint64_t frameIndex2, uint64_t clientTime, bool insertIDR)
{
	amf::AMFSurfacePtr surface;
	AMF_RESULT res = m_amfContext->CreateSurfaceFromDX11Native(pTexture, &surface, NULL);

	amf_pts start_time = amf_high_precision_clock();
	surface->SetProperty(START_TIME_PROPERTY, start_time);

	// Disable AUD (NAL Type 9) to produce the same stream format as VideoEncoderNVENC.
	surface->SetProperty(AMF_VIDEO_ENCODER_INSERT_AUD, false);
	if (insertIDR) {
		Log("Inserting IDR frame.");
		surface->SetProperty(AMF_VIDEO_ENCODER_INSERT_SPS, true);
		surface->SetProperty(AMF_VIDEO_ENCODER_INSERT_PPS, true);
		surface->SetProperty(AMF_VIDEO_ENCODER_FORCE_PICTURE_TYPE, AMF_VIDEO_ENCODER_PICTURE_TYPE_IDR);
	}
	
	while (true)
	{
		res = m_amfEncoder->SubmitInput(surface);
		if (res == AMF_INPUT_FULL)
		{
			Sleep(1);
		}
		else
		{
			break;
		}
	}

	Log("Successfully sent input to encoder.");

	amf::AMFDataPtr data;
	while (true)
	{
		res = m_amfEncoder->QueryOutput(&data);
		if (res == AMF_EOF)
		{
			Log("m_amfConverter->QueryOutput returns AMF_EOF.");
			return;
		}

		if (data != NULL)
		{
			break;
		}
		else
		{
			Sleep(1);
		}
	}

	amf_pts current_time = amf_high_precision_clock();
	start_time = 0;
	data->GetProperty(START_TIME_PROPERTY, &start_time);

	amf::AMFBufferPtr buffer(data); // query for buffer interface

	Log("VCE encode latency: %.4f ms. Size=%d bytes", double(current_time - start_time) / MILLISEC_TIME, (int)buffer->GetSize());

	if (fpOut) {
		fpOut.write(reinterpret_cast<char *>(buffer->GetNative()), buffer->GetSize());
	}
	if (m_Listener) {
		m_Listener->SendVideo(reinterpret_cast<uint8_t *>(buffer->GetNative()), (int)buffer->GetSize(), frameIndex);
	}
}
