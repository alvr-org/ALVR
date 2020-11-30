#ifndef ALVRCLIENT_UTILS_H
#define ALVRCLIENT_UTILS_H

#include <stdint.h>
#include <math.h>
#include <time.h>
#include <pthread.h>
#include <android/log.h>
#include <string>
#include <VrApi_Types.h>
#include <GLES3/gl3.h>

//
// Logging
//

// Defined in utils.cpp. 0 means no log output.
extern int gGeneralLogLevel;
extern int gSoundLogLevel;
extern int gSocketLogLevel;
extern bool gDisableExtraLatencyMode;

#define LOG(...) do { if(gGeneralLogLevel <= ANDROID_LOG_VERBOSE){__android_log_print(ANDROID_LOG_VERBOSE, "ALVR Native", __VA_ARGS__);} } while (false)
#define LOGI(...) do { if(gGeneralLogLevel <= ANDROID_LOG_INFO){__android_log_print(ANDROID_LOG_INFO, "ALVR Native", __VA_ARGS__);} } while (false)
#define LOGE(...) do { if(gGeneralLogLevel <= ANDROID_LOG_ERROR){__android_log_print(ANDROID_LOG_ERROR, "ALVR Native", __VA_ARGS__);} } while (false)

#define LOGSOUND(...) do { if(gSoundLogLevel <= ANDROID_LOG_VERBOSE){__android_log_print(ANDROID_LOG_VERBOSE, "ALVR Sound", __VA_ARGS__);} } while (false)
#define LOGSOUNDI(...) do { if(gSoundLogLevel <= ANDROID_LOG_INFO){__android_log_print(ANDROID_LOG_INFO, "ALVR Sound", __VA_ARGS__);} } while (false)

#define LOGSOCKET(...) do { if(gSocketLogLevel <= ANDROID_LOG_VERBOSE){__android_log_print(ANDROID_LOG_VERBOSE, "ALVR Socket", __VA_ARGS__);} } while (false)
#define LOGSOCKETI(...) do { if(gSocketLogLevel <= ANDROID_LOG_INFO){__android_log_print(ANDROID_LOG_INFO, "ALVR Socket", __VA_ARGS__);} } while (false)

static const int64_t USECS_IN_SEC = 1000 * 1000;

const bool gEnableFrameLog = false;

inline void FrameLog(uint64_t frameIndex, const char *format, ...)
{
    if (!gEnableFrameLog) {
        return;
    }

    char buf[10000];

    va_list args;
    va_start(args, format);
    vsnprintf(buf, sizeof(buf), format, args);
    va_end(args);

    __android_log_print(ANDROID_LOG_VERBOSE, "FrameTracking", "[Frame %lu] %s", frameIndex, buf);
}

//
// GL Logging
//

#define CHECK_GL_ERRORS 1
#ifdef CHECK_GL_ERRORS

static const char *GlErrorString(GLenum error) {
    switch (error) {
        case GL_NO_ERROR:
            return "GL_NO_ERROR";
        case GL_INVALID_ENUM:
            return "GL_INVALID_ENUM";
        case GL_INVALID_VALUE:
            return "GL_INVALID_VALUE";
        case GL_INVALID_OPERATION:
            return "GL_INVALID_OPERATION";
        case GL_INVALID_FRAMEBUFFER_OPERATION:
            return "GL_INVALID_FRAMEBUFFER_OPERATION";
        case GL_OUT_OF_MEMORY:
            return "GL_OUT_OF_MEMORY";
        default:
            return "unknown";
    }
}

[[maybe_unused]] static void GLCheckErrors(const char* file, int line) {
    const GLenum error = glGetError();
    if (error == GL_NO_ERROR) {
        return;
    }
    LOGE("GL error on %s : %d: %s", file, line, GlErrorString(error));
    abort();
}

#define GL(func)        func; GLCheckErrors(__FILE__, __LINE__ )
#else // CHECK_GL_ERRORS
#define GL(func)        func;
#endif // CHECK_GL_ERRORS

//
// Utility
//

inline uint64_t getTimestampUs(){
    timeval tv;
    gettimeofday(&tv, NULL);

    uint64_t Current = (uint64_t)tv.tv_sec * 1000 * 1000 + tv.tv_usec;
    return Current;
}

//
// Utility
//

/// Integer version of ovrRectf
typedef struct Recti_
{
    int x;
    int y;
    int width;
    int height;
} Recti;

inline std::string GetStringFromJNIString(JNIEnv *env, jstring string){
    const char *buf = env->GetStringUTFChars(string, 0);
    std::string ret = buf;
    env->ReleaseStringUTFChars(string, buf);

    return ret;
}

inline double GetTimeInSeconds() {
    struct timespec now;
    clock_gettime(CLOCK_MONOTONIC, &now);
    return (now.tv_sec * 1e9 + now.tv_nsec) * 0.000000001;
}

// https://stackoverflow.com/a/26221725
template<typename ... Args>
std::string string_format( const std::string& format, Args ... args )
{
    size_t size = snprintf( nullptr, 0, format.c_str(), args ... ) + 1; // Extra space for '\0'
    std::unique_ptr<char[]> buf( new char[ size ] );
    snprintf( buf.get(), size, format.c_str(), args ... );
    return std::string( buf.get(), buf.get() + size - 1 ); // We don't want the '\0' inside
}

#endif //ALVRCLIENT_UTILS_H
