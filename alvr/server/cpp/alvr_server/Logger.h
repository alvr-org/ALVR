#pragma once

#include "ALVR-common/exception.h"

Exception MakeException(const char *format, ...);

void Error(const char *format, ...);
void Warn(const char *format, ...);
void Info(const char *format, ...);
void Debug(const char *format, ...);
void LogPeriod(const char *tag, const char *format, ...);
