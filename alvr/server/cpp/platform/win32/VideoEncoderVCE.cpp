#include "VideoEncoderVCE.h"

#include "alvr_server/Statistics.h"
#include "alvr_server/Logger.h"
#include "alvr_server/Settings.h"

#define AMF_THROW_IF(expr) {AMF_RESULT res = expr;\
if(res != AMF_OK){throw MakeException("AMF Error %d. %s", res, L#expr);}}

const wchar_t *VideoEncoderVCE::START_TIME_PROPERTY = L"StartTimeProperty";
const wchar_t *VideoEncoderVCE::FRAME_INDEX_PROPERTY = L"FrameIndexProperty";

AMFPipe::AMFPipe(amf::AMFComponentPtr src, AMFDataReceiver receiver) 
	: m_amfComponentSrc(src)
	, m_receiver(receiver) 
{}

AMFPipe::~AMFPipe() 
{
	Debug("AMFPipe::~AMFPipe()  m_amfComponentSrc->Drain\n");
	m_amfComponentSrc->Drain();
}

void AMFPipe::doPassthrough() 
{
	amf::AMFDataPtr data;
	auto res = m_amfComponentSrc->QueryOutput(&data);
	switch (res) {
		case AMF_OK:
			if (data) 
			{
				m_receiver(data);
			}
			break;
		case AMF_NO_DEVICE:
			Debug("m_amfComponentSrc->QueryOutput returns AMF_NO_DEVICE.\n");
			return;
		case AMF_REPEAT:
			break;
		case AMF_EOF:
			Debug("m_amfComponentSrc->QueryOutput returns AMF_EOF.\n");
			return;
		default:
			Debug("m_amfComponentSrc->QueryOutput returns unknown status.\n");
			return;
	}
}

AMFSolidPipe::AMFSolidPipe(amf::AMFComponentPtr src, amf::AMFComponentPtr dst) 
	: AMFPipe(src, std::bind(&AMFSolidPipe::Passthrough, this, std::placeholders::_1))
	, m_amfComponentDst(dst) 
{}

void AMFSolidPipe::Passthrough(AMFDataPtr data) 
{
	auto res = m_amfComponentDst->SubmitInput(data);
	switch (res) {
		case AMF_OK:
			break;
		case AMF_INPUT_FULL:
			Debug("m_amfComponentDst->SubmitInput returns AMF_INPUT_FULL.\n");
			break;
		case AMF_NEED_MORE_INPUT:
			Debug("m_amfComponentDst->SubmitInput returns AMF_NEED_MORE_INPUT.\n");
			break;
		default:
			Debug("m_amfComponentDst->SubmitInput returns code %d.\n", res);
			break;
	}
}

AMFPipeline::AMFPipeline() 
	: m_thread(nullptr)
	, m_pipes()
	, isRunning(false) 
{}

AMFPipeline::~AMFPipeline() 
{
	if (m_thread) 
	{
		isRunning = false;
		Debug("AMFPipeline::~AMFPipeline() m_thread->join\n");
		m_thread->join();
		Debug("AMFPipeline::~AMFPipeline() m_thread joined.\n");
		delete m_thread;
		m_thread = nullptr;
	}
	for (auto &pipe : m_pipes) 
	{
		delete pipe;
	}
}

void AMFPipeline::Connect(AMFPipePtr pipe) 
{
	m_pipes.emplace_back(pipe);
}

void AMFPipeline::Run()
{
	Debug("Start AMFPipeline thread. Thread Id=%d\n", GetCurrentThreadId());
	while (isRunning)
	{
		for (auto &pipe : m_pipes) 
		{
			pipe->doPassthrough();
		}
	}
}

void AMFPipeline::Start()
{
	isRunning = true;
	m_thread = new std::thread(&AMFPipeline::Run, this);
}

//
// VideoEncoderVCE
//

VideoEncoderVCE::VideoEncoderVCE(std::shared_ptr<CD3DRender> d3dRender
	, std::shared_ptr<ClientConnection> listener
	, int width, int height)
	: m_d3dRender(d3dRender)
	, m_Listener(listener)
	, m_codec(Settings::Instance().m_codec)
	, m_refreshRate(Settings::Instance().m_refreshRate)
	, m_renderWidth(width)
	, m_renderHeight(height)
	, m_bitrateInMBits(Settings::Instance().mEncodeBitrateMBs)
{}

VideoEncoderVCE::~VideoEncoderVCE() {}

amf::AMFComponentPtr VideoEncoderVCE::MakeEncoder(
	amf::AMF_SURFACE_FORMAT inputFormat, int width, int height, int codec, int refreshRate, int bitrateInMbits
) 
{
	const wchar_t *pCodec;

	amf_int32 frameRateIn = refreshRate;
	amf_int64 bitRateIn = bitrateInMbits * 1000000L; // in bits

	switch (codec) {
	case ALVR_CODEC_H264:
		pCodec = AMFVideoEncoderVCE_AVC;
		break;
	case ALVR_CODEC_H265:
		pCodec = AMFVideoEncoder_HEVC;
		break;
	default:
		throw MakeException("Unsupported video encoding %d", codec);
	}

	amf::AMFComponentPtr amfEncoder;
	// Create encoder component.
	AMF_THROW_IF(g_AMFFactory.GetFactory()->CreateComponent(m_amfContext, pCodec, &amfEncoder));

	if (codec == ALVR_CODEC_H264)
	{
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_USAGE, AMF_VIDEO_ENCODER_USAGE_ULTRA_LOW_LATENCY);
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_TARGET_BITRATE, bitRateIn);
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_FRAMESIZE, ::AMFConstructSize(width, height));
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_FRAMERATE, ::AMFConstructRate(frameRateIn, 1));
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_B_PIC_PATTERN, 0);
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_PROFILE, AMF_VIDEO_ENCODER_PROFILE_HIGH);
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_PROFILE_LEVEL, 42);

		switch (m_encoderQualityPreset) {
			case SPEED:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_QUALITY_PRESET, AMF_VIDEO_ENCODER_QUALITY_PRESET_SPEED);
				break;
			case BALANCED:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_QUALITY_PRESET, AMF_VIDEO_ENCODER_QUALITY_PRESET_BALANCED);
				break;
			case QUALITY:
			default:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_QUALITY_PRESET, AMF_VIDEO_ENCODER_QUALITY_PRESET_QUALITY);
				break;
		}

		if (m_use10bit) {
			amfEncoder->SetProperty(AMF_VIDEO_ENCODER_COLOR_BIT_DEPTH, AMF_COLOR_BIT_DEPTH_10);
		} else {
			amfEncoder->SetProperty(AMF_VIDEO_ENCODER_COLOR_BIT_DEPTH, AMF_COLOR_BIT_DEPTH_8);
		}

		//No noticable performance difference and should improve subjective quality by allocating more bits to smooth areas
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_ENABLE_VBAQ, true);
		
		//Fixes rythmic pixelation. I-frames were overcompressed on default settings
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_MAX_QP, 30);
	}
	else
	{
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_USAGE, AMF_VIDEO_ENCODER_HEVC_USAGE_ULTRA_LOW_LATENCY);
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_TIER, AMF_VIDEO_ENCODER_HEVC_TIER_HIGH);
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_TARGET_BITRATE, bitRateIn);
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_FRAMESIZE, ::AMFConstructSize(width, height));
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_FRAMERATE, ::AMFConstructRate(frameRateIn, 1));

		switch (m_encoderQualityPreset) {
			case SPEED:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET, AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET_SPEED);
				break;
			case BALANCED:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET, AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET_BALANCED);
				break;
			case QUALITY:
			default:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET, AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET_QUALITY);
				break;
		}

		if (m_use10bit) {
			amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_COLOR_BIT_DEPTH, AMF_COLOR_BIT_DEPTH_10);
			amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_PROFILE, AMF_VIDEO_ENCODER_HEVC_PROFILE_MAIN_10);
		} else {
			amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_COLOR_BIT_DEPTH, AMF_COLOR_BIT_DEPTH_8);
			amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_PROFILE, AMF_VIDEO_ENCODER_HEVC_PROFILE_MAIN);
		}

		//No noticable performance difference and should improve subjective quality by allocating more bits to smooth areas
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_ENABLE_VBAQ, true);

		//Fixes rythmic pixelation. I-frames were overcompressed on default settings
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_MAX_QP_I, 30);
	}
	amf::AMFCapsPtr caps;
	amfEncoder->GetCaps(&caps);
	amf::AMFIOCapsPtr iocaps;
	caps->GetOutputCaps(&iocaps);
	amf_int32 formats = iocaps->GetNumOfFormats();
	Debug("Supported formats num: %d\n", formats);
	for (int i = 0; i < formats; i++) {
		amf::AMF_SURFACE_FORMAT format;
		bool isNative;
		iocaps->GetFormatAt(i, &format, &isNative);
		Debug("Accepts format: %d, native: %d", format, isNative);
	}
	Debug("Configured amfEncoder.\n");
	AMF_THROW_IF(amfEncoder->Init(inputFormat, width, height));

	Debug("Initialized amfEncoder.\n");

	return amfEncoder;
}

