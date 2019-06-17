#pragma once
#include "VideoEncoder.h"

#include "amf/common/AMFFactory.h"
#include "amf/include/components/VideoEncoderVCE.h"
#include "amf/include/components/VideoEncoderHEVC.h"
#include "amf/include/components/VideoConverter.h"
#include "amf/common/AMFSTL.h"
#include "amf/common/Thread.h"

typedef std::function<void (amf::AMFData *)> AMFTextureReceiver;

class AMFTextureEncoder {
public:
	AMFTextureEncoder(const amf::AMFContextPtr &amfContext
		, int codec, int width, int height, int refreshRate, int bitrateInMbits
		, amf::AMF_SURFACE_FORMAT inputFormat
		, AMFTextureReceiver receiver);
	~AMFTextureEncoder();

	void Start();
	void Shutdown();
	void Submit(amf::AMFData *data);
private:
	amf::AMFComponentPtr mEncoder;
	std::thread *mThread = NULL;
	AMFTextureReceiver mReceiver;

	void Run();
};

class AMFTextureConverter {
public:
	AMFTextureConverter(const amf::AMFContextPtr &amfContext
		, int width, int height
		, amf::AMF_SURFACE_FORMAT inputFormat, amf::AMF_SURFACE_FORMAT outputFormat
		, AMFTextureReceiver receiver);
	~AMFTextureConverter();

	void Start();
	void Shutdown();
	void Submit(amf::AMFData *data);
private:
	amf::AMFComponentPtr mConverter;
	std::thread *mThread = NULL;
	AMFTextureReceiver mReceiver;

	void Run();
};

// Video encoder for AMD VCE.
class VideoEncoderVCE : public VideoEncoder
{
public:
	VideoEncoderVCE(std::shared_ptr<CD3DRender> pD3DRender
		, std::shared_ptr<Listener> listener);
	~VideoEncoderVCE();

	void Initialize();
	void Reconfigure(int refreshRate, int renderWidth, int renderHeight, Bitrate bitrate);
	void Shutdown();

	void Transmit(ID3D11Texture2D *pTexture, uint64_t presentationTime, uint64_t videoFrameIndex, uint64_t trackingFrameIndex, uint64_t clientTime, bool insertIDR);
	void Receive(amf::AMFData *data);

	// TODO: Implement reference frame invalidation.
	bool SupportsReferenceFrameInvalidation() { return false; };
	virtual void InvalidateReferenceFrame(uint64_t videoFrameIndex) {};
private:
	static const amf::AMF_SURFACE_FORMAT CONVERTER_INPUT_FORMAT = amf::AMF_SURFACE_RGBA;
	static const amf::AMF_SURFACE_FORMAT ENCODER_INPUT_FORMAT = amf::AMF_SURFACE_RGBA;// amf::AMF_SURFACE_NV12;
	
	static const wchar_t *START_TIME_PROPERTY;
	static const wchar_t *VIDEO_FRAME_INDEX_PROPERTY;
	static const wchar_t *TRACKING_FRAME_INDEX_PROPERTY;

	const uint64_t MILLISEC_TIME = 10000;
	const uint64_t MICROSEC_TIME = 10;

	amf::AMFContextPtr mContext;
	std::shared_ptr<AMFTextureEncoder> mEncoder;
	std::shared_ptr<AMFTextureConverter> mConverter;

	std::ofstream mOutput;

	std::shared_ptr<CD3DRender> mD3DRender;
	std::shared_ptr<Listener> mListener;

	int mCodec;
	int mRefreshRate;
	int mRenderWidth;
	int mRenderHeight;
	Bitrate mBitrate;

	void ApplyFrameProperties(const amf::AMFSurfacePtr &surface, bool insertIDR);
	void SkipAUD(char **buffer, int *length);
};

