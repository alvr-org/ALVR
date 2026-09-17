#include "FrameRender.h"
#include "alvr_server/Logger.h"
#include "alvr_server/bindings.h"

#include <algorithm>
#include <cmath>
#include <filesystem>
#include <fstream>

FrameRender::FrameRender(alvr::VkContext& ctx, init_packet& init, int fds[])
    : Renderer(
          ctx.get_vk_instance(),
          ctx.get_vk_device(),
          ctx.get_vk_phys_device(),
          ctx.get_vk_queue_family_index(),
          ctx.get_vk_device_extensions()
      ) {
    m_quadShaderSize = QUAD_SHADER_COMP_SPV_LEN;
    m_quadShaderCode = reinterpret_cast<const uint32_t*>(QUAD_SHADER_COMP_SPV_PTR);

    Startup(
        init.image_create_info.extent.width,
        init.image_create_info.extent.height,
        init.image_create_info.format
    );

    for (size_t i = 0; i < 3; ++i) {
        AddImage(init.image_create_info, init.mem_index, fds[2 * i], fds[2 * i + 1]);
    }

    m_width = Settings_Instance()->m_renderWidth;
    m_height = Settings_Instance()->m_renderHeight;

    Info("FrameRender: Input size %ux%u", m_width, m_height);

    if (Settings_Instance()->m_forceSwEncoding) {
        m_handle = ExternalHandle::None;
    } else if (ctx.amd || ctx.intel) {
        m_handle = ExternalHandle::DmaBuf;
    } else if (ctx.nvidia) {
        m_handle = ExternalHandle::OpaqueFd;
    }

    setupCustomShaders("pre");

    if (Settings_Instance()->m_enableColorCorrection) {
        setupColorCorrection();
    }

    if (Settings_Instance()->m_enableFoveatedEncoding) {
        setupFoveatedRendering();
    }

    setupCustomShaders("post");

    if (m_pipelines.empty()) {
        RenderPipeline* pipeline = new RenderPipeline(this);
        pipeline->SetShader(QUAD_SHADER_COMP_SPV_PTR, QUAD_SHADER_COMP_SPV_LEN);
        m_pipelines.push_back(pipeline);
        AddPipeline(pipeline);
    }

    Info("FrameRender: Output size %ux%u", m_width, m_height);
}

FrameRender::~FrameRender() {
    for (RenderPipeline* pipeline : m_pipelines) {
        delete pipeline;
    }
}

FrameRender::Output FrameRender::CreateOutput() {
    Renderer::CreateOutput(m_width, m_height, m_handle);
    return GetOutput();
}

uint32_t FrameRender::GetEncodingWidth() const { return m_width; }

uint32_t FrameRender::GetEncodingHeight() const { return m_height; }

void FrameRender::Render(uint32_t index, uint64_t waitValue, uint64_t targetTimestampNs) {
    if (Settings_Instance()->m_enableFoveatedEncoding) {
        // These are the push constants consumed by the FFR pipeline for this frame.
        ReportEncoderFoveationCenters(
            targetTimestampNs,
            m_foveationCenterShifts.leftX,
            m_foveationCenterShifts.leftY,
            m_foveationCenterShifts.rightX,
            m_foveationCenterShifts.rightY
        );
    }

    Renderer::Render(index, waitValue);
}

void FrameRender::setupColorCorrection() {
    std::vector<VkSpecializationMapEntry> entries;

#define ENTRY(x, v)                                                                                \
    m_colorCorrectionConstants.x = v;                                                              \
    entries.push_back(                                                                             \
        { (uint32_t)entries.size(), offsetof(ColorCorrection, x), sizeof(ColorCorrection::x) }     \
    );

    ENTRY(renderWidth, m_width);
    ENTRY(renderHeight, m_height);
    ENTRY(brightness, Settings_Instance()->m_brightness);
    ENTRY(contrast, Settings_Instance()->m_contrast + 1.f);
    ENTRY(saturation, Settings_Instance()->m_saturation + 1.f);
    ENTRY(gamma, Settings_Instance()->m_gamma);
    ENTRY(sharpening, Settings_Instance()->m_sharpening);
#undef ENTRY

    RenderPipeline* pipeline = new RenderPipeline(this);
    pipeline->SetShader(COLOR_SHADER_COMP_SPV_PTR, COLOR_SHADER_COMP_SPV_LEN);
    pipeline->SetConstants(&m_colorCorrectionConstants, std::move(entries));
    m_pipelines.push_back(pipeline);
    AddPipeline(pipeline);
}

void FrameRender::setupFoveatedRendering() {
    const auto& params = Settings_Instance()->m_foveatedEncoding;
    m_width = params.encodedViewResolution[0] * 2;
    m_height = params.encodedViewResolution[1];

    std::vector<VkSpecializationMapEntry> entries;

#define ENTRY(x, v)                                                                                \
    m_foveatedRenderingConstants.x = v;                                                            \
    entries.push_back(                                                                             \
        { (uint32_t)entries.size(), offsetof(FoveationVars, x), sizeof(FoveationVars::x) }         \
    );

    ENTRY(eyeWidthRatio, params.viewRatio[0]);
    ENTRY(eyeHeightRatio, params.viewRatio[1]);
    ENTRY(centerSizeX, params.centerSize[0]);
    ENTRY(centerSizeY, params.centerSize[1]);
    ENTRY(edgeRatioX, params.edgeRatio[0]);
    ENTRY(edgeRatioY, params.edgeRatio[1]);
#undef ENTRY

    m_foveationCenterShifts = { params.centerShifts[0][0],
                                params.centerShifts[0][1],
                                params.centerShifts[1][0],
                                params.centerShifts[1][1] };

    RenderPipeline* pipeline = new RenderPipeline(this);
    pipeline->SetShader(FFR_SHADER_COMP_SPV_PTR, FFR_SHADER_COMP_SPV_LEN);
    pipeline->SetConstants(&m_foveatedRenderingConstants, std::move(entries));
    pipeline->SetPushConstants(&m_foveationCenterShifts);
    m_pipelines.push_back(pipeline);
    AddPipeline(pipeline);
}

void FrameRender::setupCustomShaders(const std::string& stage) {
    try {
        const std::filesystem::path shadersDir
            = std::filesystem::path(g_sessionPath).replace_filename("shaders");
        for (const auto& entry :
             std::filesystem::directory_iterator(shadersDir / std::filesystem::path(stage))) {
            std::ifstream fs(entry.path(), std::ios::binary | std::ios::in);
            uint32_t magic = 0;
            fs.read((char*)&magic, sizeof(uint32_t));
            if (magic != 0x07230203) {
                Warn("FrameRender: Shader file %s is not a SPIR-V file", entry.path().c_str());
                continue;
            }
            Info(
                "FrameRender: Adding [%s] shader %s", stage.c_str(), entry.path().filename().c_str()
            );
            RenderPipeline* pipeline = new RenderPipeline(this);
            pipeline->SetShader(entry.path().c_str());
            m_pipelines.push_back(pipeline);
            AddPipeline(pipeline);
        }
    } catch (...) { }
}
