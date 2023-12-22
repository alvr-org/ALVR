#include "EncodePipelineSW.h"

#include <chrono>

#include "alvr_server/Settings.h"
#include "alvr_server/Logger.h"
#include "FormatConverter.h"

namespace
{

void x264_log(void *, int level, const char *fmt, va_list args)
{
    char buf[256];
    vsnprintf(buf, sizeof(buf), fmt, args);
    switch (level) {
    case X264_LOG_ERROR:
        Error("x264: %s", buf);
        break;
    case X264_LOG_WARNING:
        Warn("x264: %s", buf);
        break;
    case X264_LOG_INFO:
        Info("x264: %s", buf);
        break;
    case X264_LOG_DEBUG:
        Debug("x264: %s", buf);
        break;
    default:
        break;
    }
}

}

alvr::EncodePipelineSW::EncodePipelineSW(Renderer *render, uint32_t width, uint32_t height)
{
  const auto& settings = Settings::Instance();

  x264_param_default_preset(&param, "ultrafast", "zerolatency");

  param.pf_log = x264_log;
  param.i_log_level = X264_LOG_INFO;

  param.b_aud = 0;
  param.b_cabac = settings.m_entropyCoding == ALVR_CABAC;
  param.b_sliced_threads = true;
  param.i_threads = settings.m_swThreadCount;
  param.i_width = width;
  param.i_height = height;
  param.rc.i_rc_method = X264_RC_ABR;

  switch (settings.m_h264Profile) {
  case ALVR_H264_PROFILE_BASELINE:
    x264_param_apply_profile(&param, "baseline");
    break;
  case ALVR_H264_PROFILE_MAIN:
    x264_param_apply_profile(&param, "main");
    break;
  default:
  case ALVR_H264_PROFILE_HIGH:
    x264_param_apply_profile(&param, "high");
    break;
  }

  auto params = FfiDynamicEncoderParams {};
  params.updated = true;
  params.bitrate_bps = 30'000'000;
  params.framerate = Settings::Instance().m_refreshRate;
  SetParams(params);

  enc = x264_encoder_open(&param);
  if (!enc) {
    throw std::runtime_error("Failed to open encoder");
  }

  x264_picture_init(&picture);
  picture.img.i_csp = X264_CSP_I420;
  picture.img.i_plane = 3;

  x264_picture_init(&picture_out);

  rgbtoyuv = new RgbToYuv420(render, render->GetOutput().image, render->GetOutput().imageInfo, render->GetOutput().semaphore);
}

alvr::EncodePipelineSW::~EncodePipelineSW()
{
  if (rgbtoyuv) {
    delete rgbtoyuv;
  }
  if (enc) {
    x264_encoder_close(enc);
  }
}

void alvr::EncodePipelineSW::PushFrame(uint64_t targetTimestampNs, bool idr)
{
  rgbtoyuv->Convert(picture.img.plane, picture.img.i_stride);
  rgbtoyuv->Sync();
  timestamp.cpu = std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now().time_since_epoch()).count();

  picture.i_type = idr ? X264_TYPE_IDR : X264_TYPE_AUTO;
  pts = picture.i_pts = targetTimestampNs;

  int nnal = 0;
  nal_size = x264_encoder_encode(enc, &nal, &nnal, &picture, &picture_out);
  if (nal_size < 0) {
    throw std::runtime_error("x264 encoder_encode failed");
  }
}

bool alvr::EncodePipelineSW::GetEncoded(FramePacket &packet)
{
  if (!nal) {
    return false;
  }
  packet.size = nal_size;
  packet.data = nal[0].p_payload;
  packet.pts = pts;
  return packet.size > 0;
}

void alvr::EncodePipelineSW::SetParams(FfiDynamicEncoderParams params)
{
  if (!params.updated) {
    return;
  }
  // x264 doesn't work well with adaptive bitrate/fps
  param.i_fps_num = Settings::Instance().m_refreshRate;
  param.i_fps_den = 1;
  param.rc.i_bitrate = params.bitrate_bps / 1'000 * 1.4; // needs higher value to hit target bitrate
  param.rc.i_vbv_buffer_size = param.rc.i_bitrate / param.i_fps_num * 1.1;
  param.rc.i_vbv_max_bitrate = param.rc.i_bitrate;
  param.rc.f_vbv_buffer_init = 0.75;
  if (enc) {
    x264_encoder_reconfig(enc, &param);
  }
}

int alvr::EncodePipelineSW::GetCodec()
{
  return ALVR_CODEC_H264;
}
