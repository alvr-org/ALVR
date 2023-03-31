#include "EncodePipelineAMF.h"
#include "amf_helper.h"

#include "alvr_server/Logger.h"
#include "alvr_server/Settings.h"

#include <thread>

#define AMF_THROW_IF(expr) {AMF_RESULT res = expr;\
if(res != AMF_OK){throw MakeException("AMF Error %ls: %s", AMFContext::get()->resultString(res), #expr);}}

static amf::AMF_SURFACE_FORMAT fromVkFormat(VkFormat format)
{
    switch (format) {
    case VK_FORMAT_R8G8B8A8_UNORM:
        return amf::AMF_SURFACE_RGBA;
    case VK_FORMAT_B8G8R8A8_UNORM:
        return amf::AMF_SURFACE_BGRA;
    default:
        return amf::AMF_SURFACE_RGBA;
    }
}

AMFPipe::AMFPipe(amf::AMFComponentPtr src, AMFDataReceiver receiver)
    : m_amfComponentSrc(src)
    , m_receiver(receiver)
{
}

AMFPipe::~AMFPipe()
{
    Debug("AMFPipe::~AMFPipe()  m_amfComponentSrc->Drain\n");
    m_amfComponentSrc->Drain();
}

void AMFPipe::doPassthrough()
{
    amf::AMFDataPtr data;
    auto res = m_amfComponentSrc->QueryOutput(&data);
    switch (res) {
    case AMF_OK:
        if (data) {
            m_receiver(data);
        }
        break;
    case AMF_NO_DEVICE:
        Debug("m_amfComponentSrc->QueryOutput returns AMF_NO_DEVICE.\n");
        return;
    case AMF_REPEAT:
        break;
    case AMF_EOF:
        Debug("m_amfComponentSrc->QueryOutput returns AMF_EOF.\n");
        return;
    default:
        Debug("m_amfComponentSrc->QueryOutput returns unknown status.\n");
        return;
    }
}

AMFSolidPipe::AMFSolidPipe(amf::AMFComponentPtr src, amf::AMFComponentPtr dst)
    : AMFPipe(src, std::bind(&AMFSolidPipe::Passthrough, this, std::placeholders::_1))
    , m_amfComponentDst(dst)
{
}

void AMFSolidPipe::Passthrough(amf::AMFDataPtr data)
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
{
}

AMFPipeline::~AMFPipeline()
{
    for (auto &pipe : m_pipes) {
        delete pipe;
    }
}

void AMFPipeline::Connect(AMFPipe *pipe)
{
    m_pipes.emplace_back(pipe);
}

void AMFPipeline::Run()
{
    for (auto &pipe : m_pipes) {
        pipe->doPassthrough();
    }
}

namespace alvr
{

EncodePipelineAMF::EncodePipelineAMF(Renderer *render, uint32_t width, uint32_t height)
    : m_render(render)
    , m_surfaceFormat(fromVkFormat(m_render->GetOutput().imageInfo.format))
    , m_codec(Settings::Instance().m_codec)
    , m_refreshRate(Settings::Instance().m_refreshRate)
    , m_renderWidth(width)
    , m_renderHeight(height)
    , m_bitrateInMBits(30)
{
    if (!AMFContext::get()->isValid()) {
        throw MakeException("AMFContext not valid");
    }

    Debug("Initializing EncodePipelineAMF.\n");

    amf::AMFVulkanDevice *dev = new amf::AMFVulkanDevice;
    dev->cbSizeof = sizeof(amf::AMFVulkanDevice);
    dev->pNext = nullptr;
    dev->hInstance = m_render->m_inst;
    dev->hPhysicalDevice = m_render->m_physDev;
    dev->hDevice = m_render->m_dev;
    AMFContext::get()->initialize(dev);

    m_amfFactory = AMFContext::get()->factory();
    m_amfContext = AMFContext::get()->context();
    m_amfContext1 = amf::AMFContext1Ptr(m_amfContext);

    amf::AMF_SURFACE_FORMAT inFormat = m_surfaceFormat;
    if (m_codec == ALVR_CODEC_H265 && Settings::Instance().m_use10bitEncoder) {
        inFormat = amf::AMF_SURFACE_R10G10B10A2;
        m_amfComponents.emplace_back(MakeConverter(m_surfaceFormat, m_renderWidth, m_renderHeight, inFormat));
    } else {
        if (Settings::Instance().m_usePreproc) {
            inFormat = amf::AMF_SURFACE_NV12;
            m_amfComponents.emplace_back(MakeConverter(m_surfaceFormat, m_renderWidth, m_renderHeight, inFormat));
            m_amfComponents.emplace_back(MakePreprocessor(inFormat, m_renderWidth, m_renderHeight));
        }
    }
    m_amfComponents.emplace_back(MakeEncoder(inFormat, m_renderWidth, m_renderHeight, m_codec, m_refreshRate));
    auto params = FfiDynamicEncoderParams {};
    params.updated = true;
    params.bitrate_bps = 30'000'000;
    params.framerate = 60.0;
    SetParams(params);

    m_pipeline = std::make_unique<AMFPipeline>();
    for (size_t i = 0; i < m_amfComponents.size() - 1; i++) {
        m_pipeline->Connect(new AMFSolidPipe(m_amfComponents[i], m_amfComponents[i + 1]));
    }

    m_pipeline->Connect(new AMFPipe(m_amfComponents.back(), std::bind(&EncodePipelineAMF::Receive, this, std::placeholders::_1)));

    Debug("Successfully initialized EncodePipelineAMF.\n");
}

EncodePipelineAMF::~EncodePipelineAMF() = default;

amf::AMFComponentPtr EncodePipelineAMF::MakeEncoder(amf::AMF_SURFACE_FORMAT inputFormat, int width, int height, int codec, int refreshRate)
{
    const wchar_t *pCodec;

    amf_int32 frameRateIn = refreshRate;

    switch (codec) {
    case ALVR_CODEC_H264:
        pCodec = AMFVideoEncoderVCE_AVC;
        break;
    case ALVR_CODEC_H265:
        pCodec = AMFVideoEncoder_HEVC;
        break;
    default:
        throw MakeException("Unsupported video encoding %d", codec);
    }

    amf::AMFComponentPtr amfEncoder;

    // Create encoder component.
    AMF_THROW_IF(m_amfFactory->CreateComponent(m_amfContext, pCodec, &amfEncoder));

    if (codec == ALVR_CODEC_H264) {
        amfEncoder->SetProperty(AMF_VIDEO_ENCODER_USAGE, AMF_VIDEO_ENCODER_USAGE_ULTRA_LOW_LATENCY);
        amfEncoder->SetProperty(AMF_VIDEO_ENCODER_PROFILE, AMF_VIDEO_ENCODER_PROFILE_HIGH);
        amfEncoder->SetProperty(AMF_VIDEO_ENCODER_PROFILE_LEVEL, 42);
        amfEncoder->SetProperty(AMF_VIDEO_ENCODER_FRAMESIZE, ::AMFConstructSize(width, height));
        amfEncoder->SetProperty(AMF_VIDEO_ENCODER_FRAMERATE, ::AMFConstructRate(frameRateIn, 1));
        amfEncoder->SetProperty(AMF_VIDEO_ENCODER_B_PIC_PATTERN, 0);

        switch (Settings::Instance().m_rateControlMode) {
        case ALVR_VBR:
            amfEncoder->SetProperty(AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD, AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD_LATENCY_CONSTRAINED_VBR);
            break;
        case ALVR_CBR:
        default:
            amfEncoder->SetProperty(AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD, AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD_CBR);
            amfEncoder->SetProperty(AMF_VIDEO_ENCODER_FILLER_DATA_ENABLE, Settings::Instance().m_fillerData);
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

        // No noticable performance difference and should improve subjective quality by allocating more bits to smooth areas
        amfEncoder->SetProperty(AMF_VIDEO_ENCODER_ENABLE_VBAQ, Settings::Instance().m_enableVbaq);

        // Turns Off IDR/I Frames
        amfEncoder->SetProperty(AMF_VIDEO_ENCODER_IDR_PERIOD, 0);

        // Disable AUD to produce the same stream format as VideoEncoderNVENC.
        amfEncoder->SetProperty(AMF_VIDEO_ENCODER_INSERT_AUD, false);

        amf::AMFCapsPtr caps;
        if (amfEncoder->GetCaps(&caps) == AMF_OK) {
            caps->GetProperty(AMF_VIDEO_ENCODER_CAPS_QUERY_TIMEOUT_SUPPORT, &m_hasQueryTimeout);
        }
        if (m_hasQueryTimeout) {
            amfEncoder->SetProperty(AMF_VIDEO_ENCODER_QUERY_TIMEOUT, 1000); // 1s timeout
        }
    } else {
        amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_USAGE, AMF_VIDEO_ENCODER_HEVC_USAGE_ULTRA_LOW_LATENCY);
        amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_FRAMESIZE, ::AMFConstructSize(width, height));
        amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_FRAMERATE, ::AMFConstructRate(frameRateIn, 1));

        switch (Settings::Instance().m_rateControlMode) {
        case ALVR_VBR:
            amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD, AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD_LATENCY_CONSTRAINED_VBR);
            break;
        case ALVR_CBR:
        default:
            amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD, AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD_CBR);
            amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_FILLER_DATA_ENABLE, Settings::Instance().m_fillerData);
            break;
        }

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

        if (Settings::Instance().m_use10bitEncoder) {
            amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_COLOR_BIT_DEPTH, AMF_COLOR_BIT_DEPTH_10);
            amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_PROFILE, AMF_VIDEO_ENCODER_HEVC_PROFILE_MAIN_10);
        } else {
            amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_COLOR_BIT_DEPTH, AMF_COLOR_BIT_DEPTH_8);
            amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_PROFILE, AMF_VIDEO_ENCODER_HEVC_PROFILE_MAIN);
        }

        amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_NOMINAL_RANGE, AMF_VIDEO_ENCODER_HEVC_NOMINAL_RANGE_FULL);

        // No noticable performance difference and should improve subjective quality by allocating more bits to smooth areas
        amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_ENABLE_VBAQ, Settings::Instance().m_enableVbaq);

        // Turns Off IDR/I Frames
        amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_NUM_GOPS_PER_IDR, 0);
        // Set infinite GOP length
        amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_GOP_SIZE, 0);

        // Disable AUD to produce the same stream format as VideoEncoderNVENC.
        amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_INSERT_AUD, false);

        amf::AMFCapsPtr caps;
        if (amfEncoder->GetCaps(&caps) == AMF_OK) {
            caps->GetProperty(AMF_VIDEO_ENCODER_CAPS_HEVC_QUERY_TIMEOUT_SUPPORT, &m_hasQueryTimeout);
        }
        if (m_hasQueryTimeout) {
            amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_QUERY_TIMEOUT, 1000); // 1s timeout
        }
    }

    Debug("Configured %s.\n", pCodec);
    AMF_THROW_IF(amfEncoder->Init(inputFormat, width, height));

    Debug("Initialized %s.\n", pCodec);

    return amfEncoder;
}

