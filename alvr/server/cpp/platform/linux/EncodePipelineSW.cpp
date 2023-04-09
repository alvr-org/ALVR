#include "EncodePipelineSW.h"

#include <chrono>
#include <string.h>

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
  use_x264 = settings.m_codec == ALVR_CODEC_H264;

  if (use_x264) {
    x264_param_default_preset(&avc.param, "ultrafast", "zerolatency");
    x264_param_apply_profile(&avc.param, "high");

    avc.param.pf_log = x264_log;
    avc.param.i_log_level = X264_LOG_INFO;

    avc.param.b_aud = 0;
    avc.param.b_cabac = settings.m_entropyCoding == ALVR_CABAC;
    avc.param.b_sliced_threads = true;
    avc.param.i_threads = settings.m_swThreadCount;
    avc.param.i_width = width;
    avc.param.i_height = height;
    avc.param.rc.i_rc_method = X264_RC_ABR;
  } else {
    hevc.api = x265_api_get(8);
    if (!hevc.api) {
      throw std::runtime_error("Failed to get x265 api");
    }
    hevc.param = hevc.api->param_alloc();
    hevc.api->param_default_preset(hevc.param, "ultrafast", "zerolatency");
    hevc.api->param_apply_profile(hevc.param, "main");

    hevc.param->logLevel = X265_LOG_INFO;

    hevc.param->sourceWidth = width;
    hevc.param->sourceHeight = height;
    hevc.param->rc.rateControlMode = X265_RC_ABR;
    hevc.param->internalCsp = X265_CSP_I420;
    hevc.param->bRepeatHeaders = 1;
  }

  auto params = FfiDynamicEncoderParams {};
  params.updated = true;
  params.bitrate_bps = 30'000'000;
  params.framerate = Settings::Instance().m_refreshRate;
  SetParams(params);

  bool opened = false;
  if (use_x264) {
    avc.enc = x264_encoder_open(&avc.param);
    opened = avc.enc;
  } else {
    hevc.enc = hevc.api->encoder_open(hevc.param);
    opened = hevc.enc;
  }
  if (!opened) {
    throw std::runtime_error("Failed to open encoder");
  }

  if (use_x264) {
    x264_picture_init(&avc.picture);
    avc.picture.img.i_csp = X264_CSP_I420;
    avc.picture.img.i_plane = 3;

    x264_picture_init(&avc.picture_out);
  } else {
    hevc.api->picture_init(hevc.param, &hevc.picture);
    hevc.api->picture_init(hevc.param, &hevc.picture_out);
  }

  rgbtoyuv = new RgbToYuv420(render, render->GetOutput().image, render->GetOutput().imageInfo, render->GetOutput().semaphore);
}

alvr::EncodePipelineSW::~EncodePipelineSW()
{
  if (rgbtoyuv) {
    delete rgbtoyuv;
  }
  if (use_x264) {
    if (avc.enc) {
      x264_encoder_close(avc.enc);
    }
  } else {
    if (hevc.param) {
      hevc.api->param_free(hevc.param);
    }
    if (hevc.enc) {
      hevc.api->encoder_close(hevc.enc);
    }
  }
}

void alvr::EncodePipelineSW::PushFrame(uint64_t targetTimestampNs, bool idr)
{
  if (use_x264) {
    rgbtoyuv->Convert(avc.picture.img.plane, avc.picture.img.i_stride);
    avc.picture.i_type = idr ? X264_TYPE_IDR : X264_TYPE_AUTO;
    pts = avc.picture.i_pts = targetTimestampNs;
  } else {
    rgbtoyuv->Convert(reinterpret_cast<uint8_t**>(hevc.picture.planes), hevc.picture.stride);
    hevc.picture.sliceType = idr ? X265_TYPE_IDR : X265_TYPE_AUTO;
    pts = hevc.picture.pts = targetTimestampNs;
  }
  rgbtoyuv->Sync();
  timestamp.cpu = std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now().time_since_epoch()).count();

  if (use_x264) {
    int nnal = 0;
    nal_size = x264_encoder_encode(avc.enc, &avc.nal, &nnal, &avc.picture, &avc.picture_out);
    if (nal_size < 0) {
      throw std::runtime_error("x264 encoder_encode failed");
    }
  } else {
    uint32_t nnal = 0;
    int ret = hevc.api->encoder_encode(hevc.enc, &hevc.nal, &nnal, &hevc.picture, &hevc.picture_out);
    if (ret < 0) {
      throw std::runtime_error("x265 encoder_encode failed");
    }
    nal_size = 0;
    for (uint32_t i = 0; i < nnal; ++i) {
      nal_size += hevc.nal[i].sizeBytes;
    }
  }
}

bool alvr::EncodePipelineSW::GetEncoded(FramePacket &packet)
{
  if (use_x264) {
    if (!avc.nal) {
      return false;
    }
    packet.data = avc.nal[0].p_payload;
  } else {
    if (!hevc.nal) {
      return false;
    }
    packet.data = hevc.nal[0].payload;
  }
  packet.size = nal_size;
  packet.pts = pts;
  return packet.size > 0;
}

void alvr::EncodePipelineSW::SetParams(FfiDynamicEncoderParams params)
{
  if (!params.updated) {
    return;
  }
  if (use_x264) {
    // x264 doesn't work well with adaptive bitrate/fps
    avc.param.i_fps_num = Settings::Instance().m_refreshRate;
    avc.param.i_fps_den = 1;
    avc.param.rc.i_bitrate = params.bitrate_bps / 1'000 * 1.4; // needs higher value to hit target bitrate
    avc.param.rc.i_vbv_buffer_size = avc.param.rc.i_bitrate / avc.param.i_fps_num * 1.1;
    avc.param.rc.i_vbv_max_bitrate = avc.param.rc.i_bitrate;
    avc.param.rc.f_vbv_buffer_init = 0.75;
    if (avc.enc) {
      x264_encoder_reconfig(avc.enc, &avc.param);
    }
  } else {
    // x265 doesn't work well with adaptive bitrate/fps
    hevc.param->fpsNum = Settings::Instance().m_refreshRate;
    hevc.param->fpsDenom = 1;
    hevc.param->rc.bitrate = params.bitrate_bps / 1'000 * 1.4; // needs higher value to hit target bitrate
    hevc.param->rc.vbvBufferSize = hevc.param->rc.bitrate / hevc.param->fpsNum * 1.1;
    hevc.param->rc.vbvMaxBitrate = hevc.param->rc.bitrate;
    hevc.param->rc.vbvBufferInit = 0.75;
    if (hevc.enc) {
      hevc.api->encoder_reconfig(hevc.enc, hevc.param);
    }
  }
}
