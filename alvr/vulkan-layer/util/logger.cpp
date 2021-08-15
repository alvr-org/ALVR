#include <cstdarg>
#include <cstdio>
#include <cstdlib>

void _log(const char *format, va_list args, bool err) {
    vfprintf(err ? stderr : stdout, format, args);
}

void Error(const char *format, ...) {
    va_list args;
    va_start(args, format);
    _log(format, args, true);
    va_end(args);
}

void Warn(const char *format, ...) {
    va_list args;
    va_start(args, format);
    _log(format, args, true);
    va_end(args);
}

void Info(const char *format, ...) {
    va_list args;
    va_start(args, format);
    _log(format, args, false);
    va_end(args);
}

void Debug(const char *format, ...) {
    if (getenv("ALVR_LOG_DEBUG") == NULL) return;
    va_list args;
    va_start(args, format);
    _log(format, args, true);
    va_end(args);
}
