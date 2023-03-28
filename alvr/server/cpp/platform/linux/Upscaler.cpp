#include "Upscaler.h"
#include "alvr_server/bindings.h"

#include <math.h>

#define A_CPU
#include "shader/ffx_a.h"
#include "shader/ffx_fsr1.h"

template <typename T>
static constexpr T align(T x, uint32_t a)
{
    return (x + a - 1) & ~(a - 1);
}

Upscaler::Upscaler(Renderer *render, VkFormat format, uint32_t width, uint32_t height, float scale, uint32_t sharpness)
    : r(render)
    , m_width(width)
    , m_height(height)
    , m_scale(scale)
    , m_sharpness(sharpness)
{
    m_output.width = align<uint32_t>(m_width * m_scale, 32);
    m_output.height = align<uint32_t>(m_height * m_scale, 32);

    initFSR();
}

Upscaler::Output Upscaler::GetOutput()
{
    return m_output;
}

std::vector<RenderPipeline*> Upscaler::GetPipelines()
{
    return m_pipelines;
}

void Upscaler::initFSR()
{
    std::vector<VkSpecializationMapEntry> entries;

    // EASU
    RenderPipeline *easu = new RenderPipeline(r);
    if (r->m_fp16) {
        easu->SetShader(FSR_EASU16_SHADER_COMP_SPV_PTR, FSR_EASU16_SHADER_COMP_SPV_LEN);
    } else {
        easu->SetShader(FSR_EASU_SHADER_COMP_SPV_PTR, FSR_EASU_SHADER_COMP_SPV_LEN);
    }

    FsrEasuCon(&m_fsrEasuConstants.con0, &m_fsrEasuConstants.con1, &m_fsrEasuConstants.con2, &m_fsrEasuConstants.con3,
               m_width, m_height, m_width, m_height, m_output.width, m_output.height);

#define ENTRY(x) entries.push_back({(uint32_t)entries.size(), offsetof(FsrEasuConstants, x), sizeof(FsrEasuConstants::x)});
    ENTRY(con0);
    ENTRY(con0_1);
    ENTRY(con0_2);
    ENTRY(con0_3);
    ENTRY(con1);
    ENTRY(con1_1);
    ENTRY(con1_2);
    ENTRY(con1_3);
    ENTRY(con2);
    ENTRY(con2_1);
    ENTRY(con2_2);
    ENTRY(con2_3);
    ENTRY(con3);
    ENTRY(con3_1);
    ENTRY(con3_2);
    ENTRY(con3_3);
#undef ENTRY

    easu->SetConstants(&m_fsrEasuConstants, std::move(entries));
    easu->SetPixelsPerGroup(16, 16);

    m_pipelines.push_back(easu);

    // RCAS
    RenderPipeline *rcas = new RenderPipeline(r);
    rcas->SetShader(FSR_RCAS_SHADER_COMP_SPV_PTR, FSR_RCAS_SHADER_COMP_SPV_LEN);

    FsrRcasCon(&m_fsrRcasConstants.con0, m_sharpness / 10.0);

#define ENTRY(x) entries.push_back({(uint32_t)entries.size(), offsetof(FsrRcasConstants, x), sizeof(FsrRcasConstants::x)});
    ENTRY(con0);
    ENTRY(con0_1);
    ENTRY(con0_2);
    ENTRY(con0_3);
#undef ENTRY

    rcas->SetConstants(&m_fsrEasuConstants, std::move(entries));
    rcas->SetPixelsPerGroup(16, 16);

    m_pipelines.push_back(rcas);
}
