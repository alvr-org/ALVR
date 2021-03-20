#include "ffmpeg.h"

#include "libavcodec/avcodec.h"
#include "libavutil/avutil.h"

void setup_video_encoder() {
    AVCodec *codec;
    AVCodecContext *c = NULL;
}