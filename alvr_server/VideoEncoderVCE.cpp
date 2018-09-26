#include "VideoEncoderVCE.h"

#define AMF_THROW_IF(expr) {AMF_RESULT res = expr;\
if(res != AMF_OK){throw MakeException("AMF Error %d. %s", res, #expr);}}

const wchar_t *VideoEncoderVCE::START_TIME_PROPERTY = L"StartTimeProperty";
const wchar_t *VideoEncoderVCE::FRAME_INDEX_PROPERTY = L"FrameIndexProperty";

//
// AMFTextureEncoder
//

AMFTextureEncoder::AMFTextureEncoder(const amf::AMFContextPtr &amfContext
	, int width, int height
	, amf::AMF_SURFACE_FORMAT inputFormat
	, AMFTextureReceiver receiver) : m_receiver(receiver)
{
	const wchar_t *pCodec;

	amf_int32 frameRateIn = Settings::Instance().m_encodeFPS;
	amf_int64 bitRateIn = Settings::Instance().m_encodeBitrateInMBits * 1000000L; // in bits

	switch (Settings::Instance().m_codec) {
	case ALVR_CODEC_H264:
		pCodec = AMFVideoEncoderVCE_AVC;
		break;
	case ALVR_CODEC_H265:
		pCodec = AMFVideoEncoder_HEVC;
		break;
	default:
		throw MakeException("Unsupported video encoding %d", Settings::Instance().m_codec);
	}

	// Create encoder component.
	AMF_THROW_IF(g_AMFFactory.GetFactory()->CreateComponent(amfContext, pCodec, &m_amfEncoder));

	if (Settings::Instance().m_codec == ALVR_CODEC_H264)
	{
		m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_USAGE, AMF_VIDEO_ENCODER_USAGE_ULTRA_LOW_LATENCY);

		m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_B_PIC_PATTERN, 0);
		m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_QUALITY_PRESET, AMF_VIDEO_ENCODER_QUALITY_PRESET_QUALITY);

		m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_TARGET_BITRATE, bitRateIn);
		m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_FRAMESIZE, ::AMFConstructSize(width, height));
		m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_FRAMERATE, ::AMFConstructRate(frameRateIn, 1));

		//m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_PROFILE, AMF_VIDEO_ENCODER_PROFILE_HIGH);
		//m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_PROFILE_LEVEL, 51);
	}
	else
	{
		m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_USAGE, AMF_VIDEO_ENCODER_HEVC_USAGE_ULTRA_LOW_LATENCY);

		m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET, AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET_QUALITY);

		m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_TARGET_BITRATE, bitRateIn);
		m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_FRAMESIZE, ::AMFConstructSize(width, height));
		m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_FRAMERATE, ::AMFConstructRate(frameRateIn, 1));

		//m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_TIER, AMF_VIDEO_ENCODER_HEVC_TIER_HIGH);
		//m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_PROFILE_LEVEL, AMF_LEVEL_5);
	}
	AMF_THROW_IF(m_amfEncoder->Init(inputFormat, width, height));

	Log("Initialized AMFTextureEncoder.");
}

AMFTextureEncoder::~AMFTextureEncoder()
{
}

void AMFTextureEncoder::Start()
{
	m_thread = new std::thread(&AMFTextureEncoder::Run, this);
}

void AMFTextureEncoder::Shutdown()
{
	Log("AMFTextureEncoder::Shutdown() m_amfEncoder->Drain");
	m_amfEncoder->Drain();
	Log("AMFTextureEncoder::Shutdown() m_thread->join");
	m_thread->join();
	Log("AMFTextureEncoder::Shutdown() joined.");
	delete m_thread;
	m_thread = NULL;
}

void AMFTextureEncoder::Submit(amf::AMFData *data)
{
	while (true)
	{
		Log("AMFTextureEncoder::Submit.");
		auto res = m_amfEncoder->SubmitInput(data);
		if (res == AMF_INPUT_FULL)
		{
			return;
		}
		else
		{
			break;
		}
	}
}

void AMFTextureEncoder::Run()
{
	Log("Start AMFTextureEncoder thread. Thread Id=%d", GetCurrentThreadId());
	amf::AMFDataPtr data;
	while (true)
	{
		auto res = m_amfEncoder->QueryOutput(&data);
		if (res == AMF_EOF)
		{
			Log("m_amfEncoder->QueryOutput returns AMF_EOF.");
			return;
		}

		if (data != NULL)
		{
			m_receiver(data);
		}
		else
		{
			Sleep(1);
		}
	}
}

//
// AMFTextureConverter
//

AMFTextureConverter::AMFTextureConverter(const amf::AMFContextPtr &amfContext
	, int width, int height
	, amf::AMF_SURFACE_FORMAT inputFormat, amf::AMF_SURFACE_FORMAT outputFormat
	, AMFTextureReceiver receiver) : m_receiver(receiver)
{
	AMF_THROW_IF(g_AMFFactory.GetFactory()->CreateComponent(amfContext, AMFVideoConverter, &m_amfConverter));

	AMF_THROW_IF(m_amfConverter->SetProperty(AMF_VIDEO_CONVERTER_MEMORY_TYPE, amf::AMF_MEMORY_DX11));
	AMF_THROW_IF(m_amfConverter->SetProperty(AMF_VIDEO_CONVERTER_OUTPUT_FORMAT, outputFormat));
	AMF_THROW_IF(m_amfConverter->SetProperty(AMF_VIDEO_CONVERTER_OUTPUT_SIZE, ::AMFConstructSize(width, height)));

	AMF_THROW_IF(m_amfConverter->Init(inputFormat, width, height));

	Log("Initialized AMFTextureConverter.");
}

