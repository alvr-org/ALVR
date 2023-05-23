#pragma once
#include "EncodePipeline.h"
#include "ffmpeg_helper.h"

#include <functional>
#include <vulkan/vulkan.h>

#include "../../shared/amf/public/common/AMFFactory.h"
#include "../../shared/amf/public/include/components/VideoEncoderVCE.h"
#include "../../shared/amf/public/include/components/VideoEncoderHEVC.h"
#include "../../shared/amf/public/include/components/VideoConverter.h"
#include "../../shared/amf/public/include/components/PreProcessing.h"
#include "../../shared/amf/public/include/core/VulkanAMF.h"
#include "../../shared/amf/public/common/AMFSTL.h"
#include "../../shared/amf/public/common/Thread.h"

typedef std::function<void(amf::AMFDataPtr)> AMFDataReceiver;

class AMFPipeline;

class AMFPipe
{
public:
    AMFPipe(amf::AMFComponentPtr src, AMFDataReceiver receiver);
    virtual ~AMFPipe();

    void doPassthrough();
protected:
    amf::AMFComponentPtr m_amfComponentSrc;
    AMFDataReceiver m_receiver;
};

class AMFSolidPipe : public AMFPipe
{
public:
    AMFSolidPipe(amf::AMFComponentPtr src, amf::AMFComponentPtr dst);
protected:
    void Passthrough(amf::AMFDataPtr data);

    amf::AMFComponentPtr m_amfComponentDst;
};

class AMFPipeline
{
public:
    AMFPipeline();
    ~AMFPipeline();

    void Connect(AMFPipe *pipe);
    void Run();
protected:
    std::vector<AMFPipe*> m_pipes;
};

enum EncoderQualityPreset {
    QUALITY = 0,
    BALANCED = 1,
    SPEED = 2
};

namespace alvr
{

class EncodePipelineAMF : public EncodePipeline
{
public:
    EncodePipelineAMF(Renderer *render, uint32_t width, uint32_t height);
    ~EncodePipelineAMF();

    void PushFrame(uint64_t targetTimestampNs, bool idr) override;
    bool GetEncoded(FramePacket &packet) override;
    void SetParams(FfiDynamicEncoderParams params) override;

private:
    amf::AMFComponentPtr MakeConverter(amf::AMF_SURFACE_FORMAT inputFormat, int width, int height, amf::AMF_SURFACE_FORMAT outputFormat);
    amf::AMFComponentPtr MakePreprocessor(amf::AMF_SURFACE_FORMAT inputFormat, int width, int height);
    amf::AMFComponentPtr MakeEncoder(amf::AMF_SURFACE_FORMAT inputFormat, int width, int height, int codec, int refreshRate);
    void Receive(amf::AMFDataPtr data);
    void ApplyFrameProperties(const amf::AMFSurfacePtr &surface, bool insertIDR);

    amf::AMFFactory *m_amfFactory = nullptr;
    amf::AMFContextPtr m_amfContext;
    amf::AMFContext1Ptr m_amfContext1;
    std::unique_ptr<AMFPipeline> m_pipeline;
    std::vector<amf::AMFComponentPtr> m_amfComponents;

    Renderer *m_render;
    amf::AMF_SURFACE_FORMAT m_surfaceFormat;

    int m_codec;
    int m_refreshRate;
    int m_renderWidth;
    int m_renderHeight;

    bool m_hasQueryTimeout = false;
    
    amf::AMFBufferPtr m_frameBuffer;
    uint64_t m_targetTimestampNs;
};

};