amf::AMFComponentPtr VideoEncoderVCE::MakeConverter(
	amf::AMF_SURFACE_FORMAT inputFormat, int width, int height, amf::AMF_SURFACE_FORMAT outputFormat
) {
	amf::AMFComponentPtr amfConverter;
	AMF_THROW_IF(g_AMFFactory.GetFactory()->CreateComponent(m_amfContext, AMFVideoConverter, &amfConverter));

	AMF_THROW_IF(amfConverter->SetProperty(AMF_VIDEO_CONVERTER_MEMORY_TYPE, amf::AMF_MEMORY_DX11));
	AMF_THROW_IF(amfConverter->SetProperty(AMF_VIDEO_CONVERTER_OUTPUT_FORMAT, outputFormat));
	AMF_THROW_IF(amfConverter->SetProperty(AMF_VIDEO_CONVERTER_OUTPUT_SIZE, ::AMFConstructSize(width, height)));

	if (m_use10bit) { 
		AMF_THROW_IF(amfConverter->SetProperty(AMF_VIDEO_CONVERTER_INPUT_TRANSFER_CHARACTERISTIC, AMF_COLOR_TRANSFER_CHARACTERISTIC_GAMMA28));
		AMF_THROW_IF(amfConverter->SetProperty(AMF_VIDEO_CONVERTER_INPUT_COLOR_PRIMARIES, AMF_COLOR_PRIMARIES_BT709));
		AMF_THROW_IF(amfConverter->SetProperty(AMF_VIDEO_CONVERTER_INPUT_COLOR_RANGE, AMF_COLOR_RANGE_FULL));
		AMF_THROW_IF(amfConverter->SetProperty(AMF_VIDEO_CONVERTER_INPUT_TONEMAPPING, AMF_VIDEO_CONVERTER_TONEMAPPING_COPY));
	}

	AMF_THROW_IF(amfConverter->Init(inputFormat, width, height));

	Debug("Initialized amfConverter.\n");
	return amfConverter;
}

