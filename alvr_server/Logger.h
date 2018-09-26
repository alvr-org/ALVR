#pragma once

#include <iostream>
#include <fstream>
#include "exception.h"

void InitCrashHandler();

void OpenLog(const char *fileName);
void CloseLog();

void Log(const wchar_t *pFormat, ...);
void Log(const char *pFormat, ...);
void LogException(const wchar_t *format, ...);
void LogException(const char *format, ...);
void FatalLog(const wchar_t *format, ...);
void FatalLog(const char *format, ...);

Exception MakeException(const wchar_t *format, ...);
Exception MakeException(const char *format, ...);

void FlushLog();