#pragma once

#include "VideoEncoder.h"
#include "shared/d3drender.h"
#include <atlbase.h>
#include <d3d11.h>
#include <dxgi.h>
#include <vector>

#include "vpl/mfx.h"
#include "vpl/mfxjpeg.h"
#include "vpl/mfxmemory.h"
#include "vpl/mfxstructures.h"
#include "vpl/mfxvideo.h"
#if (MFX_VERSION >= 2000)
#include "vpl/mfxdispatcher.h"
#endif

class VideoEncoderVPL : public VideoEncoder {
public:
    VideoEncoderVPL(std::shared_ptr<CD3DRender> pD3DRender, int width, int height);
    ~VideoEncoderVPL();

    void Initialize();
    void Shutdown();

    void Transmit(
        ID3D11Texture2D* pTexture,
        uint64_t presentationTime,
        uint64_t targetTimestampNs,
        bool insertIDR
    );

private:
    void CheckVPLConfig();
    void ChooseParams();
    void InitTransferTex();
    void InitVpl();
    void InitVplEncode();
    mfxFrameSurface1* VplImportTexture(ID3D11Texture2D* texture);
    void LogImplementationInfo();

    std::shared_ptr<CD3DRender> m_pD3DRender;
    int m_codec;
    int m_renderWidth;
    int m_renderHeight;
    int m_refreshRate;
    int m_bitrateInMBits;

    mfxU32 m_vplCodec;
    mfxU32 m_vplCodecProfile;
    mfxU32 m_vplColorFormat;
    mfxU32 m_vplChromaFormat;
    mfxU32 m_vplQualityPreset;
    mfxU32 m_vplRateControlMode;
    DXGI_FORMAT m_dxColorFormat;
    mfxVideoParam m_vplEncodeParams = {};

    mfxLoader m_vplLoader = nullptr;
    mfxSession m_vplSession = nullptr;
    mfxBitstream m_vplBitstream = {};
    mfxMemoryInterface* m_vplMemoryInterface = nullptr;
    CComPtr<ID3D11Texture2D> m_transferTex;
};
