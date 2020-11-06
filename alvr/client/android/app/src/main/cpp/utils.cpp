#include "utils.h"
#include <jni.h>
#include "bindings.h"

int gGeneralLogLevel = ANDROID_LOG_INFO;
int gSoundLogLevel = ANDROID_LOG_INFO;
int gSocketLogLevel = ANDROID_LOG_INFO;
bool gDisableExtraLatencyMode = false;
long gDebugFlags = 0;


enum DEBUG_FLAGS {
    DEBUG_FLAGS_ENABLE_FRAME_LOG = 1 << 0,
    DEBUG_FLAGS_ENABLE_GENERAL_LOG = 1 << 1,
    DEBUG_FLAGS_ENABLE_SOUND_LOG = 1 << 2,
    DEBUG_FLAGS_ENABLE_SOCKET_LOG = 1 << 3,
    DEBUG_FLAGS_DISABLE_EXTRA_LATENCY_MODE = 1 << 4,
};


bool gEnableFrameLog = false;

//extern "C"
//JNIEXPORT void JNICALL
//Java_com_polygraphene_alvr_Utils_setFrameLogEnabled(JNIEnv *env, jclass type, jlong debugFlags) {
//    gEnableFrameLog = static_cast<bool>(debugFlags & DEBUG_FLAGS_ENABLE_FRAME_LOG);
//
//    gGeneralLogLevel = (debugFlags & DEBUG_FLAGS_ENABLE_GENERAL_LOG) ?
//                       ANDROID_LOG_VERBOSE : ANDROID_LOG_INFO ;
//    gSoundLogLevel = (debugFlags & DEBUG_FLAGS_ENABLE_SOUND_LOG) ?
//                       ANDROID_LOG_VERBOSE : ANDROID_LOG_INFO ;
//    gSocketLogLevel = (debugFlags & DEBUG_FLAGS_ENABLE_SOCKET_LOG) ?
//                       ANDROID_LOG_VERBOSE : ANDROID_LOG_INFO ;
//    gDisableExtraLatencyMode = (debugFlags & DEBUG_FLAGS_DISABLE_EXTRA_LATENCY_MODE) != 0;
//    gDebugFlags = debugFlags;
//}

void setFrameLogEnabled(long long debugFlags) {
    gEnableFrameLog = static_cast<bool>(debugFlags & DEBUG_FLAGS_ENABLE_FRAME_LOG);

    gGeneralLogLevel = (debugFlags & DEBUG_FLAGS_ENABLE_GENERAL_LOG) ?
                       ANDROID_LOG_VERBOSE : ANDROID_LOG_INFO ;
    gSoundLogLevel = (debugFlags & DEBUG_FLAGS_ENABLE_SOUND_LOG) ?
                       ANDROID_LOG_VERBOSE : ANDROID_LOG_INFO ;
    gSocketLogLevel = (debugFlags & DEBUG_FLAGS_ENABLE_SOCKET_LOG) ?
                       ANDROID_LOG_VERBOSE : ANDROID_LOG_INFO ;
    gDisableExtraLatencyMode = (debugFlags & DEBUG_FLAGS_DISABLE_EXTRA_LATENCY_MODE) != 0;
    gDebugFlags = debugFlags;
}