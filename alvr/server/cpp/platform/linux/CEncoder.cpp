#include "CEncoder.h"

#include <chrono>
#include <exception>
#include <memory>
#include <poll.h>
#include <sstream>
#include <stdexcept>
#include <stdlib.h>
#include <string>
#include <sys/mman.h>
#include <sys/poll.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>
#include <iostream>
#include <fstream>
#include <vulkan/vulkan_core.h>

#include "ALVR-common/packet_types.h"
#include "alvr_server/Logger.h"
#include "alvr_server/PoseHistory.h"
#include "alvr_server/Settings.h"
#include "protocol.h"
#include "ffmpeg_helper.h"
#include "EncodePipeline.h"
#include "FrameRender.h"

extern "C" {
#include <libavutil/avutil.h>
}

CEncoder::CEncoder()
    : m_exiting(false)
    , m_targetTimestampNs(0) {
      m_encodeFinished.Set();
    }

CEncoder::~CEncoder() {
   if (m_videoEncoder) {
    m_videoEncoder->Shutdown();
    m_videoEncoder.reset();
   }
}

namespace {

void av_logfn(void*, int level, const char* data, va_list va)
{
  if (level >
#ifdef DEBUG
          AV_LOG_DEBUG)
#else
          AV_LOG_INFO)
#endif
    return;

  char buf[256];
  vsnprintf(buf, sizeof(buf), data, va);

  if (level <= AV_LOG_ERROR)
    Error("Encoder: %s", buf);
  else
    Info("Encoder: %s", buf);
}
} // namespace

bool CEncoder::CopyToStaging(VkImage *pTexture[][2], vr::VRTextureBounds_t bounds[][2], int layerCount, bool recentering
			, uint64_t presentationTime, uint64_t targetTimestampNs, const std::string& message, const std::string& debugText) {
			m_presentationTime = presentationTime;
			m_targetTimestampNs = targetTimestampNs;
			m_FrameRender->Startup();

			m_FrameRender->RenderFrame(pTexture, bounds, layerCount, recentering, message, debugText);
			return true;
}

void CEncoder::Run() {
    Info("CEncoder::Run\n");

    Debug("CEncoder: Start thread. Id=%d\n", GetCurrentThreadId());
		SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_MOST_URGENT);

    auto encode_pipeline = alvr::EncodePipeline::Create(&render, vk_ctx, frame, vk_frame_ctx, render.GetEncodingWidth(), render.GetEncodingHeight());

    try {
      while (not m_exiting) {
        m_newFrameReady.Wait();
				if (m_exiting)
					break;

        encode_pipeline->SetParams(GetDynamicEncoderParams());

        if (m_captureFrame) {
          m_captureFrame = false;
          render.CaptureInputFrame(Settings::Instance().m_captureFrameDir + "/alvr_frame_input.ppm");
          render.CaptureOutputFrame(Settings::Instance().m_captureFrameDir + "/alvr_frame_output.ppm");
        }

        //TODO: Get and transmit texture
        encode_pipeline->PushFrame(nullptr, m_presentationTime, m_targetTimestampNs, m_scheduler.CheckIDRInsertion());

        m_encodeFinished.Set();
      }
    }
    catch (std::exception &e) {
      std::stringstream err;
      err << "error in encoder thread: " << e.what();
      Error(err.str().c_str());
    }
}

void CEncoder::Stop() {
			m_bExiting = true;
			m_newFrameReady.Set();
			Join();
			m_FrameRender.reset();
}

void CEncoder::NewFrameReady() {
			m_encodeFinished.Reset();
			m_newFrameReady.Set();
}

void CEncoder::WaitForEncode() {
			m_encodeFinished.Wait();
}

void CEncoder::OnStreamStart() { m_scheduler.OnStreamStart(); }

void CEncoder::OnPacketLoss() { m_scheduler.OnPacketLoss(); }

void CEncoder::InsertIDR() { m_scheduler.InsertIDR(); }

void CEncoder::CaptureFrame() { m_captureFrame = true; }