amf::AMFComponentPtr EncodePipelineAMF::MakeConverter(amf::AMF_SURFACE_FORMAT inputFormat, int width, int height, amf::AMF_SURFACE_FORMAT outputFormat)
{
    amf::AMFComponentPtr amfConverter;
    AMF_THROW_IF(m_amfFactory->CreateComponent(m_amfContext, AMFVideoConverter, &amfConverter));

    AMF_THROW_IF(amfConverter->SetProperty(AMF_VIDEO_CONVERTER_MEMORY_TYPE, amf::AMF_MEMORY_VULKAN));
    AMF_THROW_IF(amfConverter->SetProperty(AMF_VIDEO_CONVERTER_OUTPUT_FORMAT, outputFormat));
    AMF_THROW_IF(amfConverter->SetProperty(AMF_VIDEO_CONVERTER_OUTPUT_SIZE, ::AMFConstructSize(width, height)));

    AMF_THROW_IF(amfConverter->Init(inputFormat, width, height));

    Debug("Initialized %s.\n", AMFVideoConverter);
    return amfConverter;
}

amf::AMFComponentPtr EncodePipelineAMF::MakePreprocessor(amf::AMF_SURFACE_FORMAT inputFormat, int width, int height)
{
    amf::AMFComponentPtr amfPreprocessor;
    AMF_THROW_IF(m_amfFactory->CreateComponent(m_amfContext, AMFPreProcessing, &amfPreprocessor));

    AMF_THROW_IF(amfPreprocessor->SetProperty(AMF_PP_ENGINE_TYPE, amf::AMF_MEMORY_VULKAN));
    AMF_THROW_IF(amfPreprocessor->SetProperty(AMF_PP_ADAPTIVE_FILTER_STRENGTH, Settings::Instance().m_preProcSigma));
    AMF_THROW_IF(amfPreprocessor->SetProperty(AMF_PP_ADAPTIVE_FILTER_SENSITIVITY, Settings::Instance().m_preProcTor));

    AMF_THROW_IF(amfPreprocessor->Init(inputFormat, width, height));

    Debug("Initialized %s.\n", AMFPreProcessing);
    return amfPreprocessor;
}

