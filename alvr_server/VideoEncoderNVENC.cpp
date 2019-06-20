
#include "VideoEncoderNVENC.h"
#include "..\NvEnc\NvCodecUtils.h"
#include "..\NvEnc\nvencoderclioptions.h"

VideoEncoderNVENC::VideoEncoderNVENC(std::shared_ptr<CD3DRender> pD3DRender
	, std::shared_ptr<Listener> listener, bool useNV12)
	: mD3DRender(pD3DRender)
	, mFrame(0)
	, mListener(listener)
	, mUseNV12(useNV12)
	, mCodec(Settings::Instance().mCodec)
	, mRefreshRate(Settings::Instance().mRefreshRate)
	, mRenderWidth(Settings::Instance().mRenderWidth)
	, mRenderHeight(Settings::Instance().mRenderHeight)
	, mBitrate(Settings::Instance().mEncodeBitrate)
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
	if (mUseNV12) {
		format = NV_ENC_BUFFER_FORMAT_NV12;
	}

	Log(L"Initializing CNvEncoder. Width=%d Height=%d Format=%d (useNV12:%d)", mRenderWidth, mRenderHeight
		, format, mUseNV12);

	if (mUseNV12) {
		try {
			mEncoder = std::make_shared<NvTextureEncoderCuda>(mD3DRender->GetDevice(), mRenderWidth, mRenderHeight, format, 0);
		}
		catch (NVENCException e) {
			throw MakeException(L"NvEnc NvEncoderCuda failed. Code=%d %hs", e.getErrorCode(), e.what());
		}
	}
	else {
		try {
			mEncoder = std::make_shared<NvTextureEncoderD3D11>(mD3DRender->GetDevice(), mRenderWidth, mRenderHeight, format, 0);
		}
		catch (NVENCException e) {
			throw MakeException(L"NvEnc NvEncoderD3D11 failed. Code=%d %hs", e.getErrorCode(), e.what());
		}
	}

	NV_ENC_INITIALIZE_PARAMS initializeParams = { NV_ENC_INITIALIZE_PARAMS_VER };
	NV_ENC_CONFIG encodeConfig = { NV_ENC_CONFIG_VER };
	initializeParams.encodeConfig = &encodeConfig;

	FillEncodeConfig(initializeParams, mRefreshRate, mRenderWidth, mRenderHeight, mBitrate);

	try {
		mEncoder->CreateEncoder(&initializeParams);
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

	if (Settings::Instance().mDebugCaptureOutput) {
		mOutput = std::ofstream(Settings::Instance().GetVideoOutput(), std::ios::out | std::ios::binary);
		if (!mOutput)
		{
			Log(L"unable to open output file %hs", Settings::Instance().GetVideoOutput().c_str());
		}
	}

	Log(L"CNvEncoder is successfully initialized.");
}

void VideoEncoderNVENC::Reconfigure(int refreshRate, int renderWidth, int renderHeight, Bitrate bitrate)
{
	if ((refreshRate != 0 && refreshRate != mRefreshRate) ||
		(renderWidth != 0 && renderWidth != mRenderWidth) ||
		(renderHeight != 0 && renderHeight != mRenderHeight) ||
		(bitrate.toBits() != 0 && bitrate.toBits() != mBitrate.toBits())) {
		NV_ENC_RECONFIGURE_PARAMS reconfigureParams = { NV_ENC_RECONFIGURE_PARAMS_VER };
		NV_ENC_CONFIG encodeConfig = { NV_ENC_CONFIG_VER };

		reconfigureParams.resetEncoder = 1; // Needed?
		reconfigureParams.forceIDR = 1;
		reconfigureParams.reInitEncodeParams.version = NV_ENC_INITIALIZE_PARAMS_VER;
		reconfigureParams.reInitEncodeParams.encodeConfig = &encodeConfig;

		FillEncodeConfig(reconfigureParams.reInitEncodeParams, refreshRate, renderWidth, renderHeight, bitrate);

		reconfigureParams.reInitEncodeParams.maxEncodeWidth = mRenderWidth;
		reconfigureParams.reInitEncodeParams.maxEncodeHeight = mRenderHeight;

		bool ret = false;
		try {
			ret = mEncoder->Reconfigure(&reconfigureParams);
		}
		catch (NVENCException e) {
			FatalLog(L"NvEnc Reconfigure failed with exception. Code=%d %hs. (%dHz %dx%d %dMbits) -> (%dHz %dx%d %dMbits)", e.getErrorCode(), e.what()
				, mRefreshRate, mRenderWidth, mRenderHeight, mBitrate.toMiBits()
				, refreshRate, renderWidth, renderHeight, bitrate.toMiBits()
			);
			return;
		}
		if (!ret) {
			FatalLog(L"NvEnc Reconfigure failed. Return code=%d. (%dHz %dx%d %dMbits) -> (%dHz %dx%d %dMbits)", ret
				, mRefreshRate, mRenderWidth, mRenderHeight, mBitrate.toMiBits()
				, refreshRate, renderWidth, renderHeight, bitrate.toMiBits()
			);
			return;
		}
		Log(L"NvEnc Reconfigure succeeded. (%dHz %dx%d %dMbits) -> (%dHz %dx%d %dMbits)"
			, mRefreshRate, mRenderWidth, mRenderHeight, mBitrate.toMiBits()
			, refreshRate, renderWidth, renderHeight, bitrate.toMiBits()
		);

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
	}
}

