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

OvrDirectModeComponent::OvrDirectModeComponent(std::shared_ptr<PoseHistory> poseHistory)
    : m_poseHistory(poseHistory)
    , m_submitLayer(0) { }

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
        auto pid = getpid();
        Info("VrServer PID %d\n", pid);
    }

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
            &myHandle
        );

        uint64_t ipcHandle = 0;
        vr::VRIPCResourceManager()->RefResource(myHandle, &ipcHandle);

        if (!success) {
            Error("VRCIPCResourceManager: Failed to create shared texture\n");
            for (int j = 0; j < i; j++) {
                vr::VRIPCResourceManager()->UnrefResource(processResource->sharedHandles[j]);
                m_handleMap.erase(processResource->sharedHandles[j]);
            }
            delete processResource;
            break;
        }

        int fd = 0;
        auto ret = vr::VRIPCResourceManager()->ReceiveSharedFd(ipcHandle, &fd);
        if (ret == false) {
            Error("Failed to get fd for texture\n");
            break;
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

    auto id = m_handleMap.find(sharedTextureHandle);
    if (id != m_handleMap.end()) {
        ProcessResource* p = id->second.first;

        vr::VRIPCResourceManager()->UnrefResource(p->sharedHandles[0]);
        vr::VRIPCResourceManager()->UnrefResource(p->sharedHandles[1]);
        vr::VRIPCResourceManager()->UnrefResource(p->sharedHandles[2]);

        m_handleMap.erase(p->sharedHandles[0]);
        m_handleMap.erase(p->sharedHandles[1]);
        m_handleMap.erase(p->sharedHandles[2]);
        delete p;
    } else {
        Debug("Requested to destroy not managing texture. handle:%p\n", sharedTextureHandle);
    }
}

/** Used to purge all texture sets for a given process. */
void OvrDirectModeComponent::DestroyAllSwapTextureSets(uint32_t unPid) {
    Info("DestroyAllSwapTextureSets pid=%d\n", unPid);
    for (auto it = m_handleMap.begin(); it != m_handleMap.end();) {
        if (it->second.first->pid == unPid) {
            if (it->second.second == 0) {
                ProcessResource* p = it->second.first;
                vr::VRIPCResourceManager()->UnrefResource(p->sharedHandles[0]);
                vr::VRIPCResourceManager()->UnrefResource(p->sharedHandles[1]);
                vr::VRIPCResourceManager()->UnrefResource(p->sharedHandles[2]);
                delete p;
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
    (*pIndices)[0]++;
    (*pIndices)[0] %= 3;
    (*pIndices)[1]++;
    (*pIndices)[1] %= 3;
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
            m_framePoseRotation.x = pose->motion.orientation.x;
            m_framePoseRotation.y = pose->motion.orientation.y;
            m_framePoseRotation.z = pose->motion.orientation.z;
            m_framePoseRotation.w = pose->motion.orientation.w;
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

        for (u32 i = 0; i < 3; ++i) {
            layer0Texts[i] = leftIt->second.first->sharedHandles[i];
            fds[i] = leftIt->second.first->fds[i];
        }
        for (u32 i = 0; i < 3; ++i) {
            layer0Texts[i + 3] = rightIt->second.first->sharedHandles[i];
            fds[i + 3] = rightIt->second.first->fds[i];
        }

        auto const& settings = Settings::Instance();

        // Hopefully it's the same for both eyes in a layer
        auto& desc = leftIt->second.first->textDesc;

        alvr::render::RendererCreateInfo rendererCI {
            .format = (vk::Format)(VkFormat)desc.nFormat,
            .inputEyeExtent {
                .width = desc.nWidth,
                .height = desc.nHeight,
            },
            .outputExtent {
                .width = settings.m_recommendedTargetWidth,
                .height = settings.m_recommendedTargetHeight,
            },
            .inputImgFds = fds,
        };

        enc.createImages(rendererCI);
        enc.initEncoding();

        // We'll get em next time
        return;
    }

    // TODO: Merge layers or something

    enc.present(leftIdx.value(), rightIdx.value(), m_targetTimestampNs);
}

void OvrDirectModeComponent::PostPresent(const Throttling_t* pThrottling) { /* WaitForVSync(); */
}
