#include "OvrDirectModeComponent.h"

OvrDirectModeComponent::OvrDirectModeComponent(
    std::shared_ptr<CD3DRender> pD3DRender, std::shared_ptr<PoseHistory> poseHistory
)
    : m_pD3DRender(pD3DRender)
    , m_poseHistory(poseHistory)
    , m_submitLayer(0) { }

void OvrDirectModeComponent::SetEncoder(std::shared_ptr<CEncoder> pEncoder) {
    m_pEncoder = pEncoder;
}

/** Specific to Oculus compositor support, textures supplied must be created using this method. */
void OvrDirectModeComponent::CreateSwapTextureSet(
    uint32_t unPid,
    const SwapTextureSetDesc_t* pSwapTextureSetDesc,
    SwapTextureSet_t* pOutSwapTextureSet
) {
    Debug(
        "OvrDirectModeComponent::CreateSwapTextureSet pid=%d Format=%d %dx%d SampleCount=%d",
        unPid,
        pSwapTextureSetDesc->nFormat,
        pSwapTextureSetDesc->nWidth,
        pSwapTextureSetDesc->nHeight,
        pSwapTextureSetDesc->nSampleCount
    );

    // HRESULT hr = D3D11CreateDevice(pAdapter, D3D_DRIVER_TYPE_HARDWARE, NULL, creationFlags, NULL,
    // 0, D3D11_SDK_VERSION, &pDevice, &eFeatureLevel, &pContext);

    D3D11_TEXTURE2D_DESC SharedTextureDesc = {};
    DXGI_FORMAT format = (DXGI_FORMAT)pSwapTextureSetDesc->nFormat;
    SharedTextureDesc.BindFlags = D3D11_BIND_SHADER_RESOURCE | D3D11_BIND_RENDER_TARGET;
    if (format == DXGI_FORMAT_R32G8X24_TYPELESS || format == DXGI_FORMAT_R32_TYPELESS) {
        SharedTextureDesc.BindFlags = D3D11_BIND_DEPTH_STENCIL;
    }
    SharedTextureDesc.ArraySize = 1;
    SharedTextureDesc.MipLevels = 1;
    SharedTextureDesc.SampleDesc.Count
        = pSwapTextureSetDesc->nSampleCount == 0 ? 1 : pSwapTextureSetDesc->nSampleCount;
    SharedTextureDesc.SampleDesc.Quality = 0;
    SharedTextureDesc.Usage = D3D11_USAGE_DEFAULT;
    SharedTextureDesc.Format = format;

    // Some(or all?) applications request larger texture than we specified in
    // GetRecommendedRenderTargetSize. But, we must create textures in requested size to prevent
    // cropped output. And then we must shrink texture to H.264 movie size.
    SharedTextureDesc.Width = pSwapTextureSetDesc->nWidth;
    SharedTextureDesc.Height = pSwapTextureSetDesc->nHeight;

    // SharedTextureDesc.MiscFlags = D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX |
    // D3D11_RESOURCE_MISC_SHARED_NTHANDLE;
    SharedTextureDesc.MiscFlags = D3D11_RESOURCE_MISC_SHARED;

    ProcessResource* processResource = new ProcessResource();
    processResource->pid = unPid;

    for (int i = 0; i < 3; i++) {
        HRESULT hr = m_pD3DRender->GetDevice()->CreateTexture2D(
            &SharedTextureDesc, NULL, &processResource->textures[i]
        );
        // LogDriver("texture%d %p res:%d %s", i, texture[i], hr, GetDxErrorStr(hr).c_str());
        if (FAILED(hr)) {
            Error("CreateSwapTextureSet CreateTexture2D %p %ls", hr, GetErrorStr(hr).c_str());
            delete processResource;
            break;
        }

        IDXGIResource* pResource;
        hr = processResource->textures[i]->QueryInterface(
            __uuidof(IDXGIResource), (void**)&pResource
        );
        if (FAILED(hr)) {
            Error("CreateSwapTextureSet QueryInterface %p %ls", hr, GetErrorStr(hr).c_str());
            delete processResource;
            break;
        }
        // LogDriver("QueryInterface %p res:%d %s", pResource, hr, GetDxErrorStr(hr).c_str());

        hr = pResource->GetSharedHandle(&processResource->sharedHandles[i]);
        if (FAILED(hr)) {
            Error("CreateSwapTextureSet GetSharedHandle %p %ls", hr, GetErrorStr(hr).c_str());
            delete processResource;
            pResource->Release();
            break;
        }
        // LogDriver("GetSharedHandle %p res:%d %s", processResource->sharedHandles[i], hr,
        // GetDxErrorStr(hr).c_str());

        m_handleMap.insert(
            std::make_pair(processResource->sharedHandles[i], std::make_pair(processResource, i))
        );

        pOutSwapTextureSet->rSharedTextureHandles[i]
            = (vr::SharedTextureHandle_t)processResource->sharedHandles[i];

        pResource->Release();

        Debug("Created Texture %d %p", i, processResource->sharedHandles[i]);
    }
    // m_processMap.insert(std::pair<uint32_t, ProcessResource *>(unPid, processResource));
}