void VideoEncoderNVENC::Shutdown()
{
	std::vector<std::vector<uint8_t>> vPacket;
	mEncoder->EndEncode(vPacket);

	for (std::vector<uint8_t> &packet : vPacket)
	{
		if (mOutput) {
			mOutput.write(reinterpret_cast<char*>(packet.data()), packet.size());
		}
	}
	mEncoder->DestroyEncoder();
	mEncoder.reset();

	Log(L"CNvEncoder::Shutdown");

	if (mOutput) {
		mOutput.close();
	}
}

void VideoEncoderNVENC::Transmit(ID3D11Texture2D *pTexture, uint64_t presentationTime, uint64_t videoFrameIndex, uint64_t trackingFrameIndex, uint64_t clientTime, bool insertIDR)
{
	std::vector<std::vector<uint8_t>> vPacket;

	NV_ENC_PIC_PARAMS picParams = {};
	if (insertIDR) {
		picParams.encodePicFlags = NV_ENC_PIC_FLAG_FORCEIDR;
	}
	Log(L"Frame[%llu:%llu] Encoding frame. Type=%s", videoFrameIndex, trackingFrameIndex, insertIDR ? "IDR-Frame" : "P-Frame");
	// To invalidate reference frame when frame dropped.
	picParams.inputTimeStamp = videoFrameIndex;
	mEncoder->EncodeFrame(vPacket, &picParams, mD3DRender->GetContext(), pTexture);

	Log(L"Frame[%llu:%llu] Encoding done. Tracking info delay: %lld us Encoding delay=%lld us VideoFrameIndex=%llu", videoFrameIndex, trackingFrameIndex
		, GetTimestampUs() - mListener->clientToServerTime(clientTime)
		, GetTimestampUs() - presentationTime);

	if (mListener) {
		mListener->GetStatistics()->EncodeOutput(GetTimestampUs() - presentationTime);
	}

	mFrame += (int)vPacket.size();
	for (std::vector<uint8_t> &packet : vPacket)
	{
		if (mOutput) {
			mOutput.write(reinterpret_cast<char*>(packet.data()), packet.size());
		}
		if (mListener) {
			mListener->SendVideo(packet.data(), (int)packet.size(), videoFrameIndex, trackingFrameIndex);
		}
	}

	if (Settings::Instance().mDebugFrameOutput) {
		if (!mUseNV12) {
			SaveDebugOutput(mD3DRender, vPacket, pTexture, videoFrameIndex);
		}
	}
}

void VideoEncoderNVENC::InvalidateReferenceFrame(uint64_t videoFrameIndex)
{
	if (!mSupportsReferenceFrameInvalidation) {
		return;
	}
	Log(L"Invalidate reference frame: %llu", videoFrameIndex);
	try {
		mEncoder->InvalidateRefFrames(videoFrameIndex);
	}
	catch (NVENCException &e) {
		Log(L"Failed to invalidate reference frame. Code=%d %hs", e.getErrorCode(), e.what());
	}
}