void EncodePipelineAMF::PushFrame(uint64_t targetTimestampNs, bool idr)
{
    m_targetTimestampNs = targetTimestampNs;

    amf::AMFVulkanSurface surfaceVk;
    surfaceVk.cbSizeof = sizeof(amf::AMFVulkanSurface);
    surfaceVk.pNext = nullptr;
    surfaceVk.hImage = m_render->GetOutput().image;
    surfaceVk.hMemory = m_render->GetOutput().memory;
    surfaceVk.iSize = m_render->GetOutput().size;
    surfaceVk.eFormat = m_render->GetOutput().imageInfo.format;
    surfaceVk.iWidth = m_render->GetOutput().imageInfo.extent.width;
    surfaceVk.iHeight = m_render->GetOutput().imageInfo.extent.height;
    surfaceVk.eCurrentLayout = m_render->GetOutput().layout;
    surfaceVk.eUsage = amf::AMF_SURFACE_USAGE_TRANSFER_SRC | amf::AMF_SURFACE_USAGE_UNORDERED_ACCESS;
    surfaceVk.eAccess = amf::AMF_MEMORY_CPU_LOCAL;
    surfaceVk.Sync.cbSizeof = sizeof(amf::AMFVulkanSync);
    surfaceVk.Sync.pNext = nullptr;
    surfaceVk.Sync.hSemaphore = m_render->GetOutput().semaphore;
    surfaceVk.Sync.bSubmitted = true;
    surfaceVk.Sync.hFence = nullptr;

    amf::AMFSurfacePtr surface;
    AMF_THROW_IF(m_amfContext1->CreateSurfaceFromVulkanNative(&surfaceVk, &surface, nullptr));

    ApplyFrameProperties(surface, idr);

    m_amfComponents.front()->SubmitInput(surface);

    m_render->GetOutput().layout = static_cast<VkImageLayout>(surfaceVk.eCurrentLayout);
}

