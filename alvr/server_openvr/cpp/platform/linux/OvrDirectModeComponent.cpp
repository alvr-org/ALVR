#define _GNU_SOURCE
#include <link.h>
#include <stdio.h>
#include <sys/socket.h>
#include <sys/un.h>

#include <cstdlib>
#include <iostream>
#include <unistd.h>

#include "OvrDirectModeComponent.h"
#include "SteamVrSync.h"

#include "alvr_server/Logger.h"
#include "alvr_server/include/openvr_math.h"
#include <algorithm>
#include <cmath>
#include <cstring>
#include <vector>

#include <linux/dma-buf.h>
#include <poll.h>
#include <sys/ioctl.h>

// Multiples of the frame interval. A job older than this when the worker picks
// it up is from a stall, and the swap texture sets are only three deep, so the
// slot is close to its next writer.
constexpr double JobAgeLimitFrames = 1.5;

// Consecutive writer-fence poll timeouts before the GPU wait disarms for the
// session, so a permanently wedged writer degrades the stream once instead of
// taxing every frame.
constexpr uint32_t GpuWaitDisarmThreshold = 30;

// Cap on the writer-fence poll, in frame intervals. The render submission
// waits on these fences unboundedly while holding the encoder mutex, which
// Present's rebuild paths also take, so an unsignalled fence must be caught
// here or it wedges SteamVR's frame loop through the worker built to keep it
// free.
constexpr int64_t GpuWaitPollCapFrames = 2;

// Guard sleep before render, ms, only for a frame with a texture that has no
// SteamVR timeline semaphore (see SteamVrSync.h). The compositor's own scene
// writes carry only detached pre-signaled placeholder fences, and on NVIDIA
// the dma-bufs carry none at all, so the fence wait cannot see them; this
// bounds how fresh such a write can be when the read starts. Runs on the
// worker, so it costs pipeline depth, not compositor-thread budget.
constexpr int GuardSleepMs = 4;

// Cap on the wait for a texture's timeline semaphore, in frame intervals.
// On timeout the frame is read anyway, as it would have been without it.
constexpr int64_t TimelineWaitCapFrames = 2;

// The negotiated refresh rate expressed as a frame period.
static std::chrono::nanoseconds frameInterval() {
    return std::chrono::nanoseconds((int64_t)(1e9 / Settings_Instance()->m_refreshRate));
}

// Export the sync file a reader must wait on (the writer's fences) from a
// dma-buf fd. Returns the sync_file fd, or -1 when there is nothing to wait
// on or the kernel lacks the ioctl. The caller owns the returned fd.
static int exportReadSyncFile(int dmabufFd) {
#if defined(DMA_BUF_IOCTL_EXPORT_SYNC_FILE)
    struct dma_buf_export_sync_file exportInfo = {};
    exportInfo.flags = DMA_BUF_SYNC_READ;
    exportInfo.fd = -1;
    if (ioctl(dmabufFd, DMA_BUF_IOCTL_EXPORT_SYNC_FILE, &exportInfo) != 0) {
        return -1;
    }
    return exportInfo.fd;
#else
    (void)dmabufFd;
    return -1;
#endif
}

// Close whichever of a job's fence fds are open. Every path that drops a
// job without presenting it owes this call.
template <typename Waits> static void closeFenceFds(Waits& waits) {
    for (auto& wait : waits) {
        if (wait.fenceFd >= 0) {
            close(wait.fenceFd);
            wait.fenceFd = -1;
        }
    }
}

// Loose on purpose: it rejects zeroed and uninitialized poses, which miss by
// orders of magnitude, not accumulated rounding drift.
constexpr double UnitQuatNormSqTolerance = 0.1;

static bool IsUnitQuat(double w, double x, double y, double z) {
    double normSq = w * w + x * x + y * y + z * z;
    return normSq > 1.0 - UnitQuatNormSqTolerance && normSq < 1.0 + UnitQuatNormSqTolerance;
}

