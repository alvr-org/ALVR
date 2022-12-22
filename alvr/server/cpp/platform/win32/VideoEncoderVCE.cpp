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

void AMFPipe::doPassthrough(bool hasQueryTimeout) 
{
	amf::AMFDataPtr data = nullptr;
	if (hasQueryTimeout) {
		AMF_RESULT res = m_amfComponentSrc->QueryOutput(&data);
		if (res == AMF_OK && data) {
			m_receiver(data);
		} else {
			Debug("Failed to get AMF component data. Last status: %d.\n", res);
		}
	} else {
		uint8_t timeout = 1000; // 1s timeout
		AMF_RESULT res = m_amfComponentSrc->QueryOutput(&data);
		while (!data && --timeout != 0) {
			amf_sleep(1);
			res = m_amfComponentSrc->QueryOutput(&data);
		}
		if (data) {
			m_receiver(data);
		} else {
			Debug("Failed to get AMF component data. Last status: %d.\n", res);
		}
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
	: m_pipes()
{}

AMFPipeline::~AMFPipeline() 
{
	for (auto &pipe : m_pipes) 
	{
		delete pipe;
	}
}

void AMFPipeline::Connect(AMFPipePtr pipe) 
{
	m_pipes.emplace_back(pipe);
}

void AMFPipeline::Run(bool hasQueryTimeout)
{
	for (auto &pipe : m_pipes)
	{
		pipe->doPassthrough(hasQueryTimeout);
	}
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
	, m_surfaceFormat(amf::AMF_SURFACE_RGBA)
	, m_use10bit(Settings::Instance().m_use10bitEncoder)
	, m_audByteSequence(nullptr)
	, m_audNalSize(0)
	, m_audHeaderSize(0)
	, m_hasQueryTimeout(false)
{}

VideoEncoderVCE::~VideoEncoderVCE() {
	if (m_audByteSequence) {
		delete[] m_audByteSequence;
		m_audByteSequence = nullptr;
	}
}

amf::AMFComponentPtr VideoEncoderVCE::MakeEncoder(
	amf::AMF_SURFACE_FORMAT inputFormat, int width, int height, int codec, int refreshRate, int bitrateInMbits
) 
{
	const wchar_t *pCodec;

	amf_int32 frameRateIn = refreshRate;
	amf_int64 bitRateIn = bitrateInMbits * 1'000'000L; // in bits

	switch (codec) {
	case ALVR_CODEC_H264:
		if (m_use10bit) {
			throw MakeException("H.264 10-bit encoding is not supported");
		}
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
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_PROFILE, AMF_VIDEO_ENCODER_PROFILE_HIGH);
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_PROFILE_LEVEL, 42);
		switch (Settings::Instance().m_rateControlMode) {
			case ALVR_CBR:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD, AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD_CBR);
				// Required for CBR to work correctly
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_FILLER_DATA_ENABLE, true);
				break;
			case ALVR_VBR:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD, AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD_LATENCY_CONSTRAINED_VBR);
				break;
		}
		
		switch (Settings::Instance().m_entropyCoding) {
			case ALVR_CABAC:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_CABAC_ENABLE, AMF_VIDEO_ENCODER_CABAC);
				break;
			case ALVR_CAVLC:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_CABAC_ENABLE, AMF_VIDEO_ENCODER_CALV);
				break;
		}

		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_TARGET_BITRATE, bitRateIn);
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_PEAK_BITRATE, bitRateIn);
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_FRAMESIZE, ::AMFConstructSize(width, height));
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_FRAMERATE, ::AMFConstructRate(frameRateIn, 1));
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_B_PIC_PATTERN, 0);

		switch (Settings::Instance().m_encoderQualityPreset) {
			case QUALITY:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_QUALITY_PRESET, AMF_VIDEO_ENCODER_QUALITY_PRESET_QUALITY);
				break;
			case BALANCED:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_QUALITY_PRESET, AMF_VIDEO_ENCODER_QUALITY_PRESET_BALANCED);
				break;
			case SPEED:
			default:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_QUALITY_PRESET, AMF_VIDEO_ENCODER_QUALITY_PRESET_SPEED);
				break;
		}

		//No noticable performance difference and should improve subjective quality by allocating more bits to smooth areas
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_ENABLE_VBAQ, true);
		
		//Turns Off IDR/I Frames
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_IDR_PERIOD, 0);

		// Disable AUD to produce the same stream format as VideoEncoderNVENC.
		// FIXME: This option doesn't work in 22.10.3, but works in versions prior 22.5.1
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_INSERT_AUD, false);

		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_VBV_BUFFER_SIZE, bitRateIn / frameRateIn);

		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_MAX_NUM_REFRAMES, 0);
		
		amf::AMFCapsPtr caps;
		if (amfEncoder->GetCaps(&caps) == AMF_OK) {
			caps->GetProperty(AMF_VIDEO_ENCODER_CAPS_QUERY_TIMEOUT_SUPPORT, &m_hasQueryTimeout);
		}
		if (m_hasQueryTimeout) {
			amfEncoder->SetProperty(AMF_VIDEO_ENCODER_QUERY_TIMEOUT, 1000); // 1s timeout
		}
	}
	else
	{
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_USAGE, AMF_VIDEO_ENCODER_HEVC_USAGE_ULTRA_LOW_LATENCY);
		switch (Settings::Instance().m_rateControlMode) {
			case ALVR_CBR:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD, AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD_CBR);
				// Required for CBR to work correctly
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_FILLER_DATA_ENABLE, true);
				break;
			case ALVR_VBR:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD, AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD_LATENCY_CONSTRAINED_VBR);
				break;
		}
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_TARGET_BITRATE, bitRateIn);
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_PEAK_BITRATE, bitRateIn);
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_FRAMESIZE, ::AMFConstructSize(width, height));
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_FRAMERATE, ::AMFConstructRate(frameRateIn, 1));

		switch (Settings::Instance().m_encoderQualityPreset) {
			case QUALITY:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET, AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET_QUALITY);
				break;
			case BALANCED:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET, AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET_BALANCED);
				break;
			case SPEED:
			default:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET, AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET_SPEED);
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
		
		//Turns Off IDR/I Frames
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_NUM_GOPS_PER_IDR, 0);
		//Set infinite GOP length
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_GOP_SIZE, 0);
		
		// Disable AUD to produce the same stream format as VideoEncoderNVENC.
		// FIXME: This option doesn't work in 22.10.3, but works in versions prior 22.5.1
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_INSERT_AUD, false);

		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_VBV_BUFFER_SIZE, bitRateIn / frameRateIn);

		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_MAX_NUM_REFRAMES, 0);
		
		amf::AMFCapsPtr caps;
		if (amfEncoder->GetCaps(&caps) == AMF_OK) {
			caps->GetProperty(AMF_VIDEO_ENCODER_CAPS_HEVC_QUERY_TIMEOUT_SUPPORT, &m_hasQueryTimeout);
		}
		if (m_hasQueryTimeout) {
			amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_QUERY_TIMEOUT, 1000); // 1s timeout
		}
	}

	Debug("Configured %s.\n", pCodec);
	AMF_THROW_IF(amfEncoder->Init(inputFormat, width, height));

	Debug("Initialized %s.\n", pCodec);

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

	AMF_THROW_IF(amfConverter->Init(inputFormat, width, height));

	Debug("Initialized %s.\n", AMFVideoConverter);
	return amfConverter;
}

