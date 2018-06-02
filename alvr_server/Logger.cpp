#include <Windows.h>
#include <string>
#include <sstream>
#include <mutex>
#include <time.h>
#include <locale>
#include <codecvt>
#include "Logger.h"

static const char *APP_NAME = "ALVR Server";

extern std::string g_DebugOutputDir;

static std::ofstream ofs;
static bool Opened = false;
static bool OpenFailed = false;
static uint64_t lastRefresh = 0;

void OpenLog(const char *fileName) {
	if (!Opened) {
		ofs.open(fileName);
	}
	Opened = true;
}

void LogS(const char *str)
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

	ofs << buf << str << std::endl;

	if (lastRefresh / 1000000 != q / 1000000) {
		lastRefresh = q;
		ofs.flush();
	}
}

void Log(const char *format, ...)
{
	if (!ofs.is_open()) {
		return;
	}

	va_list args;
	va_start(args, format);
	char buf2[10000];
	vsnprintf(buf2, sizeof(buf2), format, args);
	va_end(args);

	LogS(buf2);
}

void FatalLog(const char *format, ...) {
	va_list args;
	va_start(args, format);
	char buf2[10000];
	vsnprintf(buf2, sizeof(buf2), format, args);
	va_end(args);

	LogS(buf2);

	MessageBoxA(NULL, buf2, APP_NAME, MB_OK);
}