// Fill the warp push constants for a rotational reprojection from
// renderOrientation to latestOrientation. The shaders sample backwards, so the
// rotation they need is source-from-destination, R_render^T * R_latest; with
// the Hamilton product convention of openvr_math.h, conj(q_render) * q_latest
// composes the same way. Both quaternions are world-from-head, so the world
// frame cancels and the playspace transform never enters. Returns false, with
// params left disabled, on a degenerate FOV or a non-unit orientation.
static bool BuildWarpParams(
    const vr::HmdQuaternion_t& renderOrientation,
    const FfiQuat& latestOrientation,
    const FfiFov& leftFov,
    const FfiFov& rightFov,
    alvr::render::WarpParams& params
) {
    float leftTans[4]
        = { tanf(leftFov.left), tanf(leftFov.right), tanf(leftFov.up), tanf(leftFov.down) };
    float rightTans[4]
        = { tanf(rightFov.left), tanf(rightFov.right), tanf(rightFov.up), tanf(rightFov.down) };
    if (leftTans[1] - leftTans[0] == 0.f || rightTans[1] - rightTans[0] == 0.f
        || leftTans[2] - leftTans[3] == 0.f || rightTans[2] - rightTans[3] == 0.f) {
        return false;
    }
    if (!IsUnitQuat(
            renderOrientation.w, renderOrientation.x, renderOrientation.y, renderOrientation.z
        )
        || !IsUnitQuat(
            latestOrientation.w, latestOrientation.x, latestOrientation.y, latestOrientation.z
        )) {
        return false;
    }

    vr::HmdQuaternion_t qLatest
        = { latestOrientation.w, latestOrientation.x, latestOrientation.y, latestOrientation.z };
    vr::HmdQuaternion_t qDelta = vrmath::quaternionConjugate(renderOrientation) * qLatest;

    vr::HmdMatrix34_t rot;
    HmdMatrix_QuatToMat(qDelta.w, qDelta.x, qDelta.y, qDelta.z, &rot);

    // GLSL mat4 is column-major: rotation[col * 4 + row].
    for (int row = 0; row < 4; ++row) {
        for (int col = 0; col < 4; ++col) {
            params.rotation[col * 4 + row]
                = (row < 3 && col < 3) ? rot.m[row][col] : (float)(row == col);
        }
    }
    memcpy(params.leftTans, leftTans, sizeof(leftTans));
    memcpy(params.rightTans, rightTans, sizeof(rightTans));
    params.enabled = 1;
    return true;
}

void OvrDirectModeComponent::SetViewParams(const FfiViewParams params[2]) {
    std::unique_lock<std::mutex> lock(m_viewParamsMutex);
    m_viewParams[0] = params[0];
    m_viewParams[1] = params[1];
    m_viewParamsValid = true;
}

OvrDirectModeComponent::OvrDirectModeComponent(std::shared_ptr<PoseHistory> poseHistory)
    : m_poseHistory(poseHistory)
    , m_submitLayer(0) {
    m_encodeWorker = std::thread(&OvrDirectModeComponent::EncodeWorkerLoop, this);
    StartVsyncAnnouncer();
}

OvrDirectModeComponent::~OvrDirectModeComponent() {
    // Stops before the encode worker: the announcer's grid is what
    // PostPresent holds against, and a hold outliving the worker leaves a
    // present waiting on a frame nothing will encode.
    StopVsyncAnnouncer();
    {
        std::lock_guard<std::mutex> lock(m_jobMutex);
        m_workerExit = true;
        m_jobPending = false;
    }
    m_jobCv.notify_one();
    if (m_encodeWorker.joinable()) {
        m_encodeWorker.join();
    }
    // The worker is gone; whatever job was pending will never present.
    closeFenceFds(m_job.waits);
}

void OvrDirectModeComponent::DrainPendingJob() {
    uint64_t dropped = 0;
    {
        std::lock_guard<std::mutex> lock(m_jobMutex);
        if (m_jobPending) {
            closeFenceFds(m_job.waits);
            dropped = m_job.frame;
        }
        m_jobPending = false;
    }
    if (dropped != 0) {
        MarkRead(dropped);
    }
}

void OvrDirectModeComponent::MarkRead(uint64_t frame) {
    {
        std::lock_guard<std::mutex> lock(m_readMutex);
        m_lastReadFrame = std::max(m_lastReadFrame, frame);
    }
    m_readCv.notify_all();
}

