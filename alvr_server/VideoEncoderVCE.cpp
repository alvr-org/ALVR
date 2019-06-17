#include "VideoEncoderVCE.h"

#define AMF_THROW_IF(expr) {AMF_RESULT res = expr;\
if(res != AMF_OK){throw MakeException(L"AMF Error %d. %s", res, L#expr);}}

const wchar_t *VideoEncoderVCE::START_TIME_PROPERTY = L"StartTimeProperty";
const wchar_t *VideoEncoderVCE::VIDEO_FRAME_INDEX_PROPERTY = L"VideoFrameIndexProperty";
const wchar_t *VideoEncoderVCE::TRACKING_FRAME_INDEX_PROPERTY = L"TrackingFrameIndexProperty";

//
// AMFTextureEncoder
//

AMFTextureEncoder::AMFTextureEncoder(const amf::AMFContextPtr &amfContext
	, int codec, int width, int height, int refreshRate, int bitrateInMbits
	, amf::AMF_SURFACE_FORMAT inputFormat
	, AMFTextureReceiver receiver) : mReceiver(receiver)
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
		throw MakeException(L"Unsupported video encoding %d", codec);
	}

	// Create encoder component.
	AMF_THROW_IF(g_AMFFactory.GetFactory()->CreateComponent(amfContext, pCodec, &mEncoder));

	if (codec == ALVR_CODEC_H264)
	{
		mEncoder->SetProperty(AMF_VIDEO_ENCODER_USAGE, AMF_VIDEO_ENCODER_USAGE_ULTRA_LOW_LATENCY);

		mEncoder->SetProperty(AMF_VIDEO_ENCODER_B_PIC_PATTERN, 0);
		mEncoder->SetProperty(AMF_VIDEO_ENCODER_QUALITY_PRESET, AMF_VIDEO_ENCODER_QUALITY_PRESET_QUALITY);

		mEncoder->SetProperty(AMF_VIDEO_ENCODER_TARGET_BITRATE, bitRateIn);
		mEncoder->SetProperty(AMF_VIDEO_ENCODER_FRAMESIZE, ::AMFConstructSize(width, height));
		mEncoder->SetProperty(AMF_VIDEO_ENCODER_FRAMERATE, ::AMFConstructRate(frameRateIn, 1));

		//m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_PROFILE, AMF_VIDEO_ENCODER_PROFILE_HIGH);
		//m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_PROFILE_LEVEL, 51);
	}
	else
	{
		mEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_USAGE, AMF_VIDEO_ENCODER_HEVC_USAGE_ULTRA_LOW_LATENCY);

		mEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET, AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET_QUALITY);

		mEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_TARGET_BITRATE, bitRateIn);
		mEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_FRAMESIZE, ::AMFConstructSize(width, height));
		mEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_FRAMERATE, ::AMFConstructRate(frameRateIn, 1));

		//m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_TIER, AMF_VIDEO_ENCODER_HEVC_TIER_HIGH);
		//m_amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_PROFILE_LEVEL, AMF_LEVEL_5);
	}
	AMF_THROW_IF(mEncoder->Init(inputFormat, width, height));

	Log(L"Initialized AMFTextureEncoder.");
}

AMFTextureEncoder::~AMFTextureEncoder()
{
}

void AMFTextureEncoder::Start()
{
	mThread = new std::thread(&AMFTextureEncoder::Run, this);
}

void AMFTextureEncoder::Shutdown()
{
	Log(L"AMFTextureEncoder::Shutdown() m_amfEncoder->Drain");
	mEncoder->Drain();
	Log(L"AMFTextureEncoder::Shutdown() m_thread->join");
	mThread->join();
	Log(L"AMFTextureEncoder::Shutdown() joined.");
	delete mThread;
	mThread = NULL;
}

