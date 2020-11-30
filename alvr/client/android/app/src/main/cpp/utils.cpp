#include "utils.h"
#include <jni.h>
#include "bindings.h"

int gGeneralLogLevel = ANDROID_LOG_INFO;
int gSoundLogLevel = ANDROID_LOG_INFO;
int gSocketLogLevel = ANDROID_LOG_INFO;
bool gDisableExtraLatencyMode = false;
long gDebugFlags = 0;