void OvrDirectModeComponent::EncodeWorkerLoop() {
    while (true) {
        FrameJob job;
        {
            std::unique_lock<std::mutex> lock(m_jobMutex);
            m_jobCv.wait(lock, [this] { return m_jobPending || m_workerExit; });
            if (m_workerExit) {
                break;
            }
            job = m_job;
            m_jobPending = false;
        }

        auto const ageLimit = std::chrono::duration_cast<std::chrono::nanoseconds>(
            frameInterval() * JobAgeLimitFrames
        );
        if (std::chrono::steady_clock::now() - job.enqueueTime > ageLimit) {
            closeFenceFds(job.waits);
            MarkRead(job.frame);
            continue;
        }

        // Fallback for textures without a SteamVR semaphore: bounded gate on
        // the writer's implicit fences, before the encoder lock. On timeout
        // close the fds and present unguarded instead of wedging. Devices: on
        // Mesa (AMD, Intel) these fences carry the writer's work; on NVIDIA
        // the dma-bufs carry none and nothing is exported.
        bool needsGuard = false;
        std::vector<struct pollfd> pfds;
        for (auto const& wait : job.waits) {
            needsGuard = needsGuard || wait.semaphore == 0;
            if (wait.fenceFd >= 0) {
                pfds.push_back({ .fd = wait.fenceFd, .events = POLLIN, .revents = 0 });
            }
        }
        if (!pfds.empty() && !m_gpuWaitPollDisarmed) {
            auto const capNs = frameInterval() * GpuWaitPollCapFrames;
            auto const deadline = std::chrono::steady_clock::now() + capNs;
            bool allSignaled = false;
            while (!allSignaled) {
                int remainingMs = (int)std::chrono::duration_cast<std::chrono::milliseconds>(
                                      deadline - std::chrono::steady_clock::now()
                )
                                      .count();
                if (remainingMs < 0) {
                    break;
                }
                if (poll(pfds.data(), pfds.size(), remainingMs) <= 0) {
                    break;
                }
                allSignaled = true;
                for (auto const& pfd : pfds) {
                    if ((pfd.revents & POLLIN) == 0) {
                        allSignaled = false;
                    }
                }
            }
            if (allSignaled) {
                m_gpuWaitConsecTimeouts = 0;
            } else if (++m_gpuWaitConsecTimeouts >= GpuWaitDisarmThreshold) {
                m_gpuWaitPollDisarmed = true;
                Info(
                    "Encode worker: writer fences timed out %u frames in a row; "
                    "GPU wait disarmed for this session\n",
                    m_gpuWaitConsecTimeouts
                );
            }
        }
        closeFenceFds(job.waits);

        if (needsGuard && GuardSleepMs > 0) {
            std::this_thread::sleep_for(std::chrono::milliseconds(GuardSleepMs));
        }

        try {
            std::lock_guard<std::mutex> encLock(m_encMutex);

            if (job.generation != m_encGeneration) {
                MarkRead(job.frame);
                continue;
            }

            // Wait for the exact end of SteamVR's write into each texture, as
            // Steam Link does (SteamVrSync.h). Most textures are already
            // finished here; on NVIDIA the waits measured reach ~4 ms in
            // SteamVR Home and ~6 ms when a heavy application's frame ends
            // late.
            auto const timelineCapNs
                = (uint64_t)std::chrono::duration_cast<std::chrono::nanoseconds>(
                      frameInterval() * TimelineWaitCapFrames
                )
                      .count();
            for (auto const& wait : job.waits) {
                if (wait.semaphore == 0) {
                    continue;
                }
                uint64_t const semaphore = wait.semaphore;
                if (!enc.waitTimeline(
                        semaphore,
                        [semaphore] { return steamvrsync::Sync::Get().DupFd(semaphore); },
                        wait.value,
                        timelineCapNs
                    )) {
                    Warn(
                        "SteamVR texture semaphore %llx not reached or not importable; "
                        "reading the frame anyway\n",
                        (unsigned long long)semaphore
                    );
                }
            }

            // Rotationally reproject the frame from the pose it was rendered
            // with to the newest tracking pose, then stamp it with that pose's
            // timestamp: after the warp the newest pose is the image's pose,
            // and the client reprojects from whatever the stamp names. Moving
            // one without the other is what breaks world lock. Sampled here
            // rather than in Present so the target is as fresh as possible.
            uint64_t stampTimestampNs = job.targetTimestampNs;
            alvr::render::WarpParams warpParams {};
            if (enc.warpCapable()) {
                auto latest = m_poseHistory->GetLatestPose();

                FfiViewParams viewParams[2];
                bool viewParamsValid;
                {
                    std::unique_lock<std::mutex> vpLock(m_viewParamsMutex);
                    viewParams[0] = m_viewParams[0];
                    viewParams[1] = m_viewParams[1];
                    viewParamsValid = m_viewParamsValid;
                }

                if (latest && viewParamsValid
                    && BuildWarpParams(
                        job.renderOrientation,
                        latest->motion.pose.orientation,
                        viewParams[0].fov,
                        viewParams[1].fov,
                        warpParams
                    )) {
                    stampTimestampNs = latest->targetTimestampNs;
                }
            }

            uint64_t const frame = job.frame;
            enc.present(stampTimestampNs, warpParams, job.layers, [this, frame] {
                MarkRead(frame);
            });
            MarkRead(frame);
        } catch (std::exception const& e) {
            // An exception escaping a thread's start function is
            // std::terminate, which takes vrserver with it. Same recovery as
            // the compositor-thread paths: log, tear the encoder down, go
            // idle until the next client connect.
            Error("Encode worker: %s. Tearing down encoder and going idle.\n", e.what());
            MarkRead(job.frame);
            std::lock_guard<std::mutex> encLock(m_encMutex);
            // The try block's lock released when the exception left it, so a
            // rebuild may have swapped the encoder in between. A moved
            // generation means this failure belongs to an encoder that no
            // longer exists; leave the new one alone.
            if (job.generation != m_encGeneration) {
                continue;
            }
            m_encGeneration++;
            try {
                enc.shutdown();
            } catch (std::exception const& e2) {
                // Teardown of already-broken state must not become the
                // std::terminate this handler exists to prevent.
                Error("Encode worker: teardown also failed: %s\n", e2.what());
            }
            m_encoderState = EncoderState::Idle;
        }
    }
}

