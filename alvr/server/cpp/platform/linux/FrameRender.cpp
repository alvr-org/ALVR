#include "FrameRender.h"
#include "alvr_server/Settings.h"
#include "alvr_server/Logger.h"
#include "alvr_server/bindings.h"

#include <fstream>
#include <filesystem>

FrameRender::FrameRender(alvr::VkContext &ctx, init_packet &init, int fds[])
    : Renderer(ctx.get_vk_instance(), ctx.get_vk_device(), ctx.get_vk_phys_device(), ctx.get_vk_queue_family_index(), ctx.get_vk_device_extensions())
{
    Startup(init.image_create_info.extent.width, init.image_create_info.extent.height, init.image_create_info.format);

    for (size_t i = 0; i < 3; ++i) {
        AddImage(init.image_create_info, init.mem_index, fds[2 * i], fds[2 * i + 1]);
    }

    m_width = Settings::Instance().m_renderWidth;
    m_height = Settings::Instance().m_renderHeight;

    Info("FrameRender: Input size %ux%u", m_width, m_height);

    setupCustomShaders("pre");

    if (Settings::Instance().m_enableColorCorrection) {
        setupColorCorrection();
    }

    if (Settings::Instance().m_enableFoveatedRendering) {
        setupFoveatedRendering();
    }

    setupCustomShaders("post");

    if (m_pipelines.empty()) {
        RenderPipeline *pipeline = new RenderPipeline(this);
        pipeline->SetShader(RenderPipeline::VertexShader, QUAD_SHADER_VERT_SPV_PTR, QUAD_SHADER_VERT_SPV_LEN);
        pipeline->SetShader(RenderPipeline::FragmentShader, QUAD_SHADER_FRAG_SPV_PTR, QUAD_SHADER_FRAG_SPV_LEN);
        m_pipelines.push_back(pipeline);
        AddPipeline(pipeline);
    }

    Info("FrameRender: Output size %ux%u", m_width, m_height);
}

FrameRender::~FrameRender()
{
    for (RenderPipeline *pipeline : m_pipelines) {
        delete pipeline;
    }
}

FrameRender::Output FrameRender::CreateOutput()
{
    Renderer::CreateOutput(m_width, m_height);
    return GetOutput();
}

uint32_t FrameRender::GetEncodingWidth() const
{
    return m_width;
}

uint32_t FrameRender::GetEncodingHeight() const
{
    return m_height;
}

void FrameRender::setupColorCorrection()
{
    m_colorCorrectionPushConstants.renderWidth = m_width;
    m_colorCorrectionPushConstants.renderHeight = m_height;
    m_colorCorrectionPushConstants.brightness = Settings::Instance().m_brightness;
    m_colorCorrectionPushConstants.contrast = Settings::Instance().m_contrast + 1.f;
    m_colorCorrectionPushConstants.saturation = Settings::Instance().m_saturation + 1.f;
    m_colorCorrectionPushConstants.gamma = Settings::Instance().m_gamma;
    m_colorCorrectionPushConstants.sharpening = Settings::Instance().m_sharpening;

    RenderPipeline *pipeline = new RenderPipeline(this);
    pipeline->SetShader(RenderPipeline::VertexShader, QUAD_SHADER_VERT_SPV_PTR, QUAD_SHADER_VERT_SPV_LEN);
    pipeline->SetShader(RenderPipeline::FragmentShader, COLOR_SHADER_FRAG_SPV_PTR, COLOR_SHADER_FRAG_SPV_LEN);
    pipeline->SetPushConstant(RenderPipeline::FragmentShader, &m_colorCorrectionPushConstants, sizeof(m_colorCorrectionPushConstants));
    m_pipelines.push_back(pipeline);
    AddPipeline(pipeline);
}