/** Used to textures created using CreateSwapTextureSet.  Only one of the set's handles needs to be
 * used to destroy the entire set. */
void OvrDirectModeComponent::DestroySwapTextureSet(vr::SharedTextureHandle_t sharedTextureHandle) {
    Debug("OvrDirectModeComponent::DestroySwapTextureSet %p", sharedTextureHandle);

    auto it = m_handleMap.find((HANDLE)sharedTextureHandle);
    if (it != m_handleMap.end()) {
        // Release all reference (a bit forcible)
        ProcessResource* p = it->second.first;
        m_handleMap.erase(p->sharedHandles[0]);
        m_handleMap.erase(p->sharedHandles[1]);
        m_handleMap.erase(p->sharedHandles[2]);
        delete p;
    } else {
        Debug("Requested to destroy not managing texture. handle:%p", sharedTextureHandle);
    }
}

/** Used to purge all texture sets for a given process. */
void OvrDirectModeComponent::DestroyAllSwapTextureSets(uint32_t unPid) {
    Debug("OvrDirectModeComponent::DestroyAllSwapTextureSets pid=%d", unPid);

    for (auto it = m_handleMap.begin(); it != m_handleMap.end();) {
        if (it->second.first->pid == unPid) {
            if (it->second.second == 0) {
                delete it->second.first;
            }
            m_handleMap.erase(it++);
        } else {
            ++it;
        }
    }
}

/** After Present returns, calls this to get the next index to use for rendering. */
void OvrDirectModeComponent::GetNextSwapTextureSetIndex(
    vr::SharedTextureHandle_t sharedTextureHandles[2], uint32_t (*pIndices)[2]
) {
    Debug("OvrDirectModeComponent::GetNextSwapTextureSetIndex");

    (*pIndices)[0]++;
    (*pIndices)[0] %= 3;
    (*pIndices)[1]++;
    (*pIndices)[1] %= 3;
}

/** Call once per layer to draw for this frame.  One shared texture handle per eye.  Textures must
 * be created using CreateSwapTextureSet and should be alternated per frame.  Call Present once all
 * layers have been submitted. */
void OvrDirectModeComponent::SubmitLayer(const SubmitLayerPerEye_t (&perEye)[2]) {
    Debug("OvrDirectModeComponent::SubmitLayer");

    m_presentMutex.lock();

    // mHmdPose is the same pose for both eyes, getting the eye view pose
    //  requires some records keeping, unfortunately (m_eyeToHead)
    auto pPose = &perEye[0].mHmdPose;

    if (m_submitLayer == 0) {
        // Detect FrameIndex of submitted frame by pPose.
        // This is important part to achieve smooth headtracking.
        // We search for history of TrackingInfo and find the TrackingInfo which have nearest matrix
        // value.

        auto pose = m_poseHistory->GetBestPoseMatch(*pPose);
        if (pose) {
            // found the frameIndex
            m_prevTargetTimestampNs = m_targetTimestampNs;
            m_targetTimestampNs = pose->targetTimestampNs;

            m_prevFramePoseRotation = m_framePoseRotation;
            m_framePoseRotation.x = pose->motion.pose.orientation.x;
            m_framePoseRotation.y = pose->motion.pose.orientation.y;
            m_framePoseRotation.z = pose->motion.pose.orientation.z;
            m_framePoseRotation.w = pose->motion.pose.orientation.w;
        } else {
            m_targetTimestampNs = 0;
            m_framePoseRotation = HmdQuaternion_Init(0.0, 0.0, 0.0, 0.0);
        }
    }
    if (m_submitLayer < MAX_LAYERS) {
        m_submitLayers[m_submitLayer][0] = perEye[0];
        m_submitLayers[m_submitLayer][1] = perEye[1];
        m_submitLayer++;
    } else {
        Warn("Too many layers submitted!");
    }

    // CopyTexture();

    m_presentMutex.unlock();
}

