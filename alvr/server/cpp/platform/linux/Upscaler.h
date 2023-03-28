#pragma once

#include "Renderer.h"

class Upscaler
{
public:
    struct Output {
        uint32_t width = 0;
        uint32_t height = 0;
    };

    Upscaler(Renderer *render, VkFormat format, uint32_t width, uint32_t height, float scale, uint32_t sharpness);

    Output GetOutput();
    std::vector<RenderPipeline*> GetPipelines();

private:
    void initFSR();

    Renderer *r;
    uint32_t m_width;
    uint32_t m_height;
    float m_scale;
    uint32_t m_sharpness;
    Output m_output;
    std::vector<RenderPipeline*> m_pipelines;

    struct FsrEasuConstants {
        uint32_t con0;
        uint32_t con0_1;
        uint32_t con0_2;
        uint32_t con0_3;
        uint32_t con1;
        uint32_t con1_1;
        uint32_t con1_2;
        uint32_t con1_3;
        uint32_t con2;
        uint32_t con2_1;
        uint32_t con2_2;
        uint32_t con2_3;
        uint32_t con3;
        uint32_t con3_1;
        uint32_t con3_2;
        uint32_t con3_3;
    } m_fsrEasuConstants;

    struct FsrRcasConstants {
        uint32_t con0;
        uint32_t con0_1;
        uint32_t con0_2;
        uint32_t con0_3;
    } m_fsrRcasConstants;
};