void OvrDirectModeComponent::CreateSwapTextureSet(
    uint32_t unPid,
    const SwapTextureSetDesc_t* pSwapTextureSetDesc,
    SwapTextureSet_t* pOutSwapTextureSet
) {
    Info(
        "CreateSwapTextureSet pid=%d Format=%d %dx%d SampleCount=%d\n",
        unPid,
        pSwapTextureSetDesc->nFormat,
        pSwapTextureSetDesc->nWidth,
        pSwapTextureSetDesc->nHeight,
        pSwapTextureSetDesc->nSampleCount
    );

    ProcessResource* processResource = new ProcessResource();
    processResource->textDesc = *pSwapTextureSetDesc;
    processResource->pid = unPid;
    {
        char path[32];
        snprintf(path, sizeof(path), "/proc/%u/comm", unPid);
        if (FILE* comm = fopen(path, "r")) {
            char name[32] = {};
            if (fgets(name, sizeof(name), comm)) {
                processResource->compositorOwned = strncmp(name, "vrcompositor", 12) == 0;
            }
            fclose(comm);
        }
    }
    m_swapchainIndices[processResource] = 0;

    {
        auto pid = getpid();
        Info("VrServer PID %d\n", pid);
    }

    uint32_t usageFlags = static_cast<uint32_t>(
        vk::ImageUsageFlagBits::eTransferSrc | vk::ImageUsageFlagBits::eSampled
        | vk::ImageUsageFlagBits::eInputAttachment
    );

    uint32_t creationFlags = static_cast<uint32_t>(vk::ImageCreateFlagBits::eMutableFormat);

    for (int i = 0; i < 3; i++) {
        vr::SharedTextureHandle_t myHandle = 0;
        bool success = vr::VRIPCResourceManager()->NewSharedVulkanImage(
            pSwapTextureSetDesc->nFormat,
            pSwapTextureSetDesc->nWidth,
            pSwapTextureSetDesc->nHeight,
            true,
            false,
            true,
            1,
            1,
            creationFlags,
            usageFlags,
            &myHandle
        );

        uint64_t ipcHandle = 0;
        vr::VRIPCResourceManager()->RefResource(myHandle, &ipcHandle);

        if (!success) {
            Error("VRCIPCResourceManager: Failed to create shared texture\n");
            CleanupProcessResource(processResource);
            return;
        }

        int fd = 0;
        auto ret = vr::VRIPCResourceManager()->ReceiveSharedFd(ipcHandle, &fd);
        if (ret == false) {
            Error("Failed to get fd for texture\n");
            vr::VRIPCResourceManager()->UnrefResource(myHandle);
            CleanupProcessResource(processResource);
            return;
        }

        processResource->fds[i] = fd;
        processResource->sharedHandles[i] = myHandle;

        m_handleMap.insert(
            std::make_pair(processResource->sharedHandles[i], std::make_pair(processResource, i))
        );
        pOutSwapTextureSet->rSharedTextureHandles[i] = myHandle;
        Info("Created Texture %d %p\n", i, processResource->sharedHandles[i]);
    }
}

