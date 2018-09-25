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
		, int width, int height
		, amf::AMF_SURFACE_FORMAT inputFormat
		, AMFTextureReceiver receiver);
	~AMFTextureEncoder();

	void Start();
	void Shutdown();
	void Submit(amf::AMFData *data);
private:
	amf::AMFComponentPtr m_amfEncoder;
	std::thread *m_thread = NULL;
	AMFTextureReceiver m_receiver;

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
	amf::AMFComponentPtr m_amfConverter;
	std::thread *m_thread = NULL;
	AMFTextureReceiver m_receiver;

	void Run();
};

// Video encoder for AMD VCE.
class VideoEncoderVCE : public VideoEncoder
{
public:
	VideoEncoderVCE(std::shared_ptr<CD3DRender> pD3DRender
		, std::shared_ptr<Listener> listener, int width, int height);
	~VideoEncoderVCE();

	void Initialize();
	void Shutdown();

	void Transmit(ID3D11Texture2D *pTexture, uint64_t presentationTime, uint64_t frameIndex, uint64_t frameIndex2, uint64_t clientTime, bool insertIDR);
	void Receive(amf::AMFData *data);
private:
	static const amf::AMF_SURFACE_FORMAT CONVERTER_INPUT_FORMAT = amf::AMF_SURFACE_RGBA;
	static const amf::AMF_SURFACE_FORMAT ENCODER_INPUT_FORMAT = amf::AMF_SURFACE_RGBA;// amf::AMF_SURFACE_NV12;
	
	static const wchar_t *START_TIME_PROPERTY;
	static const wchar_t *FRAME_INDEX_PROPERTY;

	const double MILLISEC_TIME = 10000;

	amf::AMFContextPtr m_amfContext;
	std::shared_ptr<AMFTextureEncoder> m_encoder;
	std::shared_ptr<AMFTextureConverter> m_converter;

	std::ofstream fpOut;

	std::shared_ptr<CD3DRender> m_d3dRender;
	std::shared_ptr<Listener> m_Listener;

	int m_width;
	int m_height;

	void ApplyFrameProperties(const amf::AMFSurfacePtr &surface, bool insertIDR);
	void SkipAUD(char **buffer, int *length);
};

