#include "FFR.h"

#include "alvr_server/Utils.h"
#include "alvr_server/bindings.h"

using Microsoft::WRL::ComPtr;
using namespace d3d_render_utils;

namespace {

struct FoveationVars {
    uint32_t targetEyeWidth;
    uint32_t targetEyeHeight;
    uint32_t optimizedEyeWidth;
    uint32_t optimizedEyeHeight;

    float eyeWidthRatio;
    float eyeHeightRatio;

    float centerSizeX;
    float centerSizeY;
    float centerShiftLeftX;
    float centerShiftLeftY;
    float centerShiftRightX;
    float centerShiftRightY;
    float edgeRatioX;
    float edgeRatioY;
    float padding[2];
};

static_assert(sizeof(FoveationVars) == 64, "FoveationVars must match the HLSL constant buffer");

FoveationVars CalculateFoveationVars() {
    const auto* settings = Settings_Instance();
    const auto& params = settings->m_foveatedEncoding;

    return { settings->m_renderWidth / 2,
             settings->m_renderHeight,
             params.encodedViewResolution[0],
             params.encodedViewResolution[1],
             params.viewRatio[0],
             params.viewRatio[1],
             params.centerSize[0],
             params.centerSize[1],
             params.centerShifts[0][0],
             params.centerShifts[0][1],
             params.centerShifts[1][0],
             params.centerShifts[1][1],
             params.edgeRatio[0],
             params.edgeRatio[1],
             { 0.0f, 0.0f } };
}
}

void FFR::GetOptimizedResolution(uint32_t* width, uint32_t* height) {
    auto fovVars = CalculateFoveationVars();
    *width = fovVars.optimizedEyeWidth * 2;
    *height = fovVars.optimizedEyeHeight;
}

FFR::FFR(ID3D11Device* device)
    : mDevice(device) { }

void FFR::Initialize(ID3D11Texture2D* compositionTexture) {
    auto fovVars = CalculateFoveationVars();
    mFoveatedRenderingBuffer = CreateBuffer(mDevice.Get(), fovVars, D3D11_USAGE_DEFAULT);
    mDevice->GetImmediateContext(&mImmediateContext);

    std::vector<uint8_t> quadShaderCSO(
        QUAD_SHADER_CSO_PTR, QUAD_SHADER_CSO_PTR + QUAD_SHADER_CSO_LEN
    );
    mQuadVertexShader = CreateVertexShader(mDevice.Get(), quadShaderCSO);

    mOptimizedTexture = CreateTexture(
        mDevice.Get(),
        fovVars.optimizedEyeWidth * 2,
        fovVars.optimizedEyeHeight,
        Settings_Instance()->m_enableHdr ? DXGI_FORMAT_R16G16B16A16_FLOAT
                                         : DXGI_FORMAT_R8G8B8A8_UNORM_SRGB
    );

    if (Settings_Instance()->m_enableFoveatedEncoding) {
        std::vector<uint8_t> compressAxisAlignedShaderCSO(
            COMPRESS_AXIS_ALIGNED_CSO_PTR,
            COMPRESS_AXIS_ALIGNED_CSO_PTR + COMPRESS_AXIS_ALIGNED_CSO_LEN
        );
        auto compressAxisAlignedPipeline = RenderPipeline(mDevice.Get());
        compressAxisAlignedPipeline.Initialize(
            { compositionTexture },
            mQuadVertexShader.Get(),
            compressAxisAlignedShaderCSO,
            mOptimizedTexture.Get(),
            mFoveatedRenderingBuffer.Get()
        );

        mPipelines.push_back(compressAxisAlignedPipeline);
    } else {
        mOptimizedTexture = compositionTexture;
    }
}

void FFR::Render() {
    auto fovVars = CalculateFoveationVars();
    UpdateBuffer(mImmediateContext.Get(), mFoveatedRenderingBuffer.Get(), &fovVars);

    for (auto& p : mPipelines) {
        p.Render();
    }
}

ID3D11Texture2D* FFR::GetOutputTexture() { return mOptimizedTexture.Get(); }