/** Used to textures created using CreateSwapTextureSet.  Only one of the set's handles needs to be
 * used to destroy the entire set. */
void OvrDirectModeComponent::DestroySwapTextureSet(vr::SharedTextureHandle_t sharedTextureHandle) {
    Info("DestroySwapTextureSet %p\n", sharedTextureHandle);

    m_presentMutex.lock();
    auto id = m_handleMap.find(sharedTextureHandle);
    if (id != m_handleMap.end()) {
        ProcessResource* p = id->second.first;
        ForgetTextures({ p->sharedHandles[0], p->sharedHandles[1], p->sharedHandles[2] });
        CleanupProcessResource(p);
    } else {
        Debug("Requested to destroy not managing texture. handle:%p\n", sharedTextureHandle);
    }
    m_presentMutex.unlock();
}

/** Used to purge all texture sets for a given process. */
void OvrDirectModeComponent::DestroyAllSwapTextureSets(uint32_t unPid) {
    Info("DestroyAllSwapTextureSets pid=%d\n", unPid);

    m_presentMutex.lock();
    std::vector<ProcessResource*> resourcesToDestroy;
    for (auto it = m_handleMap.begin(); it != m_handleMap.end(); ++it) {
        if (it->second.first->pid == unPid && it->second.second == 0) {
            resourcesToDestroy.push_back(it->second.first);
        }
    }
    std::vector<vr::SharedTextureHandle_t> textures;
    for (auto* p : resourcesToDestroy) {
        textures.insert(textures.end(), p->sharedHandles, p->sharedHandles + 3);
    }
    if (!textures.empty()) {
        ForgetTextures(textures);
    }
    for (auto* p : resourcesToDestroy) {
        CleanupProcessResource(p);
    }
    m_presentMutex.unlock();
}

void OvrDirectModeComponent::ForgetTextures(std::vector<vr::SharedTextureHandle_t> const& textures
) {
    // A pending or in-flight job may still name these textures. Drop the
    // pending one, then take the encoder lock (waits for a frame being read)
    // and move the generation so a job the worker already holds is dropped
    // instead of importing an fd that is about to be closed.
    {
        std::lock_guard<std::mutex> lock(m_destroyedMutex);
        for (auto texture : textures) {
            m_destroyed.push_back(texture);
        }
        while (m_destroyed.size() > MaxDestroyedRemembered) {
            m_destroyed.pop_front();
        }
    }
    DrainPendingJob();
    std::vector<uint64_t> semaphores = steamvrsync::Sync::Get().Forget(textures);
    std::lock_guard<std::mutex> encLock(m_encMutex);
    m_encGeneration++;
    enc.releaseTextures(textures, semaphores);
}

bool OvrDirectModeComponent::WasDestroyed(vr::SharedTextureHandle_t texture) {
    std::lock_guard<std::mutex> lock(m_destroyedMutex);
    return std::find(m_destroyed.begin(), m_destroyed.end(), texture) != m_destroyed.end();
}

