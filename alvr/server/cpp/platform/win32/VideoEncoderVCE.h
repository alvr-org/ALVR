#pragma once
#include "VideoEncoder.h"

#include "amf/public/common/AMFFactory.h"
#include "amf/public/include/components/VideoEncoderVCE.h"
#include "amf/public/include/components/VideoEncoderHEVC.h"
#include "amf/public/include/components/VideoConverter.h"
#include "amf/public/include/components/PreProcessing.h"
#include "amf/public/common/AMFSTL.h"
#include "amf/public/common/Thread.h"

typedef amf::AMFData * AMFDataPtr;
typedef std::function<void (AMFDataPtr)> AMFDataReceiver;

class AMFPipeline;

class AMFPipe {
public:
	AMFPipe(amf::AMFComponentPtr src, AMFDataReceiver receiver);
	virtual ~AMFPipe();

	void doPassthrough();
protected:
	amf::AMFComponentPtr m_amfComponentSrc;
	AMFDataReceiver m_receiver;
};

typedef AMFPipe* AMFPipePtr;

class AMFSolidPipe : public AMFPipe {
public:
	AMFSolidPipe(amf::AMFComponentPtr src, amf::AMFComponentPtr dst);
protected:
	void Passthrough(AMFDataPtr);

	amf::AMFComponentPtr m_amfComponentDst;
};

class AMFPipeline {
public:
	AMFPipeline();
	~AMFPipeline();

	void Connect(AMFPipePtr pipe);
	void Start();
	void Run();
	void RunReceive();
protected:
	std::thread *m_thread;
	std::thread *m_receiveThread;
	std::vector<AMFPipePtr> m_pipes;
	bool isRunning;
};

typedef AMFPipeline* AMFPipelinePtr;

enum EncoderQualityPreset {
	QUALITY = 0,
	BALANCED = 1,
	SPEED = 2
};

// Video encoder for AMD VCE.
class VideoEncoderVCE : public VideoEncoder
{
public:
	VideoEncoderVCE(std::shared_ptr<CD3DRender> pD3DRender
		, std::shared_ptr<ClientConnection> listener
		, int width, int height);
	~VideoEncoderVCE();

	void Initialize();
	void Shutdown();

	void Transmit(ID3D11Texture2D *pTexture, uint64_t presentationTime, uint64_t targetTimestampNs, bool insertIDR);
	void Receive(AMFDataPtr data);
private:	
	static const wchar_t *START_TIME_PROPERTY;
	static const wchar_t *FRAME_INDEX_PROPERTY;

	const uint64_t MILLISEC_TIME = 10000;
	const uint64_t MICROSEC_TIME = 10;

	amf::AMFComponentPtr MakeConverter(
		amf::AMF_SURFACE_FORMAT inputFormat, int width, int height, amf::AMF_SURFACE_FORMAT outputFormat
	);
	amf::AMFComponentPtr MakePreprocessor(
		amf::AMF_SURFACE_FORMAT inputFormat, int width, int height
	);
	amf::AMFComponentPtr MakeEncoder(
		amf::AMF_SURFACE_FORMAT inputFormat, int width, int height, int codec, int refreshRate, int bitrateInMbits
	);
	amf::AMFContextPtr m_amfContext;
	AMFPipelinePtr m_pipeline;
	std::vector<amf::AMFComponentPtr> m_amfComponents;

	std::ofstream fpOut;

	std::shared_ptr<CD3DRender> m_d3dRender;
	std::shared_ptr<ClientConnection> m_Listener;

	bool m_use10bit;
	bool m_usePreProc;
	uint32_t m_preProcSigma;
	uint32_t m_preProcTor;
	EncoderQualityPreset m_encoderQualityPreset;
	amf::AMF_SURFACE_FORMAT m_surfaceFormat;

	int m_codec;
	int m_refreshRate;
	int m_renderWidth;
	int m_renderHeight;
	int m_bitrateInMBits;

	void ApplyFrameProperties(const amf::AMFSurfacePtr &surface, bool insertIDR);
	void SkipAUD(char **buffer, int *length);
};

