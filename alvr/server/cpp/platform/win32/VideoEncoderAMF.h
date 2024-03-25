#pragma once
#include "VideoEncoder.h"

#include "../../shared/amf/public/common/AMFFactory.h"
#include "../../shared/amf/public/include/components/VideoEncoderVCE.h"
#include "../../shared/amf/public/include/components/VideoEncoderHEVC.h"
#include "../../shared/amf/public/include/components/VideoEncoderAV1.h"
#include "../../shared/amf/public/include/components/VideoConverter.h"
#include "../../shared/amf/public/include/components/PreProcessing.h"
#include "../../shared/amf/public/common/AMFSTL.h"
#include "../../shared/amf/public/common/Thread.h"

typedef amf::AMFData * AMFDataPtr;
typedef std::function<void (AMFDataPtr)> AMFDataReceiver;

class AMFPipeline;

class AMFPipe {
public:
	AMFPipe(amf::AMFComponentPtr src, AMFDataReceiver receiver);
	virtual ~AMFPipe();

	void doPassthrough(bool hasQueryTimeout, uint32_t timerResolution);
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
	void Run(bool hasQueryTimeout);
protected:
	uint32_t m_timerResolution;

	std::vector<AMFPipePtr> m_pipes;
};

typedef AMFPipeline* AMFPipelinePtr;

// Video encoder for AMD VCE and VCN.
class VideoEncoderAMF : public VideoEncoder
{
public:
	VideoEncoderAMF(std::shared_ptr<CD3DRender> pD3DRender
		, int width, int height);
	~VideoEncoderAMF();

	void Initialize();
	void Shutdown();

	void Transmit(ID3D11Texture2D *pTexture, uint64_t presentationTime, uint64_t targetTimestampNs, bool insertIDR);
	void Receive(AMFDataPtr data);
private:	
	static const wchar_t *START_TIME_PROPERTY;
	static const wchar_t *FRAME_INDEX_PROPERTY;

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

	bool m_use10bit;
	amf::AMF_SURFACE_FORMAT m_surfaceFormat;

	int m_codec;
	int m_refreshRate;
	int m_renderWidth;
	int m_renderHeight;
	int m_bitrateInMBits;

	bool m_hasQueryTimeout;
	bool m_hasPreAnalysis;

	void ApplyFrameProperties(const amf::AMFSurfacePtr &surface, bool insertIDR);
};