AMFTextureConverter::~AMFTextureConverter()
{
}

void AMFTextureConverter::Start()
{
	m_thread = new std::thread(&AMFTextureConverter::Run, this);
}

void AMFTextureConverter::Shutdown()
{
	Log("AMFTextureConverter::Shutdown() m_amfConverter->Drain");
	m_amfConverter->Drain();
	Log("AMFTextureConverter::Shutdown() m_thread->join");
	m_thread->join();
	Log("AMFTextureConverter::Shutdown() joined.");
	delete m_thread;
	m_thread = NULL;
}

void AMFTextureConverter::Submit(amf::AMFData *data)
{
	while (true)
	{
		Log("AMFTextureConverter::Submit.");
		auto res = m_amfConverter->SubmitInput(data);
		if (res == AMF_INPUT_FULL)
		{
			return;
		}
		else
		{
			break;
		}
	}
}

void AMFTextureConverter::Run()
{
	Log("Start AMFTextureConverter thread. Thread Id=%d", GetCurrentThreadId());
	amf::AMFDataPtr data;
	while (true)
	{
		auto res = m_amfConverter->QueryOutput(&data);
		if (res == AMF_EOF)
		{
			Log("m_amfConverter->QueryOutput returns AMF_EOF.");
			return;
		}

		if (data != NULL)
		{
			m_receiver(data);
		}
		else
		{
			Sleep(1);
		}
	}
}

//
// VideoEncoderVCE
//

VideoEncoderVCE::VideoEncoderVCE(std::shared_ptr<CD3DRender> d3dRender
	, std::shared_ptr<Listener> listener, int width, int height)
	: m_d3dRender(d3dRender)
	, m_Listener(listener)
	, m_width(width)
	, m_height(height)
{
}

VideoEncoderVCE::~VideoEncoderVCE()
{}

void VideoEncoderVCE::Initialize()
{
	Log("Initializing VideoEncoderVCE.");
	AMF_THROW_IF(g_AMFFactory.Init());

	::amf_increase_timer_precision();

	AMF_THROW_IF(g_AMFFactory.GetFactory()->CreateContext(&m_amfContext));
	AMF_THROW_IF(m_amfContext->InitDX11(m_d3dRender->GetDevice()));

	m_encoder = std::make_shared<AMFTextureEncoder>(m_amfContext
		, m_width, m_height
		, ENCODER_INPUT_FORMAT, std::bind(&VideoEncoderVCE::Receive, this, std::placeholders::_1));
	m_converter = std::make_shared<AMFTextureConverter>(m_amfContext
		, m_width, m_height
		, CONVERTER_INPUT_FORMAT, ENCODER_INPUT_FORMAT
		, std::bind(&AMFTextureEncoder::Submit, m_encoder.get(), std::placeholders::_1));

	m_encoder->Start();
	m_converter->Start();

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
}

void VideoEncoderVCE::Shutdown()
{
	Log("Shutting down VideoEncoderVCE.");

	m_encoder->Shutdown();
	m_converter->Shutdown();

	amf_restore_timer_precision();

	if (fpOut) {
		fpOut.close();
	}
	Log("Successfully shutdown VideoEncoderVCE.");
}

void VideoEncoderVCE::Transmit(ID3D11Texture2D *pTexture, uint64_t presentationTime, uint64_t frameIndex, uint64_t frameIndex2, uint64_t clientTime, bool insertIDR)
{
	amf::AMFSurfacePtr surface;
	// Surface is cached by AMF.
	AMF_THROW_IF(m_amfContext->AllocSurface(amf::AMF_MEMORY_DX11, CONVERTER_INPUT_FORMAT, m_width, m_height, &surface));
	ID3D11Texture2D *textureDX11 = (ID3D11Texture2D*)surface->GetPlaneAt(0)->GetNative(); // no reference counting - do not Release()
	m_d3dRender->GetContext()->CopyResource(textureDX11, pTexture);

	amf_pts start_time = amf_high_precision_clock();
	surface->SetProperty(START_TIME_PROPERTY, start_time);
	surface->SetProperty(FRAME_INDEX_PROPERTY, frameIndex);

	ApplyFrameProperties(surface, insertIDR);
	
	m_converter->Submit(surface);
}

void VideoEncoderVCE::Receive(amf::AMFData *data)
{
	amf_pts current_time = amf_high_precision_clock();
	amf_pts start_time = 0;
	uint64_t frameIndex;
	data->GetProperty(START_TIME_PROPERTY, &start_time);
	data->GetProperty(FRAME_INDEX_PROPERTY, &frameIndex);

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
