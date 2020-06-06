#pragma once

#include <iostream>
#include <fstream>
#include "exception.h"
#include <wrl.h>

#include "Utils.h"
#include "driverlog.h"
#include "bindings.h"

// deprecated
void LogDriver(const char* pFormat, ...);
// deprecated
void Log(const char *pFormat, ...);
// deprecated
void LogException(const char *format, ...);
// deprecated
void FatalLog(const char *format, ...);

Exception MakeException(const char *format, ...);

void Error(const char *format, ...);
void Warn(const char *format, ...);
void Info(const char *format, ...);
void Debug(const char *format, ...);