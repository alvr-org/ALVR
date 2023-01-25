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
    , m_bitrateInMBits(Settings::Instance().mEncodeBitrateMBs)
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
    SetBitrate(m_bitrateInMBits * 1'000'000L); // in bits

    m_pipeline = std::make_unique<AMFPipeline>();
    for (size_t i = 0; i < m_amfComponents.size() - 1; i++) {
        m_pipeline->Connect(new AMFSolidPipe(m_amfComponents[i], m_amfComponents[i + 1]));
    }

    m_pipeline->Connect(new AMFPipe(m_amfComponents.back(), std::bind(&EncodePipelineAMF::Receive, this, std::placeholders::_1)));

    VkQueryPoolCreateInfo queryPoolInfo = {};
    queryPoolInfo.sType = VK_STRUCTURE_TYPE_QUERY_POOL_CREATE_INFO;
    queryPoolInfo.queryType = VK_QUERY_TYPE_TIMESTAMP;
    queryPoolInfo.queryCount = 1;
    VK_CHECK(vkCreateQueryPool(m_render->m_dev, &queryPoolInfo, nullptr, &m_queryPool));

    VkCommandBufferAllocateInfo commandBufferInfo = {};
    commandBufferInfo.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO;
    commandBufferInfo.level = VK_COMMAND_BUFFER_LEVEL_PRIMARY;
    commandBufferInfo.commandPool = m_render->m_commandPool;
    commandBufferInfo.commandBufferCount = 1;
    VK_CHECK(vkAllocateCommandBuffers(m_render->m_dev, &commandBufferInfo, &m_commandBuffer));

    Debug("Successfully initialized EncodePipelineAMF.\n");
}

EncodePipelineAMF::~EncodePipelineAMF()
{
    vkDestroyQueryPool(m_render->m_dev, m_queryPool, nullptr);
}

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
            amfEncoder->SetProperty(AMF_VIDEO_ENCODER_FILLER_DATA_ENABLE, true);
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

        switch (Settings::Instance().m_encoderQualityPreset) {
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

        amfEncoder->SetProperty(AMF_VIDEO_ENCODER_MAX_NUM_REFRAMES, 0);
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
            amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_FILLER_DATA_ENABLE, true);
            break;
        }

        switch (Settings::Instance().m_encoderQualityPreset) {
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

        amfEncoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_MAX_NUM_REFRAMES, 0);
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
    amf::AMFSurfacePtr surface;
    // Surface is cached by AMF.

    AMF_THROW_IF(m_amfContext->AllocSurface(amf::AMF_MEMORY_VULKAN, m_surfaceFormat, m_renderWidth, m_renderHeight, &surface));
    amf::AMFVulkanView *viewVk = (amf::AMFVulkanView*)surface->GetPlaneAt(0)->GetNative(); // no reference counting - do not Release()
    amf::AMFVulkanSurface *surfaceVk = viewVk->pSurface;

    VkImageMemoryBarrier imageBarriers[2];
    imageBarriers[0] = {};
    imageBarriers[0].sType = VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER;
    imageBarriers[0].oldLayout = VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL;
    imageBarriers[0].newLayout = VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL;
    imageBarriers[0].image = m_render->GetOutput().image;
    imageBarriers[0].subresourceRange.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
    imageBarriers[0].subresourceRange.layerCount = 1;
    imageBarriers[0].subresourceRange.levelCount = 1;
    imageBarriers[0].srcAccessMask = 0;
    imageBarriers[0].dstAccessMask = VK_ACCESS_TRANSFER_READ_BIT;
    imageBarriers[0].srcQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;
    imageBarriers[0].dstQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;
    imageBarriers[1] = {};
    imageBarriers[1].sType = VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER;
    imageBarriers[1].oldLayout = static_cast<VkImageLayout>(surfaceVk->eCurrentLayout);
    imageBarriers[1].newLayout = VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL;
    imageBarriers[1].image = surfaceVk->hImage;
    imageBarriers[1].subresourceRange.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
    imageBarriers[1].subresourceRange.layerCount = 1;
    imageBarriers[1].subresourceRange.levelCount = 1;
    imageBarriers[1].srcAccessMask = 0;
    imageBarriers[1].dstAccessMask = VK_ACCESS_TRANSFER_WRITE_BIT;
    imageBarriers[1].srcQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;
    imageBarriers[1].dstQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;

    VkImageCopy imageCopy;
    imageCopy.srcSubresource.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
    imageCopy.srcSubresource.mipLevel = 0;
    imageCopy.srcSubresource.baseArrayLayer = 0;
    imageCopy.srcSubresource.layerCount = 1;
    imageCopy.srcOffset.x = 0;
    imageCopy.srcOffset.y = 0;
    imageCopy.srcOffset.z = 0;
    imageCopy.dstSubresource.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
    imageCopy.dstSubresource.mipLevel = 0;
    imageCopy.dstSubresource.baseArrayLayer = 0;
    imageCopy.dstSubresource.layerCount = 1;
    imageCopy.dstOffset.x = 0;
    imageCopy.dstOffset.y = 0;
    imageCopy.dstOffset.z = 0;
    imageCopy.extent.width = m_renderWidth;
    imageCopy.extent.height = m_renderHeight;
    imageCopy.extent.depth = 1;

    VkCommandBufferBeginInfo commandBufferBegin = {};
    commandBufferBegin.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO;
    VK_CHECK(vkBeginCommandBuffer(m_commandBuffer, &commandBufferBegin));

    vkCmdResetQueryPool(m_commandBuffer, m_queryPool, 0, 1);

    vkCmdPipelineBarrier(m_commandBuffer, VK_PIPELINE_STAGE_TRANSFER_BIT, VK_PIPELINE_STAGE_TRANSFER_BIT, 0, 0, nullptr, 0, nullptr, 2, imageBarriers);
    vkCmdCopyImage(m_commandBuffer, m_render->GetOutput().image, VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL, surfaceVk->hImage, VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL, 1, &imageCopy);

    vkCmdWriteTimestamp(m_commandBuffer, VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT, m_queryPool, 0);

    vkEndCommandBuffer(m_commandBuffer);

    uint32_t waitCount = 0;
    VkSemaphore waitSemaphores[2];
    VkPipelineStageFlags waitStages[2];

    waitSemaphores[waitCount] = m_render->GetOutput().semaphore;
    waitStages[waitCount] = VK_PIPELINE_STAGE_TRANSFER_BIT;
    waitCount++;

    if (surfaceVk->Sync.bSubmitted) {
        waitSemaphores[waitCount] = surfaceVk->Sync.hSemaphore;
        waitStages[waitCount] = VK_PIPELINE_STAGE_TRANSFER_BIT;
        waitCount++;
    }

    VkSubmitInfo submitInfo = {};
    submitInfo.sType = VK_STRUCTURE_TYPE_SUBMIT_INFO;
    submitInfo.waitSemaphoreCount = waitCount;
    submitInfo.pWaitSemaphores = waitSemaphores;
    submitInfo.pWaitDstStageMask = waitStages;
    submitInfo.signalSemaphoreCount = 1;
    submitInfo.pSignalSemaphores = &surfaceVk->Sync.hSemaphore;
    submitInfo.commandBufferCount = 1;
    submitInfo.pCommandBuffers = &m_commandBuffer;
    VK_CHECK(vkQueueSubmit(m_render->m_queue, 1, &submitInfo, nullptr));

    surfaceVk->Sync.bSubmitted = true;

    m_targetTimestampNs = targetTimestampNs;

    ApplyFrameProperties(surface, idr);

    m_amfComponents.front()->SubmitInput(surface);
}

