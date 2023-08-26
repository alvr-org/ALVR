#ifndef ALVRCLIENT_UTILS_H
#define ALVRCLIENT_UTILS_H

#include <GLES3/gl3.h>
#include <android/log.h>
#include <string>

#define LOGI(...)                                                                                  \
    do {                                                                                           \
        __android_log_print(ANDROID_LOG_INFO, "[ALVR Native]", __VA_ARGS__);                         \
    } while (false)
#define LOGE(...)                                                                                  \
    do {                                                                                           \
        __android_log_print(ANDROID_LOG_ERROR, "[ALVR Native]", __VA_ARGS__);                        \
    } while (false)
#define LOGD(...)                                                                                  \
    do {                                                                                           \
        __android_log_print(ANDROID_LOG_DEBUG, "[ALVR Native]", __VA_ARGS__);                        \
    } while (false)
#define LOGV(...)                                                                                  \
    do {                                                                                           \
        __android_log_print(ANDROID_LOG_VERBOSE, "[ALVR Native]", __VA_ARGS__);                        \
    } while (false)

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

[[maybe_unused]] static void GLCheckErrors(const char *file, int line) {
    GLenum error = glGetError();
    if (error == GL_NO_ERROR) {
        return;
    }
    while (error != GL_NO_ERROR) {
        LOGE("GL error on %s : %d: %s", file, line, GlErrorString(error));
        error = glGetError();
    }
    abort();
}

#define GL(func)                                                                                   \
    func;                                                                                          \
    GLCheckErrors(__FILE__, __LINE__)

// https://stackoverflow.com/a/26221725
template <typename... Args> std::string string_format(const std::string &format, Args... args) {
    size_t size = snprintf(nullptr, 0, format.c_str(), args...) + 1; // Extra space for '\0'
    std::unique_ptr<char[]> buf(new char[size]);
    snprintf(buf.get(), size, format.c_str(), args...);
    return std::string(buf.get(), buf.get() + size - 1); // We don't want the '\0' inside
}

#endif // ALVRCLIENT_UTILS_H