void OvrDirectModeComponent::CleanupProcessResource(ProcessResource* processResource) {
    for (int i = 0; i < 3; i++) {
        if (processResource->sharedHandles[i]) {
            vr::VRIPCResourceManager()->UnrefResource(processResource->sharedHandles[i]);
            m_handleMap.erase(processResource->sharedHandles[i]);
            processResource->sharedHandles[i] = 0;
        }
        if (processResource->fds[i] >= 0) {
            close(processResource->fds[i]);
            processResource->fds[i] = -1;
        }
    }
    m_swapchainIndices.erase(processResource);
    delete processResource;
}

/** After Present returns, calls this to get the next index to use for rendering. */
void OvrDirectModeComponent::GetNextSwapTextureSetIndex(
    vr::SharedTextureHandle_t sharedTextureHandles[2], uint32_t (*pIndices)[2]
) {
    Debug("OvrDirectModeComponent::GetNextSwapTextureSetIndex");

    m_presentMutex.lock();
    for (int eye = 0; eye < 2; eye++) {
        auto it = m_handleMap.find(sharedTextureHandles[eye]);
        if (it == m_handleMap.end()) {
            continue;
        }
        auto& idx = m_swapchainIndices[it->second.first];
        idx = (idx + 1) % 3;
        if (pIndices) {
            (*pIndices)[eye] = idx;
        }
    }
    m_presentMutex.unlock();
}

/** Call once per layer to draw for this frame.  One shared texture handle per eye.  Textures must
 * be created using CreateSwapTextureSet and should be alternated per frame.  Call Present once all
 * layers have been submitted. */
void OvrDirectModeComponent::SubmitLayer(const SubmitLayerPerEye_t (&perEye)[2]) {
    m_presentMutex.lock();

    auto pPose = &perEye[0].mHmdPose; // TODO: are both poses the same? Name HMD suggests yes.

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
        Warn("Too many layers submitted!\n");
    }

    m_presentMutex.unlock();
}

