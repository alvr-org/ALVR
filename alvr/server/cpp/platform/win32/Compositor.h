#pragma once

#include "CEncoder.h"
#include "alvr_server/ClientConnection.h"
#include "alvr_server/PoseHistory.h"
#include "alvr_server/Settings.h"
#include "alvr_server/Utils.h"
#include "openvr_driver.h"
#include <array>
#include <map>

// Note not a compositor, only the interface. The actual compositor in inside FrameRender.cpp
class Compositor {
  public:
    Compositor(std::shared_ptr<CD3DRender> m_pD3DRender,
               std::shared_ptr<PoseHistory> m_poseHistory);

    void SetEncoder(std::shared_ptr<CEncoder> m_pEncoder);

    uint64_t CreateTexture(
        uint32_t width, uint32_t height, uint32_t format, uint32_t sampleCount, void *texture);

    void DestroyTexture(uint64_t id);

    void PresentLayers(void *syncTexture, const Layer *layers, uint64_t layerCount);

  private:
    std::shared_ptr<CD3DRender> m_pD3DRender;
    std::shared_ptr<CEncoder> m_pEncoder;
    std::shared_ptr<ClientConnection> m_Listener;
    std::shared_ptr<PoseHistory> m_poseHistory;

    std::map<uint64_t, ComPtr<ID3D11Texture2D>> m_textures;

    vr::HmdQuaternion_t m_prevFramePoseRotation;
    vr::HmdQuaternion_t m_framePoseRotation;
    uint64_t m_submitFrameIndex;
    uint64_t m_submitClientTime;
    uint64_t m_prevSubmitFrameIndex;
    uint64_t m_prevSubmitClientTime;
};