#pragma once
#include "alvr_server/PoseHistory.h"
#include "alvr_server/Utils.h"
#include "openvr_driver.h"

#include "alvr_server/Settings.h"

#include <map>
#include <mutex>

#include "Encoder.hpp"

class OvrDirectModeComponent : public vr::IVRDriverDirectModeComponent {
public:
    OvrDirectModeComponent(
        /* std::shared_ptr<Renderer> pVKRender,  */ std::shared_ptr<PoseHistory> poseHistory
    );

    void RequestIdr() { enc.requestIdr(); }

    /** Specific to Oculus compositor support, textures supplied must be created using this method.
     */
    virtual void CreateSwapTextureSet(
        uint32_t unPid,
        const SwapTextureSetDesc_t* pSwapTextureSetDesc,
        SwapTextureSet_t* pOutSwapTextureSet
    );

    /** Used to textures created using CreateSwapTextureSet.  Only one of the set's handles needs to
     * be used to destroy the entire set. */
    virtual void DestroySwapTextureSet(vr::SharedTextureHandle_t sharedTextureHandle);

    /** Used to purge all texture sets for a given process. */
    virtual void DestroyAllSwapTextureSets(uint32_t unPid);

    /** After Present returns, calls this to get the next index to use for rendering. */
    virtual void GetNextSwapTextureSetIndex(
        vr::SharedTextureHandle_t sharedTextureHandles[2], uint32_t (*pIndices)[2]
    );

    /** Call once per layer to draw for this frame.  One shared texture handle per eye.  Textures
     * must be created using CreateSwapTextureSet and should be alternated per frame.  Call Present
     * once all layers have been submitted. */
    virtual void SubmitLayer(const SubmitLayerPerEye_t (&perEye)[2]);

    /** Submits queued layers for display. */
    virtual void Present(vr::SharedTextureHandle_t syncTexture);

    /** Called after Present to allow driver to take more time until vsync after they've
     * successfully acquired the sync texture in Present.*/
    virtual void PostPresent(const Throttling_t* pThrottling);

private:
    std::shared_ptr<PoseHistory> m_poseHistory;

    // Resource for each process
    struct ProcessResource {
        vr::SharedTextureHandle_t sharedHandles[3];
        int fds[3];
        SwapTextureSetDesc_t textDesc;
        uint32_t pid;
    };
    std::map<vr::SharedTextureHandle_t, std::pair<ProcessResource*, int>> m_handleMap;

    static const int MAX_LAYERS = 10;
    int m_submitLayer;
    SubmitLayerPerEye_t m_submitLayers[MAX_LAYERS][2];
    vr::HmdQuaternion_t m_prevFramePoseRotation;
    vr::HmdQuaternion_t m_framePoseRotation;
    uint64_t m_targetTimestampNs;
    uint64_t m_prevTargetTimestampNs;

    std::array<vr::SharedTextureHandle_t, 6> layer0Texts;

    alvr::Encoder enc;

    std::mutex m_presentMutex;
};
