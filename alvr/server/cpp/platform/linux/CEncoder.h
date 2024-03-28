#pragma once

#include "EncodePipeline.h"
#include "FrameRender.h"
#include "alvr_server/Utils.h"
#include "alvr_server/IDRScheduler.h"
#include "shared/threadtools.h"
#include <atomic>
#include <memory>
#include <poll.h>
#include <sys/types.h>
#include <vulkan/vulkan_core.h>

class PoseHistory;

class CEncoder : public CThread {
  public:
    CEncoder();
    ~CEncoder();
    bool Init() override { return true; }

    bool CopyToStaging(VkImage *pTexture[][2], vr::VRTextureBounds_t bounds[][2], int layerCount, bool recentering
			, uint64_t presentationTime, uint64_t targetTimestampNs, const std::string& message, const std::string& debugText);

    void Run() override;
    void Stop();

    void NewFrameReady();  //TODO: implement for linux directmode
		void WaitForEncode();  //TODO: implement for linux directmode
    void OnStreamStart();
    void OnPacketLoss();
    void InsertIDR();
    bool IsConnected() { return m_connected; }
    void CaptureFrame();

  private:
    //void GetFds(int client, int (*fds)[6]);
    CThreadEvent m_newFrameReady, m_encodeFinished; //TODO use to sync present with encoded frames
    std::shared_ptr<alvr::EncodePipeline> m_videoEncoder;
    std::atomic_bool m_exiting{false};
    uint64_t m_presentationTime;
		uint64_t m_targetTimestampNs;
  
    std::shared_ptr<FrameRender> m_FrameRender;
    
    IDRScheduler m_scheduler;
    bool m_connected = false;
    std::atomic_bool m_captureFrame = false;
};
