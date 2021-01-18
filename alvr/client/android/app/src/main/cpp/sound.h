#ifndef ALVRCLIENT_SOUND_H
#define ALVRCLIENT_SOUND_H

#include <SLES/OpenSLES.h>
#include <SLES/OpenSLES_Android.h>

#include "utils.h"

class SoundPlayer {
    static const int DST_BUF_SAMPLE = 512;
    static const int BUF_COUNT = 50;
    static const int DST_RATE = 48000;
    static const int DST_BYTES = 2;
    static const int START_THRESHOLD = 5;

public:
    SoundPlayer() {
        dst_head = 0;
        dst_tail = 0;
        dst_end = 0;
        dst_nb_channels = 2;
        dst_byte_per_frame = 2 * dst_nb_channels;

        auto p = new uint8_t[DST_BUF_SAMPLE * dst_byte_per_frame * BUF_COUNT];
        for (int i = 0; i < BUF_COUNT; i++) {
            dst_data_list[i] = p + i * DST_BUF_SAMPLE * dst_byte_per_frame;
        }

        silent_buf = new uint8_t[DST_BUF_SAMPLE * dst_byte_per_frame];
        memset(silent_buf, 0, DST_BUF_SAMPLE * dst_byte_per_frame);
    }

    ~SoundPlayer() {
        destroy();
    }

    int initialize() {
        SLresult result;

        result = slCreateEngine(&engineObject, 0, NULL, 0, NULL, NULL);
        if (SL_RESULT_SUCCESS != result) {
            LOGSOUND("slCreateEngine:%d\n", result);
            return -1;
        }

        result = (*engineObject)->Realize(engineObject,
                                          SL_BOOLEAN_FALSE);
        if (SL_RESULT_SUCCESS != result) {
            LOGSOUND("Realize:%d\n", result);
            return -1;
        }

        result = (*engineObject)->GetInterface(engineObject, SL_IID_ENGINE,
                                               &engineEngine);
        if (SL_RESULT_SUCCESS != result) {
            LOGSOUND("GetInterface:%d\n", result);
            return -1;
        }

        result = (*engineEngine)->CreateOutputMix(engineEngine,
                                                  &outputMixObject, 0, NULL, NULL);
        if (SL_RESULT_SUCCESS != result) {
            LOGSOUND("CreateOutputMix:%d\n", result);
            return -1;
        }
        result = (*outputMixObject)->Realize(outputMixObject, SL_BOOLEAN_FALSE);
        if (SL_RESULT_SUCCESS != result) {
            LOGSOUND("Realize outputMixObject:%d\n", result);
            return -1;
        }

        SLDataLocator_AndroidSimpleBufferQueue loc_bufq = {SL_DATALOCATOR_ANDROIDSIMPLEBUFFERQUEUE,
                                                           BUF_COUNT};
        SLAndroidDataFormat_PCM_EX format_pcm;
        SLDataSource audioSrc = {&loc_bufq, &format_pcm};

        format_pcm.formatType = SL_ANDROID_DATAFORMAT_PCM_EX;
        format_pcm.numChannels = (SLuint32) dst_nb_channels;
        format_pcm.sampleRate = DST_RATE * 1000;
        format_pcm.bitsPerSample = (SLuint32) DST_BYTES * 8;
        format_pcm.containerSize = (SLuint32) DST_BYTES * 8;
        format_pcm.channelMask = (dst_nb_channels == 1) ? SL_SPEAKER_FRONT_CENTER : (
                SL_SPEAKER_FRONT_LEFT | SL_SPEAKER_FRONT_RIGHT);
        format_pcm.endianness = SL_BYTEORDER_LITTLEENDIAN;
        format_pcm.representation = SL_ANDROID_PCM_REPRESENTATION_SIGNED_INT;

        SLDataLocator_OutputMix loc_outmix = {SL_DATALOCATOR_OUTPUTMIX, outputMixObject};
        SLDataSink audioSnk = {&loc_outmix, NULL};

        const SLInterfaceID ids[2] = {SL_IID_BUFFERQUEUE, SL_IID_VOLUME};
        const SLboolean req[2] = {SL_BOOLEAN_TRUE, SL_BOOLEAN_TRUE};

        result = (*engineEngine)->CreateAudioPlayer(engineEngine,
                                                    &bqPlayerObject, &audioSrc,
                                                    &audioSnk,
                                                    2, ids, req);
        if (SL_RESULT_SUCCESS != result) {
            bqPlayerObject = NULL;
            return 1;
        }

        result = (*bqPlayerObject)->Realize(bqPlayerObject, SL_BOOLEAN_FALSE);
        if (SL_RESULT_SUCCESS != result) {
            bqPlayerObject = NULL;
            return 1;
        }

        result = (*bqPlayerObject)->GetInterface(bqPlayerObject, SL_IID_PLAY,
                                                 &bqPlayerPlay);
        if (SL_RESULT_SUCCESS != result) {
            bqPlayerObject = NULL;
            return 1;
        }

        result = (*bqPlayerObject)->GetInterface(bqPlayerObject, SL_IID_BUFFERQUEUE,
                                                 &bqPlayerBufferQueue);
        if (SL_RESULT_SUCCESS != result) {
            bqPlayerObject = NULL;
            return 1;
        }

        result = (*bqPlayerObject)->GetInterface(bqPlayerObject, SL_IID_VOLUME,
                                                 &bqPlayerVolume);
        if (SL_RESULT_SUCCESS != result) {
            bqPlayerObject = NULL;
            return 1;
        }

        result = (*bqPlayerBufferQueue)->RegisterCallback(bqPlayerBufferQueue,
                                                          callback,
                                                          this);
        if (SL_RESULT_SUCCESS != result) {
            bqPlayerObject = NULL;
            return 1;
        }

        result = (*bqPlayerPlay)->SetPlayState(bqPlayerPlay,
                                               SL_PLAYSTATE_STOPPED);
        if (SL_RESULT_SUCCESS != result) {
            bqPlayerObject = NULL;
            return 1;
        }

        audio_frame_count = 0;

        return 0;
    }