amf::AMFComponentPtr VideoEncoderVCE::MakePreprocessor(
	amf::AMF_SURFACE_FORMAT inputFormat, int width, int height
) {
	amf::AMFComponentPtr amfPreprocessor;
	AMF_THROW_IF(g_AMFFactory.GetFactory()->CreateComponent(m_amfContext, AMFPreProcessing, &amfPreprocessor));

	AMF_THROW_IF(amfPreprocessor->SetProperty(AMF_PP_ENGINE_TYPE, amf::AMF_MEMORY_DX11));
	AMF_THROW_IF(amfPreprocessor->SetProperty(AMF_PP_ADAPTIVE_FILTER_STRENGTH, Settings::Instance().m_preProcSigma));
	AMF_THROW_IF(amfPreprocessor->SetProperty(AMF_PP_ADAPTIVE_FILTER_SENSITIVITY, Settings::Instance().m_preProcTor));

	AMF_THROW_IF(amfPreprocessor->Init(inputFormat, width, height));

	Debug("Initialized %s.\n", AMFPreProcessing);
	return amfPreprocessor;
}

void VideoEncoderVCE::Initialize()
{
	Debug("Initializing VideoEncoderVCE.\n");
	AMF_THROW_IF(g_AMFFactory.Init());

	LoadAUDByteSequence();

	::amf_increase_timer_precision();

	AMF_THROW_IF(g_AMFFactory.GetFactory()->CreateContext(&m_amfContext));
	AMF_THROW_IF(m_amfContext->InitDX11(m_d3dRender->GetDevice()));

	amf::AMF_SURFACE_FORMAT inFormat = m_surfaceFormat;
	if (m_use10bit) {
		inFormat = amf::AMF_SURFACE_R10G10B10A2;
		m_amfComponents.emplace_back(MakeConverter(
			m_surfaceFormat, m_renderWidth, m_renderHeight, inFormat
		));
	} else {
		if (Settings::Instance().m_usePreproc) {
			inFormat = amf::AMF_SURFACE_NV12;
			m_amfComponents.emplace_back(MakeConverter(
				m_surfaceFormat, m_renderWidth, m_renderHeight, inFormat
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

	m_amfContext->Terminate();
	m_amfContext = NULL;

	g_AMFFactory.Terminate();

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
			amf_int64 bitRateIn = m_bitrateInMBits * 1'000'000L; // in bits
			if (m_codec == ALVR_CODEC_H264)
			{
				m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_TARGET_BITRATE, bitRateIn);
				m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_PEAK_BITRATE, bitRateIn);
				m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_VBV_BUFFER_SIZE, bitRateIn / m_refreshRate);
			}
			else
			{
				m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_HEVC_TARGET_BITRATE, bitRateIn);
				m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_HEVC_PEAK_BITRATE, bitRateIn);
				m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_HEVC_VBV_BUFFER_SIZE, bitRateIn / m_refreshRate);
			}
		}
	}

	AMF_THROW_IF(m_amfContext->AllocSurface(amf::AMF_MEMORY_DX11, m_surfaceFormat, m_renderWidth, m_renderHeight, &surface));
	ID3D11Texture2D *textureDX11 = (ID3D11Texture2D*)surface->GetPlaneAt(0)->GetNative(); // no reference counting - do not Release()
	m_d3dRender->GetContext()->CopyResource(textureDX11, pTexture);

	amf_pts start_time = amf_high_precision_clock();
	surface->SetProperty(START_TIME_PROPERTY, start_time);
	surface->SetProperty(FRAME_INDEX_PROPERTY, targetTimestampNs);

	ApplyFrameProperties(surface, insertIDR);

	m_amfComponents.front()->SubmitInput(surface);
	m_pipeline->Run(m_hasQueryTimeout);
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

	//Fallback in case AUD was not removed by the encoder
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
		// FIXME: This option doesn't work in drivers 22.3.1 - 22.5.1, but works in 22.10.3
		surface->SetProperty(AMF_VIDEO_ENCODER_INSERT_AUD, false);
		if (insertIDR) {
			Debug("Inserting IDR frame for H.264.\n");
			surface->SetProperty(AMF_VIDEO_ENCODER_INSERT_SPS, true);
			surface->SetProperty(AMF_VIDEO_ENCODER_INSERT_PPS, true);
			surface->SetProperty(AMF_VIDEO_ENCODER_FORCE_PICTURE_TYPE, AMF_VIDEO_ENCODER_PICTURE_TYPE_IDR);
		}
		break;
	case ALVR_CODEC_H265:
		// FIXME: This option works with 22.10.3, but may not work with older drivers
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
	default:
		throw MakeException("Invalid video codec");
	}
}

