#include "VideoEncoderAMF.h"

#include "alvr_server/Logger.h"
#include "alvr_server/Settings.h"

#define AMF_THROW_IF(expr) {AMF_RESULT res = expr;\
if(res != AMF_OK){throw MakeException("AMF Error %d. %s", res, L#expr);}}

const wchar_t *VideoEncoderAMF::START_TIME_PROPERTY = L"StartTimeProperty";
const wchar_t *VideoEncoderAMF::FRAME_INDEX_PROPERTY = L"FrameIndexProperty";

AMFPipe::AMFPipe(amf::AMFComponentPtr src, AMFDataReceiver receiver)
	: m_amfComponentSrc(src)
	, m_receiver(receiver)
{}

AMFPipe::~AMFPipe()
{
	Debug("AMFPipe::~AMFPipe()  m_amfComponentSrc->Drain\n");
	m_amfComponentSrc->Drain();
}

void AMFPipe::doPassthrough(bool hasQueryTimeout, uint32_t timerResolution)
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
		uint16_t timeout = 1000; // 1s timeout
		AMF_RESULT res = m_amfComponentSrc->QueryOutput(&data);

		timeBeginPeriod(timerResolution);
		while (!data && --timeout != 0) {
			amf_sleep(1);
			res = m_amfComponentSrc->QueryOutput(&data);
		}
		timeEndPeriod(timerResolution);

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
{
	TIMECAPS tc;
	m_timerResolution = timeGetDevCaps(&tc, sizeof(tc)) == TIMERR_NOERROR ? tc.wPeriodMin : 1;
}

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
		pipe->doPassthrough(hasQueryTimeout, m_timerResolution);
	}
}

//
// VideoEncoderAMF
//

VideoEncoderAMF::VideoEncoderAMF(std::shared_ptr<CD3DRender> d3dRender
	, int width, int height)
	: m_d3dRender(d3dRender)
	, m_codec(Settings::Instance().m_codec)
	, m_refreshRate(Settings::Instance().m_refreshRate)
	, m_renderWidth(width)
	, m_renderHeight(height)
	, m_bitrateInMBits(30)
	, m_surfaceFormat(amf::AMF_SURFACE_RGBA)
	, m_use10bit(Settings::Instance().m_use10bitEncoder)
	, m_hasQueryTimeout(false)
{}

VideoEncoderAMF::~VideoEncoderAMF() {}