/** Submits queued layers for display. */
void OvrDirectModeComponent::Present(vr::SharedTextureHandle_t syncTexture) {
    int const layerCount = std::min(m_submitLayer, MAX_LAYERS);
    m_submitLayer = 0;
    // Set below when this Present queues a frame (read by PostPresent).
    m_holdUntilRead = false;
    m_queuedThisPresent = false;

    switch (m_encoderState.load()) {
    case EncoderState::RebuildRequested: {
        Info("Rebuilding encoder from negotiated settings\n");
        DrainPendingJob();
        std::lock_guard<std::mutex> encLock(m_encMutex);
        m_encGeneration++;
        enc.shutdown();
        m_encoderReady = false;
        m_encoderState = EncoderState::Streaming;
        break;
    }
    case EncoderState::ShutdownRequested: {
        DrainPendingJob();
        std::lock_guard<std::mutex> encLock(m_encMutex);
        m_encGeneration++;
        enc.shutdown();
        m_encoderReady = false;
        m_encoderState = EncoderState::Idle;
        return;
    }
    // Before a client connects there are no negotiated settings, so building
    // an encoder here would use wrong defaults and fail noisily. Do nothing
    // until streaming starts.
    case EncoderState::Idle:
        return;
    case EncoderState::Streaming:
        break;
    }

    std::optional<alvr::render::Layer> layer0 = SubmittedLayer(0);
    if (!layer0) {
        // When an application exits, SteamVR destroys its texture sets and
        // the compositor still presents one last frame naming them. That
        // frame is skipped quietly; textures never seen are still an error.
        if (WasDestroyed(m_submitLayers[0][0].hTexture)
            || WasDestroyed(m_submitLayers[0][1].hTexture)) {
            Debug(
                "Skipping a frame whose textures SteamVR just destroyed %llu, %llu\n",
                m_submitLayers[0][0].hTexture,
                m_submitLayers[0][1].hTexture
            );
            return;
        }
        Error(
            "Textures not found in handle map %llu, %llu\n",
            m_submitLayers[0][0].hTexture,
            m_submitLayers[0][1].hTexture
        );
        return;
    }

    if (!m_encoderReady) {
        auto const& settings = Settings_Instance();
        alvr::render::RendererCreateInfo rendererCI {
            .format = layer0->format,
            .inputEyeExtent = layer0->extent,
            .outputExtent {
                .width = settings->m_recommendedTargetWidth,
                .height = settings->m_recommendedTargetHeight,
            },
        };

        // Renderer and encoder setup can throw (Vulkan, VAAPI). An exception
        // escaping Present kills vrserver, so log and stay idle.
        DrainPendingJob();
        std::lock_guard<std::mutex> encLock(m_encMutex);
        m_encGeneration++;

        try {
            enc.createImages(rendererCI);
            enc.initEncoding();
            m_encoderReady = true;
        } catch (std::exception const& e) {
            Error(
                "Could not set up the encoder: %s. Staying idle until the next client connect.\n",
                e.what()
            );
            enc.shutdown();
            m_encoderState = EncoderState::Idle;
        }

        // We'll get em next time
        return;
    }

    FrameJob job {};
    job.frame = ++m_presentFrame;
    job.layers.push_back(*layer0);
    // Layers 1..n (SteamVR's dashboard and other overlays while a scene
    // application runs) are blended over layer 0. A layer whose textures are
    // not ours is skipped rather than dropping the frame.
    for (int i = 1; i < layerCount; ++i) {
        if (auto layer = SubmittedLayer(i)) {
            job.layers.push_back(*layer);
        }
    }
    job.targetTimestampNs = m_targetTimestampNs;
    job.renderOrientation = m_framePoseRotation;
    job.generation = m_encGeneration;
    job.enqueueTime = std::chrono::steady_clock::now();

    // What marks the end of each texture's write. SteamVR's timeline
    // semaphore and its value, read now (by the worker SteamVR may already
    // have submitted the next write into the same texture); for a texture
    // without one, the dma-buf's implicit fences as before.
    auto& sync = steamvrsync::Sync::Get();
    for (auto const& layer : job.layers) {
        for (auto const& eye : layer.eyes) {
            FrameJob::TextureWait wait;
            wait.semaphore = sync.SemaphoreOf(eye.handle);
            if (wait.semaphore == 0 || !sync.Value(wait.semaphore, &wait.value)) {
                wait.semaphore = 0;
                wait.fenceFd = exportReadSyncFile(eye.fd);
                if (m_noSemaphoreLogged.insert(eye.handle).second) {
                    Info(
                        "SteamVR texture %llx has no timeline semaphore; using its implicit "
                        "fences and a %d ms guard\n",
                        (unsigned long long)eye.handle,
                        GuardSleepMs
                    );
                }
            }
            job.waits.push_back(wait);

            // vrcompositor's own textures: the value read here does not always
            // cover the right eye's write yet, and without a hold SteamVR goes
            // on and that write lands during the read (the right eye showed
            // the next frame). Scene applications' textures need no hold, and
            // holding for them would pass their late GPU frames on to the
            // compositor (VRChat fell to 60 fps).
            auto it = m_handleMap.find(eye.handle);
            if (it != m_handleMap.end() && it->second.first->compositorOwned) {
                m_holdUntilRead = true;
            }
        }
    }

    {
        std::lock_guard<std::mutex> lock(m_jobMutex);
        if (m_workerExit) {
            // Teardown already drained the mailbox; do not repopulate it.
            closeFenceFds(job.waits);
            return;
        }
        // Latest wins: a still-pending job is replaced, and its fences will
        // never be waited on, so close them here.
        if (m_jobPending) {
            closeFenceFds(m_job.waits);
        }
        m_job = job;
        m_jobPending = true;
    }
    {
        std::lock_guard<std::mutex> lock(m_readMutex);
        m_lastQueuedFrame = job.frame;
    }
    m_queuedThisPresent = true;
    m_jobCv.notify_one();
}