void VideoEncoderVCE::LoadAUDByteSequence() {
	const char H264_AUD_HEADER[] = {0x00, 0x00, 0x00, 0x01, 0x09};
	const char H265_AUD_HEADER[] = {0x00, 0x00, 0x00, 0x01, 0x46};

	switch (m_codec) {
	case ALVR_CODEC_H264:
		m_audHeaderSize = sizeof(H264_AUD_HEADER);
		m_audByteSequence = new char[m_audHeaderSize];
		m_audNalSize = 6;
		std::copy(std::begin(H264_AUD_HEADER), std::end(H264_AUD_HEADER), m_audByteSequence);
		break;
	case ALVR_CODEC_H265:
		m_audHeaderSize = sizeof(H265_AUD_HEADER);
		m_audByteSequence = new char[m_audHeaderSize];
		m_audNalSize = 7;
		std::copy(std::begin(H265_AUD_HEADER), std::end(H265_AUD_HEADER), m_audByteSequence);
		break;
	default:
		throw MakeException("Invalid video codec");
	}
}

void VideoEncoderVCE::SkipAUD(char **buffer, int *length) {
	static const char NAL_HEADER[] = {0x00, 0x00, 0x00, 0x01};

	if (*length < m_audNalSize + sizeof(NAL_HEADER)) {
		return;
	}

	// Check if start with AUD NAL.
	if (memcmp(*buffer, m_audByteSequence, m_audHeaderSize) != 0) {
		return;
	}

	// Check if AUD NAL size is m_audNalSize bytes.
	if (memcmp(*buffer + m_audNalSize, NAL_HEADER, sizeof(NAL_HEADER)) != 0) {
		return;
	}

	*buffer += m_audNalSize;
	*length -= m_audNalSize;
}
