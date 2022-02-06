#include "Compositor.h"

uint64_t getNextId() {
    static int idx = 0;
    idx++;
    return idx;
}

Compositor::Compositor(std::shared_ptr<CD3DRender> pD3DRender,
                       std::shared_ptr<PoseHistory> poseHistory)
    : m_pD3DRender(pD3DRender), m_poseHistory(poseHistory) {}

void Compositor::SetEncoder(std::shared_ptr<CEncoder> pEncoder) { this->m_pEncoder = pEncoder; }

uint64_t Compositor::CreateTexture(
    uint32_t width, uint32_t height, uint32_t format, uint32_t sampleCount, void *texture) {

    D3D11_TEXTURE2D_DESC SharedTextureDesc = {};
    SharedTextureDesc.ArraySize = 1;
    SharedTextureDesc.MipLevels = 1;
    SharedTextureDesc.SampleDesc.Count = sampleCount;
    SharedTextureDesc.SampleDesc.Quality = 0;
    SharedTextureDesc.Usage = D3D11_USAGE_DEFAULT;
    SharedTextureDesc.Format = (DXGI_FORMAT)format;

    // Some(or all?) applications request larger texture than we specified in
    // GetRecommendedRenderTargetSize. But, we must create textures in requested size to prevent
    // cropped output. And then we must shrink texture to H.264 movie size.
    SharedTextureDesc.Width = width;
    SharedTextureDesc.Height = height;

    SharedTextureDesc.BindFlags = D3D11_BIND_SHADER_RESOURCE | D3D11_BIND_RENDER_TARGET;
    // SharedTextureDesc.MiscFlags = D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX |
    // D3D11_RESOURCE_MISC_SHARED_NTHANDLE;
    SharedTextureDesc.MiscFlags = D3D11_RESOURCE_MISC_SHARED;

    ComPtr<ID3D11Texture2D> d3dTexture;
    HRESULT hr = m_pD3DRender->GetDevice()->CreateTexture2D(&SharedTextureDesc, NULL, &d3dTexture);

    IDXGIResource *pResource;
    hr = d3dTexture->QueryInterface(__uuidof(IDXGIResource), (void **)&pResource);

    hr = pResource->GetSharedHandle((HANDLE *)texture);

    auto id = getNextId();

    m_textures.insert({id, d3dTexture});

    pResource->Release();

    return id;
}

void Compositor::DestroyTexture(uint64_t id) { this->m_textures.erase(id); }

void Compositor::PresentLayers(void *syncTexture, const Layer *layers, uint64_t layerCount) {
    if (layerCount > 0) {
        auto orientation = layers[0].views[0].orientation;

        vr::HmdMatrix34_t pPose;
        HmdMatrix_QuatToMat(orientation.w, orientation.x, orientation.y, orientation.z, &pPose);

        auto pose = m_poseHistory->GetBestPoseMatch(pPose);
        if (pose) {
            // found the frameIndex
            m_prevSubmitFrameIndex = m_submitFrameIndex;
            m_prevSubmitClientTime = m_submitClientTime;
            m_submitFrameIndex = pose->info.FrameIndex;
            m_submitClientTime = pose->info.clientTime;

            m_prevFramePoseRotation = m_framePoseRotation;
            m_framePoseRotation.x = pose->info.HeadPose_Pose_Orientation.x;
            m_framePoseRotation.y = pose->info.HeadPose_Pose_Orientation.y;
            m_framePoseRotation.z = pose->info.HeadPose_Pose_Orientation.z;
            m_framePoseRotation.w = pose->info.HeadPose_Pose_Orientation.w;

            Debug("Frame pose found. m_prevSubmitFrameIndex=%llu m_submitFrameIndex=%llu\n",
                  m_prevSubmitFrameIndex,
                  m_submitFrameIndex);
        } else {
            m_submitFrameIndex = 0;
            m_submitClientTime = 0;
            m_framePoseRotation = HmdQuaternion_Init(0.0, 0.0, 0.0, 0.0);
        }
    }

    IDXGIKeyedMutex *pKeyedMutex = NULL;

    ID3D11Texture2D *pSyncTexture = m_pD3DRender->GetSharedTexture(syncTexture);
    if (!pSyncTexture) {
        Warn("[VDispDvr] SyncTexture is NULL!\n");
        return;
    }

    if (SUCCEEDED(pSyncTexture->QueryInterface(__uuidof(IDXGIKeyedMutex), (void **)&pKeyedMutex))) {
        Debug("[VDispDvr] Wait for SyncTexture Mutex.\n");
        // TODO: Reasonable timeout and timeout handling
        HRESULT hr = pKeyedMutex->AcquireSync(0, 10);
        if (hr != S_OK) {
            Debug(
                "[VDispDvr] ACQUIRESYNC FAILED!!! hr=%d %p %ls\n", hr, hr, GetErrorStr(hr).c_str());
            pKeyedMutex->Release();
            return;
        }
    }

    uint64_t presentationTime = GetTimestampUs();

    ID3D11Texture2D *pTexture[10][2];
    vr::VRTextureBounds_t bounds[10][2];

    for (uint32_t i = 0; i < layerCount; i++) {
        for (uint32_t j = 0; j < 2; j++) {
            pTexture[i][j] = this->m_textures[layers[i].views[j].texture_id].Get();
            bounds[i][j].uMin = layers[i].views[j].rect_offset.x;
            bounds[i][j].vMin = layers[i].views[j].rect_offset.y;
            bounds[i][j].uMax = bounds[i][j].uMin + layers[i].views[j].rect_size.x;
            bounds[i][j].vMax = bounds[i][j].vMin + layers[i].views[j].rect_size.y;
        }
    }
    // This can go away, but is useful to see it as a separate packet on the gpu in traces.
    m_pD3DRender->GetContext()->Flush();

    if (m_pEncoder) {
        Debug("Waiting for finish of previous encode.\n");

        // Wait for the encoder to be ready.  This is important because the encoder thread
        // blocks on transmit which uses our shared d3d context (which is not thread safe).
        m_pEncoder->WaitForEncode();

        std::string debugText;

        uint64_t submitFrameIndex = m_submitFrameIndex + Settings::Instance().m_trackingFrameOffset;
        Debug("Fix frame index. FrameIndex=%llu Offset=%d New FrameIndex=%llu\n",
              m_submitFrameIndex,
              Settings::Instance().m_trackingFrameOffset,
              submitFrameIndex);

        // Copy entire texture to staging so we can read the pixels to send to remote device.
        m_pEncoder->CopyToStaging(pTexture,
                                  bounds,
                                  layerCount,
                                  false,
                                  presentationTime,
                                  submitFrameIndex,
                                  m_submitClientTime,
                                  "",
                                  debugText);

        m_pD3DRender->GetContext()->Flush();
    }

    if (pKeyedMutex) {
        pKeyedMutex->ReleaseSync(0);
        pKeyedMutex->Release();
    }
    Debug("[VDispDvr] Mutex Released.\n");

    if (m_pEncoder) {
        m_pEncoder->NewFrameReady();
    }
}