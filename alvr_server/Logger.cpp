#include <Windows.h>
#include <string>
#include <sstream>
#include <mutex>
#include <time.h>
#include <locale>
#include <codecvt>
#include <list>
#include "Logger.h"
#include "Utils.h"

static const char *APP_NAME = "ALVR Server";
static const int STARTUP_LOG_SIZE = 500;
static const int TAIL_LOG_SIZE = 500;

extern HINSTANCE g_hInstance;

static std::ofstream ofs;
static bool Opened = false;
static bool OpenFailed = false;
static uint64_t lastRefresh = 0;
static std::string lastException;
static std::list<std::string> startupLog;
static std::list<std::string> tailLog[2];
static int currentLog = 0;

static std::wstring GetCrashReportPath() {
	wchar_t cpath[10000];
	GetModuleFileNameW(g_hInstance, cpath, sizeof(cpath) / sizeof(wchar_t));
	wchar_t *p = wcsrchr(cpath, L'\\');
	*p = L'\0';
	wcsncat_s(cpath, L"\\..\\..\\..\\CrashReport.exe", sizeof(cpath) / sizeof(wchar_t));
	return cpath;
}

static void OutputCrashLog() {
	wchar_t cpath[10000], logPath[11000];
	FILE *fp;
	wchar_t *p;
	SYSTEMTIME st;

	GetModuleFileNameW(g_hInstance, cpath, sizeof(cpath) / sizeof(wchar_t));
	p = wcsrchr(cpath, L'\\');
	*p = L'\0';

	GetLocalTime(&st);

	_snwprintf_s(logPath, sizeof(logPath), L"%s\\..\\..\\..\\ALVR_CrashLog_%04d%02d%02d_%02d%02d%02d.log",
		cpath, st.wYear, st.wMonth, st.wDay,
		st.wHour, st.wMinute, st.wSecond);

	if (_wfopen_s(&fp, logPath, L"w")) {
		return;
	}

	fprintf(fp, "Exception: %s\n", lastException.c_str());
	fprintf(fp, "========== Startup Log ==========\n");
	for (auto line : startupLog) {
		fprintf(fp, "%s\n", line.c_str());
	}
	fprintf(fp, "========== Tail Log 1 ==========\n");
	for (auto line : tailLog[1 - currentLog]) {
		fprintf(fp, "%s\n", line.c_str());
	}
	fprintf(fp, "========== Tail Log 2 ==========\n");
	for (auto line : tailLog[currentLog]) {
		fprintf(fp, "%s\n", line.c_str());
	}
	fclose(fp);
}

static void ReportError() {
	FlushLog();

	OutputCrashLog();

	ShellExecuteW(NULL, L"", GetCrashReportPath().c_str(), (L"\"" + ToWstring(lastException) + L"\"").c_str(), L"", SW_SHOWNORMAL);
}

static LONG WINAPI MyUnhandledExceptionFilter(PEXCEPTION_POINTERS pExceptionPtrs)
{
	Log("Unhandled Exception!!! %X %p", pExceptionPtrs->ExceptionRecord->ExceptionCode, pExceptionPtrs->ExceptionRecord->ExceptionAddress);
	ReportError();
	return EXCEPTION_EXECUTE_HANDLER;
}

void InitCrashHandler() {
	SetUnhandledExceptionFilter(MyUnhandledExceptionFilter);
}

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

	GetSystemTimeAsFileTime(&ft);
	FileTimeToSystemTime(&ft, &st2);
	SystemTimeToTzSpecificLocalTime(NULL, &st2, &st);

	q = (((uint64_t)ft.dwHighDateTime) << 32) | ft.dwLowDateTime;
	q /= 10;

	char buf[100];
	snprintf(buf, sizeof(buf), "[%02d:%02d:%02d.%03lld %03lld] ",
		st.wHour, st.wMinute, st.wSecond, q / 1000 % 1000, q % 1000);

	std::string line = std::string(buf) + str;
	// Store log into list for crash log.
	if (startupLog.size() < STARTUP_LOG_SIZE) {
		startupLog.push_back(line);
	}
	else {
		if (tailLog[currentLog].size() < TAIL_LOG_SIZE) {
			tailLog[currentLog].push_back(line);
		}
		else {
			currentLog = 1 - currentLog;
			tailLog[currentLog].clear();
			tailLog[currentLog].push_back(line);
		}
	}

	if (!ofs.is_open()) {
		return;
	}

	ofs << buf << str << std::endl;

	if (lastRefresh / 1000000 != q / 1000000) {
		lastRefresh = q;
		ofs.flush();
	}
}

void Log(const char *format, ...)
{
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

	lastException = buf2;
	ReportError();
}

Exception MakeException(const char *format, ...) {
	va_list args;
	va_start(args, format);
	char buf[10000];
	vsnprintf(buf, sizeof(buf), format, args);
	va_end(args);

	LogS(buf);
	lastException = buf;
	FlushLog();

	return Exception(buf);
}

void FlushLog() {
	if (!ofs.is_open()) {
		return;
	}
	ofs.flush();
}