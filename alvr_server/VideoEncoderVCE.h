#pragma once
#include "VideoEncoder.h"

#include "amf/common/AMFFactory.h"
#include "amf/include/components/VideoEncoderVCE.h"
#include "amf/include/components/VideoEncoderHEVC.h"
#include "amf/include/components/VideoConverter.h"
#include "amf/common/AMFSTL.h"
#include "amf/common/Thread.h"

// Video encoder for AMD VCE.
class VideoEncoderVCE : public VideoEncoder
{
public:
	VideoEncoderVCE(std::shared_ptr<CD3DRender> pD3DRender
		, std::shared_ptr<Listener> listener, int width, int height, bool useNV12);
	~VideoEncoderVCE();

	bool Initialize();
	void Shutdown();

	void Transmit(ID3D11Texture2D *pTexture, uint64_t presentationTime, uint64_t frameIndex, uint64_t frameIndex2, uint64_t clientTime, bool insertIDR);

private:
	amf::AMF_SURFACE_FORMAT ENCODER_INPUT_FORMAT = amf::AMF_SURFACE_RGBA;
	
	const wchar_t *START_TIME_PROPERTY = L"StartTimeProperty";

	const double MILLISEC_TIME = 10000;

	amf::AMFContextPtr m_amfContext;
	amf::AMFComponentPtr m_amfEncoder;
	amf::AMFSurfacePtr m_amfSurfaceIn;

	std::ofstream fpOut;

	std::shared_ptr<CD3DRender> m_d3dRender;
	int m_nFrame;

	std::shared_ptr<Listener> m_Listener;

	int m_width;
	int m_height;
	const bool m_useNV12;

	void ApplyFrameProperties(const amf::AMFSurfacePtr &surface, bool insertIDR);
	void SkipAUD(char **buffer, int *length);
};