    void destroy() {
        LOGSOUND("Destroy AudioPlayer.");
        if(bqPlayerPlay != NULL) {
            (*bqPlayerPlay)->SetPlayState(bqPlayerPlay,
                                          SL_PLAYSTATE_STOPPED);
        }
        if (bqPlayerObject != NULL) {
            (*bqPlayerObject)->Destroy(bqPlayerObject);
        }
        if (outputMixObject != NULL) {
            (*outputMixObject)->Destroy(outputMixObject);
        }
        if (engineObject != NULL) {
            (*engineObject)->Destroy(engineObject);
        }
        delete [] dst_data_list[0];
        delete silent_buf;
    }


    int putData(uint8_t *buf, int len) {
        audio_frame_count++;

        if (discard_frame > 0) {
            discard_frame--;
            LOGSOUND("Discard audio buffer. remain=%d", discard_frame);
            return 1;
        }

        while (len > 0) {
            if (dst_tail - dst_head >= BUF_COUNT) {
                // full
                LOGSOUND("SoundPlayer: Buffer is full.");
                break;
            }
            uint8_t *dst_data = dst_data_list[dst_tail % BUF_COUNT];

            int remain = DST_BUF_SAMPLE * dst_byte_per_frame - dst_end;
            uint8_t *p = dst_data + dst_end;

            int size = std::min(len, remain);
            memcpy(p, buf, size);
            dst_end += size;
            len -= size;
            buf += size;

            if (dst_end >= DST_BUF_SAMPLE * dst_byte_per_frame) {
                dst_tail++;
                dst_end = 0;
            }
        }

        SLuint32 ps = 0;
        (*bqPlayerPlay)->GetPlayState(bqPlayerPlay, &ps);
        //LOGSOUND("Current play state = %d, start=%d buf=%d,%d", ps, start_threshold, dst_head, dst_tail);
        if (bqPlayerObject && ps != SL_PLAYSTATE_PLAYING &&
            dst_tail - dst_head >= start_threshold) {
            LOGSOUNDI("Start playing. Current=%d, start=%d buf=%d,%d", ps, start_threshold, dst_head, dst_tail);

            (*bqPlayerPlay)->SetPlayState(bqPlayerPlay,
                                          SL_PLAYSTATE_STOPPED);
            fillSilent();
            (*bqPlayerPlay)->SetPlayState(bqPlayerPlay,
                                          SL_PLAYSTATE_PLAYING);
        }

        return 0;
    }

    // Stop until the buffer is fed.
    void Stop(){
        LOGSOUNDI("Stopping.");
        dst_head = dst_tail = 0;
        (*bqPlayerPlay)->SetPlayState(bqPlayerPlay,
                                      SL_PLAYSTATE_STOPPED);
    }
private:

    void fillSilent() {
        LOGSOUND("Fill buffer with silent.");

        SLresult res = (*bqPlayerBufferQueue)->Enqueue(
                bqPlayerBufferQueue,
                silent_buf,
                DST_BUF_SAMPLE * dst_byte_per_frame);
        if (SL_RESULT_SUCCESS != res) {
            LOGSOUND("Error on Enqueue silent buffer. Code=%d", res);
        }
    }

    static void callback(SLAndroidSimpleBufferQueueItf, void *arg) {
        SoundPlayer *self = (SoundPlayer *) arg;

        self->callback_();
    }

    void callback_() {
        SLAndroidSimpleBufferQueueState s;
        (*bqPlayerBufferQueue)->GetState(bqPlayerBufferQueue, &s);
        //LOGSOUND("Sound buffer callback is called. count=%d index=%d buf:%d,%d start:%d", s.count, s.index, dst_head, dst_tail,
        //         start_threshold);

        if (dst_head == dst_tail) {
            fillSilent();
            return;
        }

        uint8_t *dst_data = dst_data_list[dst_head % BUF_COUNT];

        SLresult res = (*bqPlayerBufferQueue)->Enqueue(
                bqPlayerBufferQueue,
                dst_data,
                DST_BUF_SAMPLE * dst_byte_per_frame);
        if (SL_RESULT_SUCCESS != res) {
            LOGSOUND("Error on Enqueue. Code=%d", res);
        }
        dst_head++;
        return;
    }

    int start_threshold = START_THRESHOLD;
    int discard_frame = 0;

    SLObjectItf engineObject = NULL;
    SLEngineItf engineEngine = NULL;
    SLObjectItf outputMixObject = NULL;
    SLObjectItf bqPlayerObject = NULL;
    SLPlayItf bqPlayerPlay = NULL;
    SLAndroidSimpleBufferQueueItf bqPlayerBufferQueue = NULL;
    SLVolumeItf bqPlayerVolume = NULL;

    uint8_t *dst_data_list[BUF_COUNT];
    int dst_head;
    int dst_tail;
    int dst_nb_channels;
    int dst_end;
    int dst_byte_per_frame;

    uint8_t *silent_buf;

    int64_t audio_frame_count;
};

#endif //ALVRCLIENT_SOUND_H