bool EncodePipelineAMF::GetEncoded(FramePacket &packet)
{
    m_frameBuffer = NULL;
    if (m_hasQueryTimeout) {
        m_pipeline->Run();
    } else {
        uint32_t timeout = 4 * 1000; // 1 second
        while (m_frameBuffer == NULL && --timeout != 0) {
            std::this_thread::sleep_for(std::chrono::microseconds(250));
            m_pipeline->Run();
        }
    }

    if (m_frameBuffer == NULL) {
        Error("Timed out waiting for encoder data");
        return false;
    }

    packet.data = reinterpret_cast<uint8_t *>(m_frameBuffer->GetNative());
    packet.size = static_cast<int>(m_frameBuffer->GetSize());
    packet.pts = m_targetTimestampNs;

    return true;
}

void EncodePipelineAMF::SetParams(FfiDynamicEncoderParams params)
{
    if (!params.updated) {
        return;
    }
    amf_int64 bitRateIn = params.bitrate_bps;
    if (m_codec == ALVR_CODEC_H264) {
        m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_TARGET_BITRATE, bitRateIn);
        m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_PEAK_BITRATE, bitRateIn);
        m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_FRAMERATE, ::AMFConstructRate(params.framerate * 1000, 1000));
        m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_VBV_BUFFER_SIZE, bitRateIn / params.framerate * 1.1);
    } else {
        m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_HEVC_TARGET_BITRATE, bitRateIn);
        m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_HEVC_PEAK_BITRATE, bitRateIn);
        m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_HEVC_FRAMERATE, ::AMFConstructRate(params.framerate * 1000, 1000));
        m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_HEVC_VBV_BUFFER_SIZE, bitRateIn / params.framerate * 1.1);
    }
}

void EncodePipelineAMF::Receive(amf::AMFDataPtr data)
{
    m_frameBuffer = amf::AMFBufferPtr(data); // query for buffer interface
}

void EncodePipelineAMF::ApplyFrameProperties(const amf::AMFSurfacePtr &surface, bool insertIDR)
{
    switch (m_codec) {
    case ALVR_CODEC_H264:
        surface->SetProperty(AMF_VIDEO_ENCODER_INSERT_AUD, false);
        if (insertIDR) {
            Debug("Inserting IDR frame for H.264.\n");
            surface->SetProperty(AMF_VIDEO_ENCODER_INSERT_SPS, true);
            surface->SetProperty(AMF_VIDEO_ENCODER_INSERT_PPS, true);
            surface->SetProperty(AMF_VIDEO_ENCODER_FORCE_PICTURE_TYPE, AMF_VIDEO_ENCODER_PICTURE_TYPE_IDR);
        }
        break;
    case ALVR_CODEC_H265:
        surface->SetProperty(AMF_VIDEO_ENCODER_HEVC_INSERT_AUD, false);
        if (insertIDR) {
            Debug("Inserting IDR frame for H.265.\n");
            // Insert VPS,SPS,PPS
            surface->SetProperty(AMF_VIDEO_ENCODER_HEVC_INSERT_HEADER, true);
            surface->SetProperty(AMF_VIDEO_ENCODER_HEVC_FORCE_PICTURE_TYPE, AMF_VIDEO_ENCODER_HEVC_PICTURE_TYPE_IDR);
        }
        break;
    default:
        throw MakeException("Invalid video codec");
    }
}

};
