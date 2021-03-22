#include "ffmpeg.h"

#include "libavcodec/avcodec.h"
#include "libavutil/avutil.h"

struct Encoder {
    AVCodecContext *context;
};

struct Encoder *encoder_alloc(struct EncoderAllocData setup_data) {
    struct Encoder *encoder = malloc(sizeof(struct Encoder));

    return encoder;
}

void encoder_free(struct Encoder **encoder) {
    free(*encoder);
    *encoder = NULL;
}