/** Submits queued layers for display. */
void OvrDirectModeComponent::Present(vr::SharedTextureHandle_t syncTexture) {
    Debug("OvrDirectModeComponent::Present");

    m_presentMutex.lock();

    ReportPresent(m_targetTimestampNs, 0);

    bool useMutex = true;

    IDXGIKeyedMutex* pKeyedMutex = NULL;

    uint32_t layerCount = m_submitLayer;
    m_submitLayer = 0;

    if (m_prevTargetTimestampNs == m_targetTimestampNs) {
        Debug("Discard duplicated frame. FrameIndex=%llu (Ignoring)", m_targetTimestampNs);
        // return;
    }

    ID3D11Texture2D* pSyncTexture = m_pD3DRender->GetSharedTexture((HANDLE)syncTexture);
    if (!pSyncTexture) {
        Warn("[VDispDvr] SyncTexture is NULL!");
        m_presentMutex.unlock();
        return;
    }

    if (useMutex) {
        // Access to shared texture must be wrapped in AcquireSync/ReleaseSync
        // to ensure the compositor has finished rendering to it before it gets used.
        // This enforces scheduling of work on the gpu between processes.
        if (SUCCEEDED(
                pSyncTexture->QueryInterface(__uuidof(IDXGIKeyedMutex), (void**)&pKeyedMutex)
            )) {
            // TODO: Reasonable timeout and timeout handling
            HRESULT hr = pKeyedMutex->AcquireSync(0, 10);
            if (hr != S_OK) {
                Debug(
                    "[VDispDvr] ACQUIRESYNC FAILED!!! hr=%d %p %ls", hr, hr, GetErrorStr(hr).c_str()
                );
                pKeyedMutex->Release();
                m_presentMutex.unlock();
                return;
            }
        }
    }

    CopyTexture(layerCount);

    if (useMutex) {
        if (pKeyedMutex) {
            pKeyedMutex->ReleaseSync(0);
            pKeyedMutex->Release();
        }
    }

    ReportComposed(m_targetTimestampNs, 0);

    if (m_pEncoder) {
        m_pEncoder->NewFrameReady();
    }

    m_presentMutex.unlock();
}

void OvrDirectModeComponent::PostPresent() {
    Debug("OvrDirectModeComponent::PostPresent");

    WaitForVSync();
}

void OvrDirectModeComponent::CopyTexture(uint32_t layerCount) {

    uint64_t presentationTime = GetTimestampUs();

    ID3D11Texture2D* pTexture[MAX_LAYERS][2];
    ComPtr<ID3D11Texture2D> Texture[MAX_LAYERS][2];
    vr::VRTextureBounds_t bounds[MAX_LAYERS][2];
    vr::HmdMatrix34_t poses[MAX_LAYERS];

    for (uint32_t i = 0; i < layerCount; i++) {
        // Find left eye texture.
        HANDLE leftEyeTexture = (HANDLE)m_submitLayers[i][0].hTexture;
        auto it = m_handleMap.find(leftEyeTexture);
        if (it == m_handleMap.end()) {
            // Ignore this layer.
            Debug(
                "Submitted texture is not found on HandleMap. eye=right layer=%d/%d Texture "
                "Handle=%p",
                i,
                layerCount,
                leftEyeTexture
            );
        } else {
            Texture[i][0] = it->second.first->textures[it->second.second];
            D3D11_TEXTURE2D_DESC desc;
            Texture[i][0]->GetDesc(&desc);

            // Find right eye texture.
            HANDLE rightEyeTexture = (HANDLE)m_submitLayers[i][1].hTexture;
            it = m_handleMap.find(rightEyeTexture);
            if (it == m_handleMap.end()) {
                // Ignore this layer
                Debug(
                    "Submitted texture is not found on HandleMap. eye=left layer=%d/%d Texture "
                    "Handle=%p",
                    i,
                    layerCount,
                    rightEyeTexture
                );
                Texture[i][0].Reset();
            } else {
                Texture[i][1] = it->second.first->textures[it->second.second];
            }
        }

        pTexture[i][0] = Texture[i][0].Get();
        pTexture[i][1] = Texture[i][1].Get();
        bounds[i][0] = m_submitLayers[i][0].bounds;
        bounds[i][1] = m_submitLayers[i][1].bounds;
        poses[i] = m_submitLayers[i][0].mHmdPose;
    }

    // This can go away, but is useful to see it as a separate packet on the gpu in traces.
    m_pD3DRender->GetContext()->Flush();

    if (m_pEncoder) {
        // Wait for the encoder to be ready.  This is important because the encoder thread
        // blocks on transmit which uses our shared d3d context (which is not thread safe).
        m_pEncoder->WaitForEncode();

        std::string debugText;

        uint64_t submitFrameIndex = m_targetTimestampNs;

        // Copy entire texture to staging so we can read the pixels to send to remote device.
        m_pEncoder->CopyToStaging(
            pTexture,
            bounds,
            poses,
            layerCount,
            false,
            presentationTime,
            submitFrameIndex,
            "",
            debugText
        );

        m_pD3DRender->GetContext()->Flush();
    }
}
