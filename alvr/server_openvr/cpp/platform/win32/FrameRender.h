#pragma once

#include <memory>
#include <stdint.h>
#include <string>

#include <d3d11.h>
#include <d3dcompiler.h>
#include <directxcolors.h>
#include <directxmath.h>
#include <wrl.h>

#include <cinttypes>
#include <dxgi.h>
#include <unknwn.h>
#include <windows.h>

#include "FFR.h"
#include "alvr_server/openvr_driver_wrap.h"
#include "d3d-render-utils/RenderPipelineYUV.h"
#include "shared/d3drender.h"

#define GPU_PRIORITY_VAL 7

using Microsoft::WRL::ComPtr;

template <class T> class ComQIPtr : public ComPtr<T> {

public:
    inline ComQIPtr(IUnknown* unk) {
        this->ptr_ = nullptr;
        unk->QueryInterface(__uuidof(T), (void**)this->GetAddressOf());
    }

    inline ComPtr<T>& operator=(IUnknown* unk) {
        ComPtr<T>::Clear();
        unk->QueryInterface(__uuidof(T), (void**)this->GetAddressOf());
        return *this;
    }
};

class FrameRender {
public:
    FrameRender(std::shared_ptr<CD3DRender> pD3DRender);
    virtual ~FrameRender();

    bool Startup();
    void SetViewParams(
        vr::HmdRect2_t projLeft,
        vr::HmdMatrix34_t eyeToHeadLeft,
        vr::HmdRect2_t projRight,
        vr::HmdMatrix34_t eyeToHeadRight
    );
    bool RenderFrame(
        ID3D11Texture2D* pTexture[][2],
        vr::VRTextureBounds_t bounds[][2],
        vr::HmdMatrix34_t poses[],
        int layerCount,
        bool recentering,
        const std::string& message,
        const std::string& debugText
    );
    void GetEncodingResolution(uint32_t* width, uint32_t* height);

    ComPtr<ID3D11Texture2D> GetTexture();

private:
    std::shared_ptr<CD3DRender> m_pD3DRender;
    ComPtr<ID3D11Texture2D> m_pStagingTexture;

    ComPtr<ID3D11VertexShader> m_pVertexShader;
    ComPtr<ID3D11PixelShader> m_pPixelShader;

    ComPtr<ID3D11InputLayout> m_pVertexLayout;
    ComPtr<ID3D11Buffer> m_pVertexBuffer;
    ComPtr<ID3D11Buffer> m_pIndexBuffer;
    ComPtr<ID3D11Buffer> m_pFrameRenderCBuffer;

    ComPtr<ID3D11SamplerState> m_pSamplerLinear;

    ComPtr<ID3D11RenderTargetView> m_pRenderTargetView;
    ComPtr<ID3D11DepthStencilState> m_depthStencilState;

    D3D11_VIEWPORT m_viewportL, m_viewportR, m_viewport;
    D3D11_RECT m_scissorL, m_scissorR, m_scissor;

    ComPtr<ID3D11BlendState> m_pBlendStateFirst;
    ComPtr<ID3D11BlendState> m_pBlendState;

    ComPtr<ID3D11Resource> m_recenterTexture;
    ComPtr<ID3D11ShaderResourceView> m_recenterResourceView;
    ComPtr<ID3D11Resource> m_messageBGTexture;
    ComPtr<ID3D11ShaderResourceView> m_messageBGResourceView;

    vr::HmdRect2_t m_viewProj[2];
    vr::HmdMatrix34_t m_eyeToHead[2];

    struct SimpleVertex {
        DirectX::XMFLOAT4 Pos;
        DirectX::XMFLOAT2 Tex;
        uint32_t View;
    };
    // Parameter for Draw method. 2-triangles for both eyes.
    static const int VERTEX_INDEX_COUNT = 12;

    std::unique_ptr<d3d_render_utils::RenderPipeline> m_colorCorrectionPipeline;
    bool enableColorCorrection;

    std::unique_ptr<FFR> m_ffr;
    bool enableFFE;

    std::unique_ptr<d3d_render_utils::RenderPipelineYUV> m_yuvPipeline;

    static bool SetGpuPriority(ID3D11Device* device) {
        typedef enum _D3DKMT_SCHEDULINGPRIORITYCLASS {
            D3DKMT_SCHEDULINGPRIORITYCLASS_IDLE,
            D3DKMT_SCHEDULINGPRIORITYCLASS_BELOW_NORMAL,
            D3DKMT_SCHEDULINGPRIORITYCLASS_NORMAL,
            D3DKMT_SCHEDULINGPRIORITYCLASS_ABOVE_NORMAL,
            D3DKMT_SCHEDULINGPRIORITYCLASS_HIGH,
            D3DKMT_SCHEDULINGPRIORITYCLASS_REALTIME
        } D3DKMT_SCHEDULINGPRIORITYCLASS;

        ComQIPtr<IDXGIDevice> dxgiDevice(device);
        if (!dxgiDevice) {
            Info("[GPU PRIO FIX] Failed to get IDXGIDevice\n");
            return false;
        }

        HMODULE gdi32 = GetModuleHandleW(L"GDI32");
        if (!gdi32) {
            Info("[GPU PRIO FIX] Failed to get GDI32\n");
            return false;
        }

        NTSTATUS(WINAPI * d3dkmt_spspc)(HANDLE, D3DKMT_SCHEDULINGPRIORITYCLASS);
        d3dkmt_spspc = (decltype(d3dkmt_spspc))GetProcAddress(
            gdi32, "D3DKMTSetProcessSchedulingPriorityClass"
        );
        if (!d3dkmt_spspc) {
            Info("[GPU PRIO FIX] Failed to get d3dkmt_spspc\n");
            return false;
        }

        NTSTATUS status
            = d3dkmt_spspc(GetCurrentProcess(), D3DKMT_SCHEDULINGPRIORITYCLASS_REALTIME);
        if (status
            == 0xc0000022) { // STATUS_ACCESS_DENIED, see http://deusexmachina.uk/ntstatus.html
            Info(
                "[GPU PRIO FIX] Failed to set process (%d) priority class, please run ALVR as "
                "Administrator.\n",
                GetCurrentProcess()
            );
            return false;
        } else if (status != 0) {
            Info(
                "[GPU PRIO FIX] Failed to set process (%d) priority class: %u\n",
                GetCurrentProcess(),
                status
            );
            return false;
        }

        HRESULT hr = dxgiDevice->SetGPUThreadPriority(GPU_PRIORITY_VAL);
        if (FAILED(hr)) {
            Info("[GPU PRIO FIX] SetGPUThreadPriority failed\n");
            return false;
        }

        Debug("[GPU PRIO FIX] D3D11 GPU priority setup success\n");
        return true;
    }
};
