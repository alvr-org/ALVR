#include "EncodePipelineSW.h"

#include <chrono>

#include "FormatConverter.h"
#include "alvr_server/Logger.h"
#include "alvr_server/Settings.h"

#include <cstring>

namespace {

void h264_log(void*, int level, const char* message) {
    switch (level) {
    case WELS_LOG_ERROR:
        Error("h264: %s", message);
        break;
    case WELS_LOG_WARNING:
        Warn("h264: %s", message);
        break;
    case WELS_LOG_INFO:
        Info("h264: %s", message);
        break;
    case WELS_LOG_DEBUG:
        Debug("h264: %s", message);
        break;
    default:
        break;
    }
}

}

alvr::EncodePipelineSW::EncodePipelineSW(Renderer* render, uint32_t width, uint32_t height) {
    const auto& settings = Settings::Instance();
    SEncParamExt param;

    int rv = WelsCreateSVCEncoder (&encoder_);
    assert (rv == 0);
    assert (encoder_ != NULL);

    encoder_->GetDefaultParams (&param);
    param.iUsageType = SCREEN_CONTENT_REAL_TIME;
    param.iPicWidth = width;
    param.iPicHeight = height;
    param.iTargetBitrate = 30'000'000;
    param.fMaxFrameRate = Settings::Instance().m_refreshRate;
    param.iRCMode = RC_BITRATE_MODE;

    param.iComplexityMode = LOW_COMPLEXITY;
    param.bPrefixNalAddingCtrl = 0;
    param.iEntropyCodingModeFlag = settings.m_entropyCoding == ALVR_CABAC;

    
    param.iSpatialLayerNum = 1;
    param.iMultipleThreadIdc = settings.m_swThreadCount;

    for (int i = 0; i < param.iSpatialLayerNum; i++) {
        param.sSpatialLayers[i].iVideoWidth = width >> (param.iSpatialLayerNum - 1 - i);
        param.sSpatialLayers[i].iVideoHeight = height >> (param.iSpatialLayerNum - 1 - i);
        param.sSpatialLayers[i].fFrameRate = Settings::Instance().m_refreshRate;
        param.sSpatialLayers[i].iSpatialBitrate = 30'000'000;

        param.sSpatialLayers[i].sSliceArgument.uiSliceMode = SM_SINGLE_SLICE;

        switch (settings.m_h264Profile) {
        case ALVR_H264_PROFILE_BASELINE:
            param.sSpatialLayers[i].uiProfileIdc = PRO_BASELINE;
            break;
        case ALVR_H264_PROFILE_MAIN:
            param.sSpatialLayers[i].uiProfileIdc = PRO_MAIN;
            break;
        default:
        case ALVR_H264_PROFILE_HIGH:
            param.sSpatialLayers[i].uiProfileIdc = PRO_HIGH;
            break;
        }

    }
    encoder_->InitializeExt (&param);

    int level = WELS_LOG_DEBUG;
    encoder_->SetOption (ENCODER_OPTION_TRACE_CALLBACK, (void *)h264_log);
    encoder_->SetOption (ENCODER_OPTION_TRACE_LEVEL, &level);

    auto params = FfiDynamicEncoderParams {};
    params.updated = true;
    params.bitrate_bps = 30'000'000;
    params.framerate = Settings::Instance().m_refreshRate;
    SetParams(params);

    int videoFormat = videoFormatI420;
    encoder_->SetOption (ENCODER_OPTION_DATAFORMAT, &videoFormat);
    
    int frameSize = width * height * 3 / 2;
    buf = new uint8_t[frameSize];
    buf_out = new std::vector<uint8_t>();
    memset (&info, 0, sizeof (SFrameBSInfo));
    memset (&pic, 0, sizeof (SSourcePicture));
    pic.iPicWidth = width;
    pic.iPicHeight = height;
    pic.iColorFormat = videoFormatI420;
    pic.iStride[0] = pic.iPicWidth;
    pic.iStride[1] = pic.iStride[2] = pic.iPicWidth >> 1;
    pic.pData[0] = buf;
    pic.pData[1] = pic.pData[0] + width * height;
    pic.pData[2] = pic.pData[1] + (width * height >> 2);

    rgbtoyuv = new RgbToYuv420(
        render,
        render->GetOutput().image,
        render->GetOutput().imageInfo,
        render->GetOutput().semaphore
    );
}

alvr::EncodePipelineSW::~EncodePipelineSW() {
    if (rgbtoyuv) {
        delete rgbtoyuv;
    }
    if (encoder_) {
        encoder_->Uninitialize();
        WelsDestroySVCEncoder (encoder_);
        delete buf;
        delete buf_out;
    }
}

void alvr::EncodePipelineSW::PushFrame(uint64_t targetTimestampNs, bool idr) {
    rgbtoyuv->Convert(pic.pData, pic.iStride);
    rgbtoyuv->Sync();
    timestamp.cpu = std::chrono::duration_cast<std::chrono::nanoseconds>(
                        std::chrono::steady_clock::now().time_since_epoch()
    )
                        .count();

    encoder_->ForceIntraFrame(idr);
    pts = pic.uiTimeStamp = targetTimestampNs;
    is_idr = idr;

    //prepare input data
    int rv = encoder_->EncodeFrame (&pic, &info);
    if (rv != cmResultSuccess) {
        throw std::runtime_error("openh264 EncodeFrame failed");
    }

    buf_out->clear();
    if (info.eFrameType != videoFrameTypeSkip) 
    {
        //output bitstream
        for (int iLayer=0; iLayer < info.iLayerNum; iLayer++)
        {
            SLayerBSInfo* pLayerBsInfo = &info.sLayerInfo[iLayer];

            int iLayerSize = 0;
            int iNalIdx = pLayerBsInfo->iNalCount - 1;
            do {
                iLayerSize += pLayerBsInfo->pNalLengthInByte[iNalIdx];
                --iNalIdx;
            } while (iNalIdx >= 0);

            unsigned char *outBuf = pLayerBsInfo->pBsBuf;
            buf_out->insert(buf_out->end(), outBuf, outBuf+iLayerSize);
        }
    }
}

bool alvr::EncodePipelineSW::GetEncoded(FramePacket& packet) {
    if (buf_out->size() == 0) {
        return false;
    }
    packet.size = buf_out->size();
    packet.data = buf_out->data();
    packet.pts = pts;
    packet.isIDR = is_idr;
    return packet.size > 0;
}

void alvr::EncodePipelineSW::SetParams(FfiDynamicEncoderParams params) {
    if (!params.updated) {
        return;
    }
    // x264 doesn't work well with adaptive bitrate/fps
    encoder_->SetOption (ENCODER_OPTION_FRAME_RATE, &Settings::Instance().m_refreshRate);
    encoder_->SetOption (ENCODER_OPTION_BITRATE, &params.bitrate_bps); // needs higher value to hit target bitrate
}

int alvr::EncodePipelineSW::GetCodec() { return ALVR_CODEC_H264; }