bool EncodePipelineAMF::GetEncoded(std::vector<uint8_t> &out, uint64_t *pts)
{
    if (m_hasQueryTimeout) {
        m_pipeline->Run();
    } else {
        uint32_t timeout = 4 * 1000; // 1 second
        while (m_outBuffer.empty() && --timeout != 0) {
            std::this_thread::sleep_for(std::chrono::microseconds(250));
            m_pipeline->Run();
        }
    }

    if (m_outBuffer.empty()) {
        Error("Timed out waiting for encoder data");
        return false;
    }

    out = m_outBuffer;
    *pts = m_targetTimestampNs;
    m_outBuffer.clear();

    uint64_t query;
    VK_CHECK(vkGetQueryPoolResults(m_render->m_dev, m_queryPool, 0, 1, sizeof(uint64_t), &query, sizeof(uint64_t), VK_QUERY_RESULT_64_BIT));
    timestamp.gpu = query * m_render->m_timestampPeriod;

    return true;
}

void EncodePipelineAMF::SetBitrate(int64_t bitrate)
{
    if (m_codec == ALVR_CODEC_H264) {
        m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_TARGET_BITRATE, bitrate);
        m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_PEAK_BITRATE, bitrate);
        m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_VBV_BUFFER_SIZE, bitrate / m_refreshRate * 1.1);
    } else {
        m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_HEVC_TARGET_BITRATE, bitrate);
        m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_HEVC_PEAK_BITRATE, bitrate);
        m_amfComponents.back()->SetProperty(AMF_VIDEO_ENCODER_HEVC_VBV_BUFFER_SIZE, bitrate / m_refreshRate * 1.1);
    }
}

void EncodePipelineAMF::Receive(amf::AMFDataPtr data)
{
    amf::AMFBufferPtr buffer(data); // query for buffer interface

    char *p = reinterpret_cast<char*>(buffer->GetNative());
    int length = static_cast<int>(buffer->GetSize());

    m_outBuffer = std::vector<uint8_t>(p, p + length);
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
