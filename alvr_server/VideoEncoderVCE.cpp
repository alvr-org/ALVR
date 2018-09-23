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

	amf_int32 frameRateIn = Settings::Instance().m_encodeFPS;
	amf_int64 bitRateIn = Settings::Instance().m_encodeBitrateInMBits * 1000000L; // in bits

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

		res = m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_B_PIC_PATTERN, 0);
		res = m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_QUALITY_PRESET, AMF_VIDEO_ENCODER_QUALITY_PRESET_SPEED);

		res = m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_TARGET_BITRATE, bitRateIn);
		res = m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_FRAMESIZE, ::AMFConstructSize(m_width, m_height));
		res = m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_FRAMERATE, ::AMFConstructRate(frameRateIn, 1));

		//m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_PROFILE, AMF_VIDEO_ENCODER_PROFILE_HIGH);
		//m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_PROFILE_LEVEL, 51);
	}
	else
	{
		res = m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_USAGE, AMF_VIDEO_ENCODER_HEVC_USAGE_ULTRA_LOW_LATENCY);

		res = m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET, AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET_SPEED);

		res = m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_TARGET_BITRATE, bitRateIn);
		res = m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_FRAMESIZE, ::AMFConstructSize(m_width, m_height));
		res = m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_FRAMERATE, ::AMFConstructRate(frameRateIn, 1));

		//m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_TIER, AMF_VIDEO_ENCODER_HEVC_TIER_HIGH);
		//m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_PROFILE_LEVEL, AMF_LEVEL_5);
	}
	res = m_amfEncoder->Init(ENCODER_INPUT_FORMAT, m_width, m_height);
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

	ApplyFrameProperties(surface, insertIDR);
	
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

	char *p = reinterpret_cast<char *>(buffer->GetNative());
	int length = buffer->GetSize();

	SkipAUD(&p, &length);

	if (fpOut) {
		fpOut.write(p, length);
	}
	if (m_Listener) {
		m_Listener->SendVideo(reinterpret_cast<uint8_t *>(p), length, frameIndex);
	}
}

void VideoEncoderVCE::ApplyFrameProperties(const amf::AMFSurfacePtr &surface, bool insertIDR) {
	switch (Settings::Instance().m_codec) {
	case ALVR_CODEC_H264:
		// Disable AUD (NAL Type 9) to produce the same stream format as VideoEncoderNVENC.
		surface->SetProperty(AMF_VIDEO_ENCODER_INSERT_AUD, false);
		if (insertIDR) {
			Log("Inserting IDR frame for H.264.");
			surface->SetProperty(AMF_VIDEO_ENCODER_INSERT_SPS, true);
			surface->SetProperty(AMF_VIDEO_ENCODER_INSERT_PPS, true);
			surface->SetProperty(AMF_VIDEO_ENCODER_FORCE_PICTURE_TYPE, AMF_VIDEO_ENCODER_PICTURE_TYPE_IDR);
		}
		break;
	case ALVR_CODEC_H265:
		// This option is ignored. Maybe a bug on AMD driver.
		surface->SetProperty(AMF_VIDEO_ENCODER_HEVC_INSERT_AUD, false);
		if (insertIDR) {
			Log("Inserting IDR frame for H.265.");
			// Insert VPS,SPS,PPS
			// These options don't work properly on older AMD driver (Radeon Software 17.7, AMF Runtime 1.4.4)
			// Fixed in 18.9.2 & 1.4.9
			surface->SetProperty(AMF_VIDEO_ENCODER_HEVC_INSERT_HEADER, true);
			surface->SetProperty(AMF_VIDEO_ENCODER_HEVC_FORCE_PICTURE_TYPE, AMF_VIDEO_ENCODER_HEVC_PICTURE_TYPE_IDR);
		}
		break;
	}
}

void VideoEncoderVCE::SkipAUD(char **buffer, int *length) {
	// H.265 encoder always produces AUD NAL even if AMF_VIDEO_ENCODER_HEVC_INSERT_AUD is set. But it is not needed.
	static const int AUD_NAL_SIZE = 7;

	if (Settings::Instance().m_codec != ALVR_CODEC_H265) {
		return;
	}

	if (*length < AUD_NAL_SIZE + 4) {
		return;
	}

	// Check if start with AUD NAL.
	if (memcmp(*buffer, "\x00\x00\x00\x01\x46", 5) != 0) {
		return;
	}
	// Check if AUD NAL size is AUD_NAL_SIZE bytes.
	if (memcmp(*buffer + AUD_NAL_SIZE, "\x00\x00\x00\x01", 4) != 0) {
		return;
	}
	*buffer += AUD_NAL_SIZE;
	*length -= AUD_NAL_SIZE;
}