void VideoEncoderNVENC::FillEncodeConfig(NV_ENC_INITIALIZE_PARAMS &initializeParams, int refreshRate, int renderWidth, int renderHeight, Bitrate bitrate)
{
	auto &encodeConfig = *initializeParams.encodeConfig;
	GUID EncoderGUID = mCodec == ALVR_CODEC_H264 ? NV_ENC_CODEC_H264_GUID : NV_ENC_CODEC_HEVC_GUID;

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

	mEncoder->CreateDefaultEncoderParams(&initializeParams, EncoderGUID, NV_ENC_PRESET_LOW_LATENCY_HQ_GUID);

	initializeParams.encodeWidth  = initializeParams.darWidth  = renderWidth;
	initializeParams.encodeHeight = initializeParams.darHeight = renderHeight;
	initializeParams.frameRateNum = refreshRate;
	initializeParams.frameRateDen = 1;

	// Use reference frame invalidation to faster recovery from frame loss if supported.
	mSupportsReferenceFrameInvalidation = mEncoder->GetCapabilityValue(EncoderGUID, NV_ENC_CAPS_SUPPORT_REF_PIC_INVALIDATION);
	bool supportsIntraRefresh = mEncoder->GetCapabilityValue(EncoderGUID, NV_ENC_CAPS_SUPPORT_INTRA_REFRESH);
	Log(L"VideoEncoderNVENC: SupportsReferenceFrameInvalidation: %d", mSupportsReferenceFrameInvalidation);
	Log(L"VideoEncoderNVENC: SupportsIntraRefresh: %d", supportsIntraRefresh);

	// 16 is recommended when using reference frame invalidation. But it has caused bad visual quality.
	// Now, use 0 (use default).
	int maxNumRefFrames = 0;

	if (mCodec == ALVR_CODEC_H264) {
		auto &config = encodeConfig.encodeCodecConfig.h264Config;
		config.repeatSPSPPS = 1;
		if (supportsIntraRefresh) {
			config.enableIntraRefresh = 1;
			// Do intra refresh every 10sec.
			config.intraRefreshPeriod = refreshRate * 10;
			config.intraRefreshCnt = refreshRate;
		}
		config.maxNumRefFrames = maxNumRefFrames;
		config.idrPeriod = NVENC_INFINITE_GOPLENGTH;
	}
	else {
		auto &config = encodeConfig.encodeCodecConfig.hevcConfig;
		config.repeatSPSPPS = 1;
		if (supportsIntraRefresh) {
			config.enableIntraRefresh = 1;
			// Do intra refresh every 10sec.
			config.intraRefreshPeriod = refreshRate * 10;
			config.intraRefreshCnt = refreshRate;
		}
		config.maxNumRefFramesInDPB = maxNumRefFrames;
		config.idrPeriod = NVENC_INFINITE_GOPLENGTH;
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
	encodeConfig.gopLength = NVENC_INFINITE_GOPLENGTH;
	encodeConfig.frameIntervalP = 1;

	// NV_ENC_PARAMS_RC_CBR_HQ is equivalent to NV_ENC_PARAMS_RC_2_PASS_FRAMESIZE_CAP.
	//encodeConfig.rcParams.rateControlMode = NV_ENC_PARAMS_RC_CBR_LOWDELAY_HQ;// NV_ENC_PARAMS_RC_CBR_HQ;
	encodeConfig.rcParams.rateControlMode = NV_ENC_PARAMS_RC_CBR_HQ;
	uint32_t maxFrameSize = static_cast<uint32_t>(bitrate.toBits() / refreshRate);
	Log(L"VideoEncoderNVENC: maxFrameSize=%d bits", maxFrameSize);
	encodeConfig.rcParams.vbvBufferSize = maxFrameSize;
	encodeConfig.rcParams.vbvInitialDelay = maxFrameSize;
	encodeConfig.rcParams.maxBitRate = static_cast<uint32_t>(bitrate.toBits());
	encodeConfig.rcParams.averageBitRate = static_cast<uint32_t>(bitrate.toBits());
}