amf::AMFComponentPtr VideoEncoderAMF::MakeEncoder(
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
	case ALVR_CODEC_HEVC:
		pCodec = AMFVideoEncoder_HEVC;
		break;
	case ALVR_CODEC_AV1:
		pCodec = AMFVideoEncoder_AV1;
		break;
	default:
		throw MakeException("Unsupported video encoding %d", codec);
	}

	amf::AMFComponentPtr amfEncoder;
	// Create encoder component.
	AMF_THROW_IF(g_AMFFactory.GetFactory()->CreateComponent(m_amfContext, pCodec, &amfEncoder));

	switch (codec) {
	case ALVR_CODEC_H264:
	{
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_USAGE, AMF_VIDEO_ENCODER_USAGE_ULTRA_LOW_LATENCY);
      	switch (Settings::Instance().m_h264Profile) {
      	case ALVR_H264_PROFILE_BASELINE:
        	amfEncoder->SetProperty(AMF_VIDEO_ENCODER_PROFILE, AMF_VIDEO_ENCODER_PROFILE_BASELINE);
        	break;
      	case ALVR_H264_PROFILE_MAIN:
        	amfEncoder->SetProperty(AMF_VIDEO_ENCODER_PROFILE, AMF_VIDEO_ENCODER_PROFILE_MAIN);
        	break;
      	default:
      	case ALVR_H264_PROFILE_HIGH:
        	amfEncoder->SetProperty(AMF_VIDEO_ENCODER_PROFILE, AMF_VIDEO_ENCODER_PROFILE_HIGH);
        	break;
      	}
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_PROFILE_LEVEL, 42);
		switch (Settings::Instance().m_rateControlMode) {
			case ALVR_CBR:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD, AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD_CBR);
				// Required for CBR to work correctly
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_FILLER_DATA_ENABLE, Settings::Instance().m_fillerData);
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

		switch (Settings::Instance().m_amdEncoderQualityPreset) {
			case ALVR_QUALITY:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_QUALITY_PRESET, AMF_VIDEO_ENCODER_QUALITY_PRESET_QUALITY);
				break;
			case ALVR_BALANCED:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_QUALITY_PRESET, AMF_VIDEO_ENCODER_QUALITY_PRESET_BALANCED);
				break;
			case ALVR_SPEED:
			default:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_QUALITY_PRESET, AMF_VIDEO_ENCODER_QUALITY_PRESET_SPEED);
				break;
		}

		amf::AMFCapsPtr caps;
		if (amfEncoder->GetCaps(&caps) == AMF_OK) {
			caps->GetProperty(AMF_VIDEO_ENCODER_CAP_PRE_ANALYSIS, &m_hasPreAnalysis);
			caps->GetProperty(AMF_VIDEO_ENCODER_CAPS_QUERY_TIMEOUT_SUPPORT, &m_hasQueryTimeout);
		}

		if (Settings::Instance().m_enablePreAnalysis) {
			if (!Settings::Instance().m_usePreproc || Settings::Instance().m_use10bitEncoder) {
				Warn("Pre-analysis could not be enabled because \"Use preproc\" is not enabled or \"Reduce color banding\" is enabled.");
			} else if (m_hasPreAnalysis) {
				Warn("Enabling h264 pre-analysis. You may experience higher latency when this is enabled.");
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_PRE_ANALYSIS_ENABLE, Settings::Instance().m_enablePreAnalysis);
			} else {
				Warn("Pre-analysis could not be enabled because your GPU does not support it for h264 encoding.");
			}
		}

		// Enable Full Range 
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_FULL_RANGE_COLOR, Settings::Instance().m_useFullRangeEncoding);

		//No noticable performance difference and should improve subjective quality by allocating more bits to smooth areas
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_ENABLE_VBAQ, Settings::Instance().m_enableVbaq);

		// May impact performance but improves quality in high-motion areas
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HIGH_MOTION_QUALITY_BOOST_ENABLE, Settings::Instance().m_enableHmqb);

		//Turns Off IDR/I Frames
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_IDR_PERIOD, 0);

		// Disable AUD to produce the same stream format as VideoEncoderNVENC.
		// FIXME: This option doesn't work in 22.10.3, but works in versions prior 22.5.1
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_INSERT_AUD, false);

		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_VBV_BUFFER_SIZE, bitRateIn / frameRateIn * 1.1);

		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_MAX_NUM_REFRAMES, 0);

		if (m_hasQueryTimeout) {
			amfEncoder->SetProperty(AMF_VIDEO_ENCODER_QUERY_TIMEOUT, 1000); // 1s timeout
		}
	}
	case ALVR_CODEC_HEVC:
	{
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_USAGE, AMF_VIDEO_ENCODER_HEVC_USAGE_ULTRA_LOW_LATENCY);
		switch (Settings::Instance().m_rateControlMode) {
			case ALVR_CBR:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD, AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD_CBR);
				// Required for CBR to work correctly
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_FILLER_DATA_ENABLE, Settings::Instance().m_fillerData);
				break;
			case ALVR_VBR:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD, AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD_LATENCY_CONSTRAINED_VBR);
				break;
		}
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_TARGET_BITRATE, bitRateIn);
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_PEAK_BITRATE, bitRateIn);
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_FRAMESIZE, ::AMFConstructSize(width, height));
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_FRAMERATE, ::AMFConstructRate(frameRateIn, 1));

		switch (Settings::Instance().m_amdEncoderQualityPreset) {
			case ALVR_QUALITY:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET, AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET_QUALITY);
				break;
			case ALVR_BALANCED:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET, AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET_BALANCED);
				break;
			case ALVR_SPEED:
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

		amf::AMFCapsPtr caps;
		if (amfEncoder->GetCaps(&caps) == AMF_OK) {
			caps->GetProperty(AMF_VIDEO_ENCODER_HEVC_CAP_PRE_ANALYSIS, &m_hasPreAnalysis);
			caps->GetProperty(AMF_VIDEO_ENCODER_CAPS_HEVC_QUERY_TIMEOUT_SUPPORT, &m_hasQueryTimeout);
		}

		if (Settings::Instance().m_enablePreAnalysis) {
			if (!Settings::Instance().m_usePreproc || Settings::Instance().m_use10bitEncoder) {
				Warn("Pre-analysis could not be enabled because \"Use preproc\" is not enabled or \"Reduce color banding\" is enabled.");
			} else if (m_hasPreAnalysis) {
				Warn("Enabling HEVC pre-analysis. You may experience higher latency when this is enabled.");
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_PRE_ANALYSIS_ENABLE, Settings::Instance().m_enablePreAnalysis);
			} else {
				Warn("Pre-analysis could not be enabled because your GPU does not support it for HEVC encoding.");
			}
		}

		// Enable Full Range 
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_NOMINAL_RANGE, Settings::Instance().m_useFullRangeEncoding ? AMF_VIDEO_ENCODER_HEVC_NOMINAL_RANGE_FULL : AMF_VIDEO_ENCODER_HEVC_NOMINAL_RANGE_STUDIO);

		//No noticable performance difference and should improve subjective quality by allocating more bits to smooth areas
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_ENABLE_VBAQ, Settings::Instance().m_enableVbaq);

		// May impact performance but improves quality in high-motion areas
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_HIGH_MOTION_QUALITY_BOOST_ENABLE, Settings::Instance().m_enableHmqb);

		//Turns Off IDR/I Frames
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_NUM_GOPS_PER_IDR, 0);
		//Set infinite GOP length
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_GOP_SIZE, 0);

		// Disable AUD to produce the same stream format as VideoEncoderNVENC.
		// FIXME: This option doesn't work in 22.10.3, but works in versions prior 22.5.1
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_INSERT_AUD, false);

		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_VBV_BUFFER_SIZE, bitRateIn / frameRateIn * 1.1);

		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_MAX_NUM_REFRAMES, 0);

		if (m_hasQueryTimeout) {
			amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_QUERY_TIMEOUT, 1000); // 1s timeout
		}
	}
	case ALVR_CODEC_AV1:
	{
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_AV1_USAGE, AMF_VIDEO_ENCODER_AV1_USAGE_ULTRA_LOW_LATENCY);
		switch (Settings::Instance().m_rateControlMode) {
			case ALVR_CBR:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_AV1_RATE_CONTROL_METHOD, AMF_VIDEO_ENCODER_AV1_RATE_CONTROL_METHOD_CBR);
				// Required for CBR to work correctly
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_AV1_FILLER_DATA, Settings::Instance().m_fillerData);
				break;
			case ALVR_VBR:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_AV1_RATE_CONTROL_METHOD, AMF_VIDEO_ENCODER_AV1_RATE_CONTROL_METHOD_LATENCY_CONSTRAINED_VBR);
				break;
		}
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_AV1_TARGET_BITRATE, bitRateIn);
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_AV1_PEAK_BITRATE, bitRateIn);
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_AV1_FRAMESIZE, ::AMFConstructSize(width, height));
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_AV1_FRAMERATE, ::AMFConstructRate(frameRateIn, 1));

		switch (Settings::Instance().m_amdEncoderQualityPreset) {
			case ALVR_QUALITY:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_AV1_QUALITY_PRESET, AMF_VIDEO_ENCODER_AV1_QUALITY_PRESET_QUALITY);
				break;
			case ALVR_BALANCED:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_AV1_QUALITY_PRESET, AMF_VIDEO_ENCODER_AV1_QUALITY_PRESET_BALANCED);
				break;
			case ALVR_SPEED:
			default:
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_AV1_QUALITY_PRESET, AMF_VIDEO_ENCODER_AV1_QUALITY_PRESET_SPEED);
				break;
		}

		if (m_use10bit) {
			amfEncoder->SetProperty(AMF_VIDEO_ENCODER_AV1_COLOR_BIT_DEPTH, AMF_COLOR_BIT_DEPTH_10);
			// There's no separate profile for 10-bit for AV1 (as of AMF v1.4.33). Assumedly MAIN works fine for both.
			amfEncoder->SetProperty(AMF_VIDEO_ENCODER_AV1_PROFILE, AMF_VIDEO_ENCODER_AV1_PROFILE_MAIN);
		} else {
			amfEncoder->SetProperty(AMF_VIDEO_ENCODER_AV1_COLOR_BIT_DEPTH, AMF_COLOR_BIT_DEPTH_8);
			amfEncoder->SetProperty(AMF_VIDEO_ENCODER_AV1_PROFILE, AMF_VIDEO_ENCODER_AV1_PROFILE_MAIN);
		}

		// There is no VBAQ option for AV1. Instead it has CAQ (Content adaptive quantization)
		if (Settings::Instance().m_enableVbaq) {
			amfEncoder->SetProperty(AMF_VIDEO_ENCODER_AV1_AQ_MODE, AMF_VIDEO_ENCODER_AV1_AQ_MODE_CAQ);
		} else {
			amfEncoder->SetProperty(AMF_VIDEO_ENCODER_AV1_AQ_MODE, AMF_VIDEO_ENCODER_AV1_AQ_MODE_NONE);
		}

		amf::AMFCapsPtr caps;
		if (amfEncoder->GetCaps(&caps) == AMF_OK) {
			caps->GetProperty(AMF_VIDEO_ENCODER_AV1_CAP_PRE_ANALYSIS, &m_hasPreAnalysis);
		}
		
		if (Settings::Instance().m_enablePreAnalysis) {
			if (!Settings::Instance().m_usePreproc || Settings::Instance().m_use10bitEncoder) {
				Warn("Pre-analysis could not be enabled because \"Use preproc\" is not enabled or \"Reduce color banding\" is enabled.");
			} else if (m_hasPreAnalysis) {
				Warn("Enabling AV1 pre-analysis. You may experience higher latency when this is enabled.");
				amfEncoder->SetProperty(AMF_VIDEO_ENCODER_AV1_PRE_ANALYSIS_ENABLE, Settings::Instance().m_enablePreAnalysis);
			} else {
				Warn("Pre-analysis could not be enabled because your GPU does not support it for AV1 encoding.");
			}
		}

		// Enable Full Range 
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_AV1_OUTPUT_COLOR_PROFILE, Settings::Instance().m_useFullRangeEncoding ? AMF_VIDEO_CONVERTER_COLOR_PROFILE_JPEG : AMF_VIDEO_CONVERTER_COLOR_PROFILE_UNKNOWN);

		// May impact performance but improves quality in high-motion areas
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_AV1_HIGH_MOTION_QUALITY_BOOST, Settings::Instance().m_enableHmqb);

		// Set infinite GOP length
		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_AV1_GOP_SIZE, 0);

		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_AV1_VBV_BUFFER_SIZE, bitRateIn / frameRateIn * 1.2);

		amfEncoder->SetProperty(AMF_VIDEO_ENCODER_AV1_MAX_NUM_REFRAMES, 0);

		// AV1 assumed always has support for query timeout.
		m_hasQueryTimeout = true;

		if (m_hasQueryTimeout) {
			amfEncoder->SetProperty(AMF_VIDEO_ENCODER_AV1_QUERY_TIMEOUT, 1000); // 1s timeout
		}
	}
	}

	Debug("Configured %s.\n", pCodec);
	AMF_THROW_IF(amfEncoder->Init(inputFormat, width, height));

	Debug("Initialized %s.\n", pCodec);

	return amfEncoder;
}

