#include "Logger.h"

#include <cstdarg>

#include "driverlog.h"
#include "bindings.h"

void _log(const char *format, va_list args, void (*logFn)(const char *), bool driverLog = false)
{
	char buf[1024];
	int count = vsnprintf(buf, sizeof(buf), format, args);
	if (count > (int)sizeof(buf))
		count = (int)sizeof(buf);
	if (count > 0 && buf[count - 1] == '\n')
		buf[count - 1] = '\0';

	logFn(buf);

	//TODO: driver logger should concider current log level
#ifndef ALVR_DEBUG_LOG
	if (driverLog)
#endif
		DriverLog(buf);
}

Exception MakeException(const char *format, ...)
{
	va_list args;
	va_start(args, format);
	Exception e = FormatExceptionV(format, args);
	va_end(args);

	return e;
}

void Error(const char *format, ...)
{
	va_list args;
	va_start(args, format);
	_log(format, args, LogError, true);
	va_end(args);
}

void Warn(const char *format, ...)
{
	va_list args;
	va_start(args, format);
	_log(format, args, LogWarn, true);
	va_end(args);
}

void Info(const char *format, ...)
{
	va_list args;
	va_start(args, format);
	// Don't log to SteamVR/writing to file for info level, this is mostly statistics info
	_log(format, args, LogInfo);
	va_end(args);
}

void Debug(const char *format, ...)
{
// Use our define instead of _DEBUG - see build.rs for details.
#ifdef ALVR_DEBUG_LOG
	va_list args;
	va_start(args, format);
	_log(format, args, LogDebug);
	va_end(args);
#else
	(void)format;
#endif
}

void LogPeriod(const char *tag, const char *format, ...)
{
	va_list args;
	va_start(args, format);

	char buf[1024];
	int count = vsnprintf(buf, sizeof(buf), format, args);
	if (count > (int)sizeof(buf))
		count = (int)sizeof(buf);
	if (count > 0 && buf[count - 1] == '\n')
		buf[count - 1] = '\0';

	LogPeriodically(tag, buf);

	va_end(args);
}