amf::AMFComponentPtr VideoEncoderVCE::MakePreprocessor(
	amf::AMF_SURFACE_FORMAT inputFormat, int width, int height
) {
	amf::AMFComponentPtr amfPreprocessor;
	AMF_THROW_IF(g_AMFFactory.GetFactory()->CreateComponent(m_amfContext, AMFPreProcessing, &amfPreprocessor));

	AMF_THROW_IF(amfPreprocessor->SetProperty(AMF_PP_ENGINE_TYPE, amf::AMF_MEMORY_DX11));
	AMF_THROW_IF(amfPreprocessor->SetProperty(AMF_PP_ADAPTIVE_FILTER_STRENGTH, m_preProcSigma));
	AMF_THROW_IF(amfPreprocessor->SetProperty(AMF_PP_ADAPTIVE_FILTER_SENSITIVITY, m_preProcTor));

	AMF_THROW_IF(amfPreprocessor->Init(inputFormat, width, height));

	Debug("Initialized amfPreprocessor.\n");
	return amfPreprocessor;
}

void VideoEncoderVCE::Initialize()
{
	const auto &settings = Settings::Instance();

	m_use10bit = settings.m_use10bitEncoder;
	m_usePreProc = settings.m_usePreproc;
	m_encoderQualityPreset = static_cast<EncoderQualityPreset>(settings.m_encoderQualityPreset);
	m_preProcSigma = settings.m_preProcSigma;
	m_preProcTor = settings.m_preProcTor;

	Debug("Initializing VideoEncoderVCE.\n");
	AMF_THROW_IF(g_AMFFactory.Init());

	::amf_increase_timer_precision();

	AMF_THROW_IF(g_AMFFactory.GetFactory()->CreateContext(&m_amfContext));
	AMF_THROW_IF(m_amfContext->InitDX11(m_d3dRender->GetDevice()));

	amf::AMF_SURFACE_FORMAT inFormat = amf::AMF_SURFACE_RGBA;
	if (m_use10bit) {
		inFormat = amf::AMF_SURFACE_P010;
		m_amfComponents.emplace_back(MakeConverter(
			amf::AMF_SURFACE_RGBA, m_renderWidth, m_renderHeight, inFormat
		));
	} else {
		if (m_usePreProc) {
			inFormat = amf::AMF_SURFACE_NV12;
			m_amfComponents.emplace_back(MakeConverter(
				amf::AMF_SURFACE_RGBA, m_renderWidth, m_renderHeight, inFormat
			));
			m_amfComponents.emplace_back(MakePreprocessor(
				inFormat, m_renderWidth, m_renderHeight
			));
		}
	}
	m_amfComponents.emplace_back(MakeEncoder(
		inFormat, m_renderWidth, m_renderHeight, m_codec, m_refreshRate, m_bitrateInMBits
	));

	m_pipeline = new AMFPipeline();
	for (int i = 0; i < m_amfComponents.size() - 1; i++) {
		m_pipeline->Connect(new AMFSolidPipe(
			m_amfComponents[i], m_amfComponents[i + 1]
		));
	}

	m_pipeline->Connect(new AMFPipe(
		m_amfComponents.back(), std::bind(&VideoEncoderVCE::Receive, this, std::placeholders::_1)
	));

	m_pipeline->Start();

	Debug("Successfully initialized VideoEncoderVCE.\n");
}

