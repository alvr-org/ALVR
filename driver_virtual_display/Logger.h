/*
* Copyright 2017-2018 NVIDIA Corporation.  All rights reserved.
*
* Please refer to the NVIDIA end user license agreement (EULA) associated
* with this source code for terms and conditions that govern your use of
* this software. Any use, reproduction, disclosure, or distribution of
* this software and related documentation outside the terms of the EULA
* is strictly prohibited.
*
*/

#pragma once

#include <iostream>
#include <fstream>
#include <string>
#include <sstream>
#include <mutex>
#include <time.h>
#include <locale>
#include <codecvt>

extern std::string g_DebugOutputDir;

static std::ofstream ofs;
static bool OpenFailed = false;

inline void OpenLog(const char *fileName) {
	ofs.open(fileName);
}

inline void Log(const char *pFormat, ...)
{
	FILETIME ft;
	SYSTEMTIME st2, st;
	uint64_t q;
	
	if (!ofs.is_open()) {
		return;
	}

	GetSystemTimeAsFileTime(&ft);
	FileTimeToSystemTime(&ft, &st2);
	SystemTimeToTzSpecificLocalTime(NULL, &st2, &st);

	q = (((uint64_t)ft.dwHighDateTime) << 32) | ft.dwLowDateTime;
	q /= 10;

	char buf[100];
	snprintf(buf, sizeof(buf), "[%02d:%02d:%02d.%03lld %03lld] ",
		st.wHour, st.wMinute, st.wSecond, q / 1000 % 1000, q % 1000);

	va_list args;
	va_start(args, pFormat);
	char buf2[10000];
	vsnprintf(buf2, sizeof(buf2), pFormat, args);
	va_end(args);

	ofs << buf << buf2 << std::endl;
}
