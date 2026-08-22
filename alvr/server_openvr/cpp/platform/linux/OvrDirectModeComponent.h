#pragma once
#include "alvr_server/PoseHistory.h"
#include "alvr_server/Utils.h"
#include "alvr_server/bindings.h"
#include "openvr_driver.h"

#include "alvr_server/Settings.h"

#include <map>
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <mutex>
#include <thread>

#include <vulkan/vulkan.h>

#include "Encoder.hpp"

class OvrDirectModeComponent : public vr::IVRDriverDirectModeComponent {
public:
    OvrDirectModeComponent(
        /* std::shared_ptr<Renderer> pVKRender,  */ std::shared_ptr<PoseHistory> poseHistory
    );
    ~OvrDirectModeComponent();

    void RequestIdr() { enc.requestIdr(); }

    // Called from the event loop thread when negotiated settings arrive.
    // The state change itself is applied on the compositor thread inside
    // Present, which then builds the encoder from the new settings.
    void RequestEncoderReset() { m_encoderState = EncoderState::RebuildRequested; }

    // Called from the event loop thread when the client disconnects. Present
    // tears the encoder down and goes idle until the next connect.
    void RequestEncoderShutdown() { m_encoderState = EncoderState::ShutdownRequested; }

    // Called on the event loop thread whenever the client's projection
    // changes. Supplies the per-eye FOV the reprojection needs.
    void SetViewParams(const FfiViewParams params[2]);

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
    void CleanupProcessResource(ProcessResource* processResource);

    static const int MAX_LAYERS = 10;
    int m_submitLayer;
    SubmitLayerPerEye_t m_submitLayers[MAX_LAYERS][2];
    vr::HmdQuaternion_t m_prevFramePoseRotation;
    vr::HmdQuaternion_t m_framePoseRotation;
    uint64_t m_targetTimestampNs;
    uint64_t m_prevTargetTimestampNs;
    // Track current texture index for each eye to avoid UB with uninitialized data
    std::map<ProcessResource*, uint32_t> m_swapchainIndices;

    std::array<vr::SharedTextureHandle_t, 6> layer0Texts {};

    alvr::Encoder enc;

    // Written by the event loop thread, applied and advanced by the
    // compositor thread at the top of Present.
    enum class EncoderState { Idle, RebuildRequested, Streaming, ShutdownRequested };
    std::atomic<EncoderState> m_encoderState { EncoderState::Idle };

    std::mutex m_presentMutex;

    // Single-slot latest-wins mailbox: at steady state the worker drains
    // faster than frames arrive, and if it falls behind, the newest frame
    // replaces the stale pending one.
    struct FrameJob {
        uint32_t leftIdx;
        uint32_t rightIdx;
        uint64_t targetTimestampNs;
        // Orientation the layer was rendered with, captured in Present. The
        // worker needs it to build the reprojection, and it cannot read
        // m_framePoseRotation itself because the compositor thread has
        // overwritten it by then.
        vr::HmdQuaternion_t renderOrientation;
        // Encoder-state generation this job was built against. The worker
        // discards the job if the generation moved (rebuild, shutdown, set
        // switch) between enqueue and processing.
        uint64_t generation;
        std::chrono::steady_clock::time_point enqueueTime;
    };
    void EncodeWorkerLoop();
    // Drop a pending job. Call before any path that touches the encoder from
    // the compositor thread.
    void DrainPendingJob();

    std::thread m_encodeWorker;
    std::mutex m_jobMutex;
    std::condition_variable m_jobCv;
    FrameJob m_job {};
    bool m_jobPending = false;
    bool m_workerExit = false;
    // Serializes encoder access: the worker's per-frame enc.present against
    // the compositor thread's rebuild and shutdown paths. Never held together
    // with m_jobMutex.
    std::mutex m_encMutex;
    // Incremented under m_encMutex by every compositor-thread path that
    // mutates encoder state. A job whose generation is stale gets dropped
    // instead of presenting old indices against rebuilt state.
    uint64_t m_encGeneration = 0;

    // Written by the event loop thread in SetViewParams, read by the worker.
    // Invalid until the client's projection arrives, and the reprojection
    // stays off until then because it has no FOV to build a ray from.
    std::mutex m_viewParamsMutex;
    FfiViewParams m_viewParams[2] {};
    bool m_viewParamsValid = false;
};
