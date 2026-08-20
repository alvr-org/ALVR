#define _GNU_SOURCE
#include <link.h>
#include <stdio.h>
#include <sys/socket.h>
#include <sys/un.h>

#include <cstdlib>
#include <iostream>
#include <unistd.h>

#include "OvrDirectModeComponent.h"

#include "alvr_server/Logger.h"
#include "alvr_server/include/openvr_math.h"
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

// The declared vsync grid leads the real frame deadline by this much. Sleeping
// to the tick and declaring a zero offset advertises no headroom, which
// measured as heavy applications locked to half rate; 2 ms measured as a full
// 72.
constexpr auto RunningStart = std::chrono::microseconds(2000);

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

// Guard sleep before render, ms. The compositor's own scene writes carry only
// detached pre-signaled placeholder fences, so the GPU-side fence wait cannot
// see them; this bounds how fresh such a write can be when the read starts.
// Runs on the worker, so it costs pipeline depth, not compositor-thread
// budget.
constexpr int GuardSleepMs = 4;

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
static void closeWaitFds(int fds[2]) {
    for (int e = 0; e < 2; ++e) {
        if (fds[e] >= 0) {
            close(fds[e]);
            fds[e] = -1;
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
}

OvrDirectModeComponent::~OvrDirectModeComponent() {
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
    closeWaitFds(m_job.waitFds);
    for (int fd : m_syncFds) {
        if (fd >= 0) {
            close(fd);
        }
    }
}

void OvrDirectModeComponent::DrainPendingJob() {
    std::lock_guard<std::mutex> lock(m_jobMutex);
    if (m_jobPending) {
        closeWaitFds(m_job.waitFds);
    }
    m_jobPending = false;
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
            closeWaitFds(job.waitFds);
            continue;
        }

        // Bounded gate on the writer's fences, before the encoder lock. On
        // timeout close the fds and present unguarded instead of wedging.
        if (job.waitFds[0] >= 0 || job.waitFds[1] >= 0) {
            bool dropFds = m_gpuWaitPollDisarmed;
            if (!dropFds) {
                struct pollfd pfds[2] = {};
                nfds_t count = 0;
                for (int e = 0; e < 2; ++e) {
                    if (job.waitFds[e] >= 0) {
                        pfds[count].fd = job.waitFds[e];
                        pfds[count].events = POLLIN;
                        ++count;
                    }
                }
                auto const capNs = frameInterval() * GpuWaitPollCapFrames;
                auto const deadline = std::chrono::steady_clock::now() + capNs;
                bool allSignaled = false;
                while (!allSignaled) {
                    int remainingMs
                        = (int)std::chrono::duration_cast<std::chrono::milliseconds>(
                              deadline - std::chrono::steady_clock::now()
                        )
                              .count();
                    if (remainingMs < 0) {
                        break;
                    }
                    if (poll(pfds, count, remainingMs) <= 0) {
                        break;
                    }
                    allSignaled = true;
                    for (nfds_t i = 0; i < count; ++i) {
                        if ((pfds[i].revents & POLLIN) == 0) {
                            allSignaled = false;
                        }
                    }
                }
                if (allSignaled) {
                    m_gpuWaitConsecTimeouts = 0;
                } else {
                    dropFds = true;
                    if (++m_gpuWaitConsecTimeouts >= GpuWaitDisarmThreshold) {
                        m_gpuWaitPollDisarmed = true;
                        Info("Encode worker: writer fences timed out %u frames in a row; "
                             "GPU wait disarmed for this session\n",
                             m_gpuWaitConsecTimeouts);
                    }
                }
            }
            if (dropFds) {
                closeWaitFds(job.waitFds);
            }
        }

        if (GuardSleepMs > 0) {
            std::this_thread::sleep_for(std::chrono::milliseconds(GuardSleepMs));
        }

        try {
            std::lock_guard<std::mutex> encLock(m_encMutex);

            if (job.generation != m_encGeneration) {
                closeWaitFds(job.waitFds);
                continue;
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

            enc.present(job.leftIdx, job.rightIdx, stampTimestampNs, warpParams, job.waitFds);
        } catch (std::exception const& e) {
            // An exception escaping a thread's start function is
            // std::terminate, which takes vrserver with it. Same recovery as
            // the compositor-thread paths: log, tear the encoder down, go
            // idle until the next client connect.
            // The job's fds may be partially consumed by the throw point, so
            // they are deliberately not closed here.
            Error("Encode worker: %s. Tearing down encoder and going idle.\n", e.what());
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
    m_swapchainIndices[processResource] = 0;

    {
        auto pid = getpid();
        Info("VrServer PID %d\n", pid);
    }

    uint32_t usageFlags = static_cast<uint32_t>(
        vk::ImageUsageFlagBits::eTransferSrc | 
        vk::ImageUsageFlagBits::eSampled | 
        vk::ImageUsageFlagBits::eInputAttachment
    );

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
            0, // Change creation flags if changed in renderer. Otherwise, the image may not be usable in the renderer.
            usageFlags, // Change usage flags if changed in renderer. Otherwise, the image may not be usable in the renderer.
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
    for (auto* p : resourcesToDestroy) {
        CleanupProcessResource(p);
    }
    m_presentMutex.unlock();
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
    m_submitLayer = 0;

    switch (m_encoderState.load()) {
    case EncoderState::RebuildRequested: {
        Info("Rebuilding encoder from negotiated settings\n");
        DrainPendingJob();
        std::lock_guard<std::mutex> encLock(m_encMutex);
        m_encGeneration++;
        enc.shutdown();
        layer0Texts.fill(0);
        m_encoderState = EncoderState::Streaming;
        break;
    }
    case EncoderState::ShutdownRequested: {
        DrainPendingJob();
        std::lock_guard<std::mutex> encLock(m_encMutex);
        m_encGeneration++;
        enc.shutdown();
        layer0Texts.fill(0);
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

    std::optional<u32> leftIdx;
    std::optional<u32> rightIdx;

    for (u32 i = 0; i < layer0Texts.size(); ++i) {
        if (layer0Texts[i] == m_submitLayers[0][0].hTexture)
            leftIdx = i;

        if (layer0Texts[i] == m_submitLayers[0][1].hTexture)
            rightIdx = i;
    }

    if (!leftIdx.has_value() || !rightIdx.has_value()) {
        auto leftIt = m_handleMap.find(m_submitLayers[0][0].hTexture);
        auto rightIt = m_handleMap.find(m_submitLayers[0][1].hTexture);

        if (leftIt == m_handleMap.end() || rightIt == m_handleMap.end()) {
            Error(
                "Textures not found in handle map %llu, %llu\n",
                m_submitLayers[0][0].hTexture,
                m_submitLayers[0][1].hTexture
            );
            return;
        }

        std::array<int, 6> fds;

        // Vulkan takes ownership of an fd when it imports it, so the copies kept
        // in the handle map are only good for a single import. Hand it a dup
        // every time or the next rebuild fails with InvalidExternalHandle.
        for (u32 i = 0; i < 3; ++i) {
            layer0Texts[i] = leftIt->second.first->sharedHandles[i];
            fds[i] = dup(leftIt->second.first->fds[i]);
        }
        for (u32 i = 0; i < 3; ++i) {
            layer0Texts[i + 3] = rightIt->second.first->sharedHandles[i];
            fds[i + 3] = dup(rightIt->second.first->fds[i]);
        }

        for (u32 i = 0; i < fds.size(); ++i) {
            if (fds[i] == -1) {
                Error("Could not duplicate texture fd %u, skipping encoder setup\n", i);
                for (u32 j = 0; j < fds.size(); ++j) {
                    if (fds[j] != -1)
                        close(fds[j]);
                }
                layer0Texts.fill(0);
                return;
            }
        }

        // Keep a second dup of each texture fd for exporting write fences at
        // Present; Vulkan consumes the import dups below.
        for (u32 i = 0; i < fds.size(); ++i) {
            if (m_syncFds[i] >= 0) {
                close(m_syncFds[i]);
            }
            m_syncFds[i] = dup(fds[i]);
        }

        auto const& settings = Settings_Instance();

        // Hopefully it's the same for both eyes in a layer
        auto& desc = leftIt->second.first->textDesc;

        alvr::render::RendererCreateInfo rendererCI {
            .format = (vk::Format)(VkFormat)desc.nFormat,
            .inputEyeExtent {
                .width = desc.nWidth,
                .height = desc.nHeight,
            },
            .outputExtent {
                .width = settings->m_recommendedTargetWidth,
                .height = settings->m_recommendedTargetHeight,
            },
            .inputImgFds = fds,
        };

        // Renderer and encoder setup can throw (Vulkan import, VAAPI). An
        // exception escaping Present kills vrserver, so log and stay idle.
        DrainPendingJob();
        std::lock_guard<std::mutex> encLock(m_encMutex);
        m_encGeneration++;

        try {
            enc.createImages(rendererCI);
            enc.initEncoding();
        } catch (std::exception const& e) {
            Error(
                "Could not set up the encoder: %s. Staying idle until the next client connect.\n",
                e.what()
            );
            enc.shutdown();
            layer0Texts.fill(0);
            m_encoderState = EncoderState::Idle;
        }

        // We'll get em next time
        return;
    }

    // TODO: Merge layers or something

    FrameJob job {};
    job.leftIdx = leftIdx.value();
    job.rightIdx = rightIdx.value();
    job.targetTimestampNs = m_targetTimestampNs;
    job.renderOrientation = m_framePoseRotation;
    job.generation = m_encGeneration;
    job.enqueueTime = std::chrono::steady_clock::now();

    // Export each eye's write fences so the render submission starts exactly
    // when the writer finishes. Both eyes or neither: a single armed eye
    // would order one read correctly and leave the other racing, which is
    // strictly harder to diagnose than the symmetric fallback.
    {
        u32 const eyeSlot[2] = { leftIdx.value(), rightIdx.value() };
        for (int e = 0; e < 2; ++e) {
            if (m_syncFds[eyeSlot[e]] >= 0) {
                job.waitFds[e] = exportReadSyncFile(m_syncFds[eyeSlot[e]]);
            }
        }
        if (job.waitFds[0] < 0 || job.waitFds[1] < 0) {
            closeWaitFds(job.waitFds);
        }
    }

    {
        std::lock_guard<std::mutex> lock(m_jobMutex);
        if (m_workerExit) {
            // Teardown already drained the mailbox; do not repopulate it.
            closeWaitFds(job.waitFds);
            return;
        }
        // Latest wins: a still-pending job is replaced, and its fences will
        // never be waited on, so close them here.
        if (m_jobPending) {
            closeWaitFds(m_job.waitFds);
        }
        m_job = job;
        m_jobPending = true;
    }
    m_jobCv.notify_one();
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

    // A direct mode driver paces the compositor: vrserver starts the next
    // frame when we send VsyncEvent. Firing it immediately lets the compositor
    // free-run as fast as encoding completes, which floods the client decoder
    // and starves the rest of the desktop. PostPresent exists so the driver
    // can hold this thread until vsync, so pace it to the negotiated rate.
    using std::chrono::nanoseconds;
    using std::chrono::steady_clock;

    auto const interval = frameInterval();
    auto const now = steady_clock::now();

    if (m_nextVsync == steady_clock::time_point {}) {
        // First frame. The default-constructed epoch would otherwise fall into
        // the catch-up loop and iterate once per frame interval of uptime.
        m_nextVsync = now + interval;
    } else if (m_nextVsync > now + interval) {
        // Clock went backwards: resynchronize.
        m_nextVsync = now + interval;
    } else {
        // Advance along the grid, losing whole ticks that were missed. A loop
        // slightly over budget every frame has to shed ticks cleanly rather
        // than accumulate debt and then snap, which reads as a burst-pause
        // cadence with tens of milliseconds of queueing.
        m_nextVsync += interval;
        while (m_nextVsync <= now) {
            m_nextVsync += interval;
            m_skippedVsyncs++;
        }
    }

    // Hold the next tick for the frames SteamVR asked us to throttle, so the
    // cadence we signal matches the one its pose prediction assumes.
    m_nextVsync += interval * throttleFrames;

    // Wake a phase lead before the tick and declare the vsync at that announce
    // point. VsyncEvent takes a time offset in seconds, so declaring early
    // gives WaitGetPoses release and pose prediction a head start on the real
    // deadline. The offset self-corrects wake jitter: it goes slightly
    // negative when we oversleep, which reads as the vsync having just passed.
    auto const announce = m_nextVsync - RunningStart;
    std::this_thread::sleep_until(announce);

    double offset
        = std::chrono::duration_cast<std::chrono::duration<double>>(announce - steady_clock::now())
              .count();
    vr::VRServerDriverHost()->VsyncEvent(offset);
}

void OvrDirectModeComponent::GetFrameTiming(vr::DriverDirectMode_FrameTiming* pFrameTiming) {
    if (pFrameTiming == nullptr
        || pFrameTiming->m_nSize < sizeof(vr::DriverDirectMode_FrameTiming)) {
        return;
    }

    // Reporting skipped ticks as dropped frames looks like honest cadence
    // feedback, but the compositor reads it as GPU overload and turns on
    // interleaved reprojection: half rate, so the grid then skips a tick every
    // frame, so we report another drop, throttled forever. Measured as Home
    // locked to 35 with the report in and 72 without. Consume the counter
    // without reporting it until there is a channel that does not throttle.
    (void)m_skippedVsyncs.exchange(0);
    pFrameTiming->m_nNumFramePresents = 1;
    pFrameTiming->m_nNumMisPresented = 0;
    pFrameTiming->m_nNumDroppedFrames = 0;
    // These bits overlap the throttle mask and must not be echoed back
    // unhandled, per the interface header.
    pFrameTiming->m_nReprojectionFlags = 0;
}
