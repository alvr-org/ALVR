#pragma once

#include <iostream>
#include <fstream>
#include "exception.h"

void InitCrashHandler();

void OpenLog(const char *fileName);

void Log(const char *pFormat, ...);
void LogException(const char *format, ...);
void FatalLog(const char *format, ...);

Exception MakeException(const char *format, ...);

void FlushLog();