void FrameRender::setupFoveatedRendering()
{
    float targetEyeWidth = (float)m_width / 2;
    float targetEyeHeight = (float)m_height;

    float centerSizeX = (float)Settings::Instance().m_foveationCenterSizeX;
    float centerSizeY = (float)Settings::Instance().m_foveationCenterSizeY;
    float centerShiftX = (float)Settings::Instance().m_foveationCenterShiftX;
    float centerShiftY = (float)Settings::Instance().m_foveationCenterShiftY;
    float edgeRatioX = (float)Settings::Instance().m_foveationEdgeRatioX;
    float edgeRatioY = (float)Settings::Instance().m_foveationEdgeRatioY;

    float edgeSizeX = targetEyeWidth-centerSizeX*targetEyeWidth;
    float edgeSizeY = targetEyeHeight-centerSizeY*targetEyeHeight;

    float centerSizeXAligned = 1.-ceil(edgeSizeX/(edgeRatioX*2.))*(edgeRatioX*2.)/targetEyeWidth;
    float centerSizeYAligned = 1.-ceil(edgeSizeY/(edgeRatioY*2.))*(edgeRatioY*2.)/targetEyeHeight;

    float edgeSizeXAligned = targetEyeWidth-centerSizeXAligned*targetEyeWidth;
    float edgeSizeYAligned = targetEyeHeight-centerSizeYAligned*targetEyeHeight;

    float centerShiftXAligned = ceil(centerShiftX*edgeSizeXAligned/(edgeRatioX*2.))*(edgeRatioX*2.)/edgeSizeXAligned;
    float centerShiftYAligned = ceil(centerShiftY*edgeSizeYAligned/(edgeRatioY*2.))*(edgeRatioY*2.)/edgeSizeYAligned;

    float foveationScaleX = (centerSizeXAligned+(1.-centerSizeXAligned)/edgeRatioX);
    float foveationScaleY = (centerSizeYAligned+(1.-centerSizeYAligned)/edgeRatioY);

    float optimizedEyeWidth = foveationScaleX*targetEyeWidth;
    float optimizedEyeHeight = foveationScaleY*targetEyeHeight;

    // round the frame dimensions to a number of pixel multiple of 32 for the encoder
    auto optimizedEyeWidthAligned = (uint32_t)ceil(optimizedEyeWidth / 32.f) * 32;
    auto optimizedEyeHeightAligned = (uint32_t)ceil(optimizedEyeHeight / 32.f) * 32;

    float eyeWidthRatioAligned = optimizedEyeWidth/optimizedEyeWidthAligned;
    float eyeHeightRatioAligned = optimizedEyeHeight/optimizedEyeHeightAligned;

    m_foveatedRenderingPushConstants.targetEyeWidth = targetEyeWidth;
    m_foveatedRenderingPushConstants.targetEyeHeight = targetEyeHeight;
    m_foveatedRenderingPushConstants.optimizedEyeWidth = optimizedEyeWidthAligned;
    m_foveatedRenderingPushConstants.optimizedEyeHeight = optimizedEyeHeightAligned;
    m_foveatedRenderingPushConstants.eyeWidthRatio = eyeWidthRatioAligned;
    m_foveatedRenderingPushConstants.eyeHeightRatio = eyeHeightRatioAligned;
    m_foveatedRenderingPushConstants.centerSizeX = centerSizeXAligned;
    m_foveatedRenderingPushConstants.centerSizeY = centerSizeYAligned;
    m_foveatedRenderingPushConstants.centerShiftX = centerShiftXAligned;
    m_foveatedRenderingPushConstants.centerShiftY = centerShiftYAligned;
    m_foveatedRenderingPushConstants.edgeRatioX = edgeRatioX;
    m_foveatedRenderingPushConstants.edgeRatioY = edgeRatioY;

    m_width = m_foveatedRenderingPushConstants.optimizedEyeWidth * 2;
    m_height = m_foveatedRenderingPushConstants.optimizedEyeHeight;

    RenderPipeline *pipeline = new RenderPipeline(this);
    pipeline->SetShader(RenderPipeline::VertexShader, QUAD_SHADER_VERT_SPV_PTR, QUAD_SHADER_VERT_SPV_LEN);
    pipeline->SetShader(RenderPipeline::FragmentShader, FFR_SHADER_FRAG_SPV_PTR, FFR_SHADER_FRAG_SPV_LEN);
    pipeline->SetPushConstant(RenderPipeline::FragmentShader, &m_foveatedRenderingPushConstants, sizeof(m_foveatedRenderingPushConstants));
    m_pipelines.push_back(pipeline);
    AddPipeline(pipeline);
}

void FrameRender::setupCustomShaders(const std::string &stage)
{
    try {
        const std::filesystem::path shadersDir = std::filesystem::path(g_sessionPath).replace_filename("shaders");
        for (const auto &entry : std::filesystem::directory_iterator(shadersDir / std::filesystem::path(stage))) {
            std::ifstream fs(entry.path(), std::ios::binary | std::ios::in);
            uint32_t magic = 0;
            fs.read((char*)&magic, sizeof(uint32_t));
            if (magic != 0x07230203) {
                Warn("FrameRender: Shader file %s is not a SPIR-V file", entry.path().c_str());
                continue;
            }
            Info("FrameRender: Adding [%s] shader %s", stage.c_str(), entry.path().filename().c_str());
            RenderPipeline *pipeline = new RenderPipeline(this);
            pipeline->SetShader(RenderPipeline::VertexShader, QUAD_SHADER_VERT_SPV_PTR, QUAD_SHADER_VERT_SPV_LEN);
            pipeline->SetShader(RenderPipeline::FragmentShader, entry.path().c_str());
            m_pipelines.push_back(pipeline);
            AddPipeline(pipeline);
        }
    } catch (...) { }
}
