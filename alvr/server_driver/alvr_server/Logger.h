#pragma once

#include <iostream>
#include <fstream>
#include "exception.h"
#include <wrl.h>

#include "Utils.h"
#include "driverlog.h"

void InitCrashHandler();

void OpenLog(const char *fileName);
void CloseLog();

void LogDriver(const char* pFormat, ...);

void Log(const wchar_t *pFormat, ...);
void Log(const char *pFormat, ...);
//void LogException(const wchar_t *format, ...);
void LogException(const char *format, ...);
//void FatalLog(const wchar_t *format, ...);
void FatalLog(const char *format, ...);
void LogHR(const std::wstring &message, HRESULT hr);
void ThrowHR(const std::wstring &message, HRESULT hr);

Exception MakeException(const wchar_t *format, ...);
Exception MakeException(const char *format, ...);

void FlushLog();