std::optional<alvr::render::Layer> OvrDirectModeComponent::SubmittedLayer(int i) {
    alvr::render::Layer layer;
    for (int eye = 0; eye < 2; ++eye) {
        auto const& submitted = m_submitLayers[i][eye];
        auto it = m_handleMap.find(submitted.hTexture);
        if (it == m_handleMap.end()) {
            return std::nullopt;
        }
        auto const& b = submitted.bounds;
        if (!std::isfinite(b.uMin) || !std::isfinite(b.uMax) || !std::isfinite(b.vMin)
            || !std::isfinite(b.vMax) || b.uMin < 0.f || b.uMin > 1.f || b.uMax < 0.f
            || b.uMax > 1.f || b.vMin < 0.f || b.vMin > 1.f || b.vMax < 0.f || b.vMax > 1.f
            || b.uMin == b.uMax || b.vMin == b.vMax) {
            Warn("Invalid texture bounds for layer %d eye %d; skipping it\n", i, eye);
            return std::nullopt;
        }
        auto const* resource = it->second.first;
        layer.format = (vk::Format)(VkFormat)resource->textDesc.nFormat;
        layer.extent = vk::Extent2D { .width = resource->textDesc.nWidth,
                                      .height = resource->textDesc.nHeight };
        layer.eyes[eye] = {
            .handle = submitted.hTexture,
            .fd = resource->fds[it->second.second],
            .uMin = b.uMin,
            .vMin = b.vMin,
            .uMax = b.uMax,
            .vMax = b.vMax,
        };
    }
    return layer;
}

void OvrDirectModeComponent::PostPresent(const Throttling_t* pThrottling) {
    // Prop_Hmd_SupportsAppThrottling_Bool is set, so SteamVR sends throttle
    // hints here and predicts poses assuming we honor them. Ignoring them
    // while it throttles an app predicts poses for a cadence we are not
    // keeping, which reprojects the world against head motion.
    uint32_t throttleFrames = 0;
    if (pThrottling != nullptr) {
        throttleFrames = pThrottling->nFramesToThrottle;
        if (pThrottling->nFramesToThrottle != m_lastThrottleFrames
            || pThrottling->nAdditionalFramesToPredict != m_lastPredictFrames) {
            m_lastThrottleFrames = pThrottling->nFramesToThrottle;
            m_lastPredictFrames = pThrottling->nAdditionalFramesToPredict;
            Info(
                "Throttling: framesToThrottle=%u additionalFramesToPredict=%u\n",
                pThrottling->nFramesToThrottle,
                pThrottling->nAdditionalFramesToPredict
            );
        }
    }

    // Upstream's pacing against the vsync grid stays while nothing is being
    // streamed (before a client connects, while the encoder is set up) and
    // while SteamVR throttles an application, which is what honours the hint.
    // Every test of this change ran that way, and the one run without any
    // pacing from SteamVR's start hit an NVIDIA Xid 32 in vrcompositor before
    // the client connected. While frames are streamed it does not hold every
    // frame until the next tick: that is almost a whole frame, after which
    // vrcompositor's WaitForAcquire times out and it draws into the texture
    // being read (SteamVR Home shook). The vsync announcer alone kept the
    // compositor at the refresh rate then (72.0 fps measured). A throttled
    // application is untested with this change.
    if (throttleFrames > 0 || !m_queuedThisPresent) {
        PaceAfterPresent(throttleFrames);
        return;
    }

    // Hold SteamVR's submit thread until the worker has read this frame, but
    // only for frames that read vrcompositor's own textures (see Present),
    // and at most one frame interval. Devices: Linux only.
    if (m_holdUntilRead) {
        std::unique_lock<std::mutex> lock(m_readMutex);
        uint64_t const target = m_lastQueuedFrame;
        m_readCv.wait_for(lock, frameInterval(), [&] { return m_lastReadFrame >= target; });
    }
}

void OvrDirectModeComponent::GetFrameTiming(vr::DriverDirectMode_FrameTiming* pFrameTiming) {
    if (pFrameTiming == nullptr
        || pFrameTiming->m_nSize < sizeof(vr::DriverDirectMode_FrameTiming)) {
        return;
    }

    // Skipped ticks are not reported as dropped frames: the compositor reads a
    // drop as GPU overload and enables interleaved reprojection, which halves
    // the rate, so the grid then skips a tick every frame and reports another
    // drop, throttled from then on. Consume the counter without reporting it
    // until there is a channel that does not throttle.
    (void)TakeSkippedVsyncs();
    pFrameTiming->m_nNumFramePresents = 1;
    pFrameTiming->m_nNumMisPresented = 0;
    pFrameTiming->m_nNumDroppedFrames = 0;
    // These bits overlap the throttle mask and must not be echoed back
    // unhandled, per the interface header.
    pFrameTiming->m_nReprojectionFlags = 0;
}