amf::AMFComponentPtr VideoEncoderAMF::MakeConverter(
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

amf::AMFComponentPtr VideoEncoderAMF::MakePreprocessor(
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

void VideoEncoderAMF::Initialize()
{
	Debug("Initializing VideoEncoderAMF.\n");
	AMF_THROW_IF(g_AMFFactory.Init());

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
		m_amfComponents.back(), std::bind(&VideoEncoderAMF::Receive, this, std::placeholders::_1)
	));

	Debug("Successfully initialized VideoEncoderAMF.\n");
}

void VideoEncoderAMF::Shutdown()
{
	Debug("Shutting down VideoEncoderAMF.\n");

	delete m_pipeline;

	for (auto &component : m_amfComponents) {
		component->Release();
		delete component;
	}

	m_amfContext->Terminate();
	m_amfContext = NULL;

	g_AMFFactory.Terminate();

	if (fpOut) {
		fpOut.close();
	}
	Debug("Successfully shutdown VideoEncoderAMF.\n");
}

void VideoEncoderAMF::Transmit(ID3D11Texture2D *pTexture, uint64_t presentationTime, uint64_t targetTimestampNs, bool insertIDR)
{
	amf::AMFSurfacePtr surface;
	// Surface is cached by AMF.

	auto params = GetDynamicEncoderParams();
	if (params.updated) {
		amf_int64 bitRateIn = params.bitrate_bps / params.framerate * m_refreshRate; // in bits
		if (m_codec == ALVR_CODEC_H264)
		{
			m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_TARGET_BITRATE, bitRateIn);
			m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_PEAK_BITRATE, bitRateIn);
			m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_VBV_BUFFER_SIZE, bitRateIn / m_refreshRate * 1.1);
		}
		else
		{
			m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_HEVC_TARGET_BITRATE, bitRateIn);
			m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_HEVC_PEAK_BITRATE, bitRateIn);
			m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_HEVC_VBV_BUFFER_SIZE, bitRateIn / m_refreshRate * 1.1);
		}

		if (Settings::Instance().m_amdBitrateCorruptionFix) {
			RequestIDR();
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

void VideoEncoderAMF::Receive(AMFDataPtr data)
{
	amf_pts current_time = amf_high_precision_clock();
	amf_pts start_time = 0;
	uint64_t targetTimestampNs;
	data->GetProperty(START_TIME_PROPERTY, &start_time);
	data->GetProperty(FRAME_INDEX_PROPERTY, &targetTimestampNs);

	amf::AMFBufferPtr buffer(data); // query for buffer interface

	char *p = reinterpret_cast<char *>(buffer->GetNative());
	int length = static_cast<int>(buffer->GetSize());

	if (fpOut) {
		fpOut.write(p, length);
	}

	uint64_t type;
	bool isIdr;
	if(m_codec == ALVR_CODEC_H264)
	{
		data->GetProperty(AMF_VIDEO_ENCODER_OUTPUT_DATA_TYPE, &type);
		isIdr = type == AMF_VIDEO_ENCODER_OUTPUT_DATA_TYPE_IDR;
	}
	else
	{
		data->GetProperty(AMF_VIDEO_ENCODER_HEVC_OUTPUT_DATA_TYPE, &type);
		isIdr = type == AMF_VIDEO_ENCODER_HEVC_OUTPUT_DATA_TYPE_IDR;
	}

	ParseFrameNals(m_codec, reinterpret_cast<uint8_t *>(p), length, targetTimestampNs, isIdr);
}

void VideoEncoderAMF::ApplyFrameProperties(const amf::AMFSurfacePtr &surface, bool insertIDR) {
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
	case ALVR_CODEC_HEVC:
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
	case ALVR_CODEC_AV1:
		if (insertIDR) {
			Debug("Inserting IDR frame for AV1.\n");
			surface->SetProperty(AMF_VIDEO_ENCODER_AV1_FORCE_INSERT_SEQUENCE_HEADER, true);
			surface->SetProperty(AMF_VIDEO_ENCODER_AV1_FORCE_FRAME_TYPE, AMF_VIDEO_ENCODER_AV1_FORCE_FRAME_TYPE_KEY);
		}
		break;
	default:
		throw MakeException("Invalid video codec");
	}
}
