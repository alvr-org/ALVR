
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
	, mBitrateInMBits(Settings::Instance().mEncodeBitrate.toMiBits())
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
	GUID EncoderGUID = mCodec == ALVR_CODEC_H264 ? NV_ENC_CODEC_H264_GUID : NV_ENC_CODEC_HEVC_GUID;
	mEncoder->CreateDefaultEncoderParams(&initializeParams, EncoderGUID, NV_ENC_PRESET_LOW_LATENCY_HQ_GUID);

	if (mCodec == ALVR_CODEC_H264) {
		initializeParams.encodeConfig->encodeCodecConfig.h264Config.repeatSPSPPS = 1;
	}
	else {
		initializeParams.encodeConfig->encodeCodecConfig.hevcConfig.repeatSPSPPS = 1;
	}

	initializeParams.encodeConfig->rcParams.rateControlMode = NV_ENC_PARAMS_RC_CBR_LOWDELAY_HQ;
	initializeParams.frameRateNum = mRefreshRate;
	initializeParams.encodeConfig->rcParams.maxBitRate =
		initializeParams.encodeConfig->rcParams.averageBitRate = mBitrateInMBits * 1000 * 1000;
	// Disable automatic IDR insertion by NVENC. We need to manually insert IDR when packet is dropped.
	initializeParams.encodeConfig->gopLength = NVENC_INFINITE_GOPLENGTH;

	//initializeParams.maxEncodeWidth = 3840;
	//initializeParams.maxEncodeHeight = 2160;

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

void VideoEncoderNVENC::Reconfigure(int refreshRate, int renderWidth, int renderHeight, int bitrateInMBits)
{
	if ((refreshRate != 0 && refreshRate != mRefreshRate) ||
		(renderWidth != 0 && renderWidth != mRenderWidth) ||
		(renderHeight != 0 && renderHeight != mRenderHeight) ||
		(bitrateInMBits != 0 && bitrateInMBits != mBitrateInMBits)) {
		NV_ENC_RECONFIGURE_PARAMS reconfigureParams = { NV_ENC_RECONFIGURE_PARAMS_VER };
		NV_ENC_CONFIG encodeConfig = { NV_ENC_CONFIG_VER };

		reconfigureParams.resetEncoder = 1; // Needed?
		reconfigureParams.forceIDR = 1;
		reconfigureParams.reInitEncodeParams.version = NV_ENC_INITIALIZE_PARAMS_VER;
		reconfigureParams.reInitEncodeParams.encodeConfig = &encodeConfig;

		GUID EncoderGUID = mCodec == ALVR_CODEC_H264 ? NV_ENC_CODEC_H264_GUID : NV_ENC_CODEC_HEVC_GUID;

		mEncoder->CreateDefaultEncoderParams(&reconfigureParams.reInitEncodeParams, EncoderGUID, NV_ENC_PRESET_LOW_LATENCY_HQ_GUID);

		if (mCodec == ALVR_CODEC_H264) {
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
			ret = mEncoder->Reconfigure(&reconfigureParams);
		}
		catch (NVENCException e) {
			FatalLog(L"NvEnc Reconfigure failed with exception. Code=%d %hs. (%dHz %dx%d %dMbits) -> (%dHz %dx%d %dMbits)", e.getErrorCode(), e.what()
				, mRefreshRate, mRenderWidth, mRenderHeight, mBitrateInMBits
				, refreshRate, renderWidth, renderHeight, bitrateInMBits
			);
			return;
		}
		if (!ret) {
			FatalLog(L"NvEnc Reconfigure failed. Return code=%d. (%dHz %dx%d %dMbits) -> (%dHz %dx%d %dMbits)", ret
				, mRefreshRate, mRenderWidth, mRenderHeight, mBitrateInMBits
				, refreshRate, renderWidth, renderHeight, bitrateInMBits
			);
			return;
		}
		Log(L"NvEnc Reconfigure succeeded. (%dHz %dx%d %dMbits) -> (%dHz %dx%d %dMbits)"
			, mRefreshRate, mRenderWidth, mRenderHeight, mBitrateInMBits
			, refreshRate, renderWidth, renderHeight, bitrateInMBits
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
		if (bitrateInMBits != 0) {
			mBitrateInMBits = bitrateInMBits;
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

void VideoEncoderNVENC::Transmit(ID3D11Texture2D *pTexture, uint64_t presentationTime, uint64_t frameIndex, uint64_t frameIndex2, uint64_t clientTime, bool insertIDR)
{
	std::vector<std::vector<uint8_t>> vPacket;

	NV_ENC_PIC_PARAMS picParams = {};
	if (insertIDR) {
		Log(L"Inserting IDR frame.");
		picParams.encodePicFlags = NV_ENC_PIC_FLAG_FORCEIDR;
	}
	mEncoder->EncodeFrame(vPacket, &picParams, mD3DRender->GetContext(), pTexture);

	Log(L"Tracking info delay: %lld us FrameIndex=%llu", GetTimestampUs() - mListener->clientToServerTime(clientTime), frameIndex);
	Log(L"Encoding delay: %lld us FrameIndex=%llu", GetTimestampUs() - presentationTime, frameIndex);

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
			mListener->SendVideo(packet.data(), (int)packet.size(), frameIndex);
		}
	}

	if (Settings::Instance().mDebugFrameOutput) {
		if (!mUseNV12) {
			SaveDebugOutput(mD3DRender, vPacket, pTexture, frameIndex2);
		}
	}
}