void AMFTextureEncoder::Submit(amf::AMFData *data)
{
	while (true)
	{
		Log(L"AMFTextureEncoder::Submit.");
		auto res = mEncoder->SubmitInput(data);
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
	Log(L"Start AMFTextureEncoder thread. Thread Id=%d", GetCurrentThreadId());
	amf::AMFDataPtr data;
	while (true)
	{
		auto res = mEncoder->QueryOutput(&data);
		if (res == AMF_EOF)
		{
			Log(L"m_amfEncoder->QueryOutput returns AMF_EOF.");
			return;
		}

		if (data != NULL)
		{
			mReceiver(data);
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
	, AMFTextureReceiver receiver) : mReceiver(receiver)
{
	AMF_THROW_IF(g_AMFFactory.GetFactory()->CreateComponent(amfContext, AMFVideoConverter, &mConverter));

	AMF_THROW_IF(mConverter->SetProperty(AMF_VIDEO_CONVERTER_MEMORY_TYPE, amf::AMF_MEMORY_DX11));
	AMF_THROW_IF(mConverter->SetProperty(AMF_VIDEO_CONVERTER_OUTPUT_FORMAT, outputFormat));
	AMF_THROW_IF(mConverter->SetProperty(AMF_VIDEO_CONVERTER_OUTPUT_SIZE, ::AMFConstructSize(width, height)));

	AMF_THROW_IF(mConverter->Init(inputFormat, width, height));

	Log(L"Initialized AMFTextureConverter.");
}

AMFTextureConverter::~AMFTextureConverter()
{
}

void AMFTextureConverter::Start()
{
	mThread = new std::thread(&AMFTextureConverter::Run, this);
}

void AMFTextureConverter::Shutdown()
{
	Log(L"AMFTextureConverter::Shutdown() m_amfConverter->Drain");
	mConverter->Drain();
	Log(L"AMFTextureConverter::Shutdown() m_thread->join");
	mThread->join();
	Log(L"AMFTextureConverter::Shutdown() joined.");
	delete mThread;
	mThread = NULL;
}

void AMFTextureConverter::Submit(amf::AMFData *data)
{
	while (true)
	{
		Log(L"AMFTextureConverter::Submit.");
		auto res = mConverter->SubmitInput(data);
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
	Log(L"Start AMFTextureConverter thread. Thread Id=%d", GetCurrentThreadId());
	amf::AMFDataPtr data;
	while (true)
	{
		auto res = mConverter->QueryOutput(&data);
		if (res == AMF_EOF)
		{
			Log(L"m_amfConverter->QueryOutput returns AMF_EOF.");
			return;
		}

		if (data != NULL)
		{
			mReceiver(data);
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
	, std::shared_ptr<Listener> listener)
	: mD3DRender(d3dRender)
	, mListener(listener)
	, mCodec(Settings::Instance().mCodec)
	, mRefreshRate(Settings::Instance().mRefreshRate)
	, mRenderWidth(Settings::Instance().mRenderWidth)
	, mRenderHeight(Settings::Instance().mRenderHeight)
	, mBitrate(Settings::Instance().mEncodeBitrate)
{
}

VideoEncoderVCE::~VideoEncoderVCE()
{}

void VideoEncoderVCE::Initialize()
{
	Log(L"Initializing VideoEncoderVCE.");
	AMF_THROW_IF(g_AMFFactory.Init());

	::amf_increase_timer_precision();

	AMF_THROW_IF(g_AMFFactory.GetFactory()->CreateContext(&mContext));
	AMF_THROW_IF(mContext->InitDX11(mD3DRender->GetDevice()));

	mEncoder = std::make_shared<AMFTextureEncoder>(mContext
		, mCodec, mRenderWidth, mRenderHeight, mRefreshRate, static_cast<int>(mBitrate.toMiBits())
		, ENCODER_INPUT_FORMAT, std::bind(&VideoEncoderVCE::Receive, this, std::placeholders::_1));
	mConverter = std::make_shared<AMFTextureConverter>(mContext
		, mRenderWidth, mRenderHeight
		, CONVERTER_INPUT_FORMAT, ENCODER_INPUT_FORMAT
		, std::bind(&AMFTextureEncoder::Submit, mEncoder.get(), std::placeholders::_1));

	mEncoder->Start();
	mConverter->Start();

	//
	// Initialize debug video output
	//

	if (Settings::Instance().mDebugCaptureOutput) {
		mOutput = std::ofstream(Settings::Instance().GetVideoOutput(), std::ios::out | std::ios::binary);
		if (!mOutput)
		{
			Log(L"Unable to open output file %hs", Settings::Instance().GetVideoOutput().c_str());
		}
	}

	Log(L"Successfully initialized VideoEncoderVCE.");
}

void VideoEncoderVCE::Reconfigure(int refreshRate, int renderWidth, int renderHeight, Bitrate bitrate)
{
	if ((refreshRate != 0 && refreshRate != mRefreshRate) ||
		(renderWidth != 0 && renderWidth != mRenderWidth) ||
		(renderHeight != 0 && renderHeight != mRenderHeight) ||
		(bitrate.toBits() != 0 && bitrate.toBits() != mBitrate.toBits())) {

		Log(L"VideoEncoderVCE: Start to reconfigure. (%dHz %dx%d %dMbits) -> (%dHz %dx%d %dMbits)"
			, mRefreshRate, mRenderWidth, mRenderHeight, mBitrate.toMiBits()
			, refreshRate, renderWidth, renderHeight, bitrate.toMiBits()
		);

		try {
			Shutdown();

			if (refreshRate != 0) {
				mRefreshRate = refreshRate;
			}
			if (renderWidth != 0) {
				mRenderWidth = renderWidth;
			}
			if (renderHeight != 0) {
				mRenderHeight = renderHeight;
			}
			if (bitrate.toBits() != 0) {
				mBitrate = bitrate;
			}

			Initialize();
		}
		catch (Exception &e) {
			FatalLog(L"VideoEncoderVCE: Failed to reconfigure. %hs"
				, e.what()
			);
			return;
		}
 	}
}

void VideoEncoderVCE::Shutdown()
{
	Log(L"Shutting down VideoEncoderVCE.");

	mEncoder->Shutdown();
	mConverter->Shutdown();

	amf_restore_timer_precision();

	if (mOutput) {
		mOutput.close();
	}
	Log(L"Successfully shutdown VideoEncoderVCE.");
}

void VideoEncoderVCE::Transmit(ID3D11Texture2D *pTexture, uint64_t presentationTime, uint64_t videoFrameIndex, uint64_t trackingFrameIndex, uint64_t clientTime, bool insertIDR)
{
	amf::AMFSurfacePtr surface;
	// Surface is cached by AMF.
	AMF_THROW_IF(mContext->AllocSurface(amf::AMF_MEMORY_DX11, CONVERTER_INPUT_FORMAT, mRenderWidth, mRenderHeight, &surface));
	ID3D11Texture2D *textureDX11 = (ID3D11Texture2D*)surface->GetPlaneAt(0)->GetNative(); // no reference counting - do not Release()
	mD3DRender->GetContext()->CopyResource(textureDX11, pTexture);

	amf_pts start_time = amf_high_precision_clock();
	surface->SetProperty(START_TIME_PROPERTY, start_time);
	surface->SetProperty(VIDEO_FRAME_INDEX_PROPERTY, videoFrameIndex);
	surface->SetProperty(TRACKING_FRAME_INDEX_PROPERTY, trackingFrameIndex);

	ApplyFrameProperties(surface, insertIDR);

	Log(L"Submit surface. frameIndex=%llu", trackingFrameIndex);
	mConverter->Submit(surface);
}

void VideoEncoderVCE::Receive(amf::AMFData *data)
{
	amf_pts current_time = amf_high_precision_clock();
	amf_pts start_time = 0;
	uint64_t videoFrameIndex, trackingFrameIndex;
	data->GetProperty(START_TIME_PROPERTY, &start_time);
	data->GetProperty(VIDEO_FRAME_INDEX_PROPERTY, &videoFrameIndex);
	data->GetProperty(TRACKING_FRAME_INDEX_PROPERTY, &trackingFrameIndex);

	amf::AMFBufferPtr buffer(data); // query for buffer interface

	Log(L"VCE encode latency: %.4f ms. Size=%d bytes trackingFrameIndex=%llu", double(current_time - start_time) / (double)MILLISEC_TIME, (int)buffer->GetSize()
		, trackingFrameIndex);

	if (mListener) {
		mListener->GetStatistics()->EncodeOutput((current_time - start_time) / MICROSEC_TIME);
	}

	char *p = reinterpret_cast<char *>(buffer->GetNative());
	int length = static_cast<int>(buffer->GetSize());

	SkipAUD(&p, &length);

	if (mOutput) {
		mOutput.write(p, length);
	}
	if (mListener) {
		mListener->SendVideo(reinterpret_cast<uint8_t *>(p), length, videoFrameIndex, trackingFrameIndex);
	}
}

void VideoEncoderVCE::ApplyFrameProperties(const amf::AMFSurfacePtr &surface, bool insertIDR) {
	switch (mCodec) {
	case ALVR_CODEC_H264:
		// Disable AUD (NAL Type 9) to produce the same stream format as VideoEncoderNVENC.
		surface->SetProperty(AMF_VIDEO_ENCODER_INSERT_AUD, false);
		if (insertIDR) {
			Log(L"Inserting IDR frame for H.264.");
			surface->SetProperty(AMF_VIDEO_ENCODER_INSERT_SPS, true);
			surface->SetProperty(AMF_VIDEO_ENCODER_INSERT_PPS, true);
			surface->SetProperty(AMF_VIDEO_ENCODER_FORCE_PICTURE_TYPE, AMF_VIDEO_ENCODER_PICTURE_TYPE_IDR);
		}
		break;
	case ALVR_CODEC_H265:
		// This option is ignored. Maybe a bug on AMD driver.
		surface->SetProperty(AMF_VIDEO_ENCODER_HEVC_INSERT_AUD, false);
		if (insertIDR) {
			Log(L"Inserting IDR frame for H.265.");
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

	if (mCodec != ALVR_CODEC_H265) {
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