void VideoEncoderVCE::Shutdown()
{
	Debug("Shutting down VideoEncoderVCE.\n");

	delete m_pipeline;

	for (auto &component : m_amfComponents) {
		component->Release();
		delete component;
	}

	amf_restore_timer_precision();

	if (fpOut) {
		fpOut.close();
	}
	Debug("Successfully shutdown VideoEncoderVCE.\n");
}

void VideoEncoderVCE::Transmit(ID3D11Texture2D *pTexture, uint64_t presentationTime, uint64_t targetTimestampNs, bool insertIDR)
{
	amf::AMFSurfacePtr surface;
	// Surface is cached by AMF.

	if (m_Listener) {
		if (m_Listener->GetStatistics()->CheckBitrateUpdated()) {
			m_bitrateInMBits = m_Listener->GetStatistics()->GetBitrate();
			amf_int64 bitRateIn = m_bitrateInMBits * 1000000L; // in bits
			if (m_codec == ALVR_CODEC_H264)
			{
				m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_TARGET_BITRATE, bitRateIn);
			}
			else
			{
				m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_HEVC_TARGET_BITRATE, bitRateIn);
			}
		}
	}

	AMF_THROW_IF(m_amfContext->AllocSurface(amf::AMF_MEMORY_DX11, amf::AMF_SURFACE_RGBA, m_renderWidth, m_renderHeight, &surface));
	ID3D11Texture2D *textureDX11 = (ID3D11Texture2D*)surface->GetPlaneAt(0)->GetNative(); // no reference counting - do not Release()
	m_d3dRender->GetContext()->CopyResource(textureDX11, pTexture);

	amf_pts start_time = amf_high_precision_clock();
	surface->SetProperty(START_TIME_PROPERTY, start_time);
	surface->SetProperty(FRAME_INDEX_PROPERTY, targetTimestampNs);

	ApplyFrameProperties(surface, insertIDR);

	m_amfComponents.front()->SubmitInput(surface);
}

void VideoEncoderVCE::Receive(AMFDataPtr data)
{
	amf_pts current_time = amf_high_precision_clock();
	amf_pts start_time = 0;
	uint64_t targetTimestampNs;
	data->GetProperty(START_TIME_PROPERTY, &start_time);
	data->GetProperty(FRAME_INDEX_PROPERTY, &targetTimestampNs);

	amf::AMFBufferPtr buffer(data); // query for buffer interface

	if (m_Listener) {
		m_Listener->GetStatistics()->EncodeOutput();
	}

	char *p = reinterpret_cast<char *>(buffer->GetNative());
	int length = static_cast<int>(buffer->GetSize());

	SkipAUD(&p, &length);

	if (fpOut) {
		fpOut.write(p, length);
	}
	if (m_Listener) {
		m_Listener->SendVideo(reinterpret_cast<uint8_t *>(p), length, targetTimestampNs);
	}
}

void VideoEncoderVCE::ApplyFrameProperties(const amf::AMFSurfacePtr &surface, bool insertIDR) {
	switch (m_codec) {
	case ALVR_CODEC_H264:
		// Disable AUD (NAL Type 9) to produce the same stream format as VideoEncoderNVENC.
		surface->SetProperty(AMF_VIDEO_ENCODER_INSERT_AUD, false);
		if (insertIDR) {
			Debug("Inserting IDR frame for H.264.\n");
			surface->SetProperty(AMF_VIDEO_ENCODER_INSERT_SPS, true);
			surface->SetProperty(AMF_VIDEO_ENCODER_INSERT_PPS, true);
			surface->SetProperty(AMF_VIDEO_ENCODER_FORCE_PICTURE_TYPE, AMF_VIDEO_ENCODER_PICTURE_TYPE_IDR);
		}
		break;
	case ALVR_CODEC_H265:
		// This option is ignored. Maybe a bug on AMD driver.
		surface->SetProperty(AMF_VIDEO_ENCODER_HEVC_INSERT_AUD, false);
		if (insertIDR) {
			Debug("Inserting IDR frame for H.265.\n");
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

	if (m_codec != ALVR_CODEC_H265) {
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
