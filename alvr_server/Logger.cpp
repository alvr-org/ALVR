#include <Windows.h>
#include <string>
#include <mutex>
#include <time.h>
#include <share.h>
#include <list>
#include "Logger.h"
#include "Utils.h"
#include "openvr-utils\ipctools.h"
#include "exception.h"
#include "common-utils.h"

#pragma comment(lib, "psapi.lib")
#pragma comment(lib, "dbghelp.lib")

// Some versions of imagehlp.dll lack the proper packing directives themselves
// so we need to do it.
#pragma pack( push, before_imagehlp, 8 )
#include <imagehlp.h>
#pragma pack( pop, before_imagehlp )

static const char *APP_NAME = "ALVR Server";
static const int STARTUP_LOG_SIZE = 500;
static const int TAIL_LOG_SIZE = 500;

extern HINSTANCE gInstance;

static FILE *gLogFile = nullptr;
static bool gOpened = false;
static bool gOpenFailed = false;
static uint64_t gLastRefresh = 0;
static std::wstring gLastException;
static IPCMutex gMutex(NULL);
static std::list<std::wstring> gStartupLog;
static std::list<std::wstring> gTailLog[2];
static int gCurrentLog = 0;

static std::wstring GetCrashReportPath() {
	wchar_t cpath[10000];
	GetModuleFileNameW(gInstance, cpath, sizeof(cpath) / sizeof(wchar_t));
	wchar_t *p = wcsrchr(cpath, L'\\');
	*p = L'\0';
	wcsncat_s(cpath, L"\\..\\..\\..\\CrashReport.exe", sizeof(cpath) / sizeof(wchar_t));
	return cpath;
}

static void GenerateExceptionInfo(wchar_t *logPath, PEXCEPTION_POINTERS pExceptionPtrs) {
	FILE *fp;

	if (_wfopen_s(&fp, logPath, L"a, ccs=UTF-8")) {
		return;
	}
	HANDLE process = GetCurrentProcess();
	DWORD64 address = (DWORD64)pExceptionPtrs->ExceptionRecord->ExceptionAddress;

	const int max_name_len = 1024;
	IMAGEHLP_SYMBOL64 *sym = (IMAGEHLP_SYMBOL64 *)malloc(sizeof(IMAGEHLP_SYMBOL64) + max_name_len);
	IMAGEHLP_LINE64 line = { 0 };
	DWORD offset_from_symbol;

	line.SizeOfStruct = sizeof line;

	memset(sym, 0, sizeof(IMAGEHLP_SYMBOL64) + max_name_len);
	sym->SizeOfStruct = sizeof(IMAGEHLP_SYMBOL64);
	sym->MaxNameLength = max_name_len;
	DWORD64 displacement;

	SymInitialize(process, NULL, true);

	BOOL ret = SymGetSymFromAddr64(process, address, &displacement, sym);
	BOOL ret2 = SymGetLineFromAddr64(process, address, &offset_from_symbol, &line);
	
	fwprintf(fp, L"========== Exception info start ==========\n");
	fwprintf(fp, L"ExceptionCode=%X Address=%016llX ThreadId=%d\n", pExceptionPtrs->ExceptionRecord->ExceptionCode, address, GetCurrentThreadId());
	if (ret) {
		std::vector<char> und_name(max_name_len);
		UnDecorateSymbolName(sym->Name, &und_name[0], max_name_len, UNDNAME_COMPLETE);

		fwprintf(fp, L"%hs(%hs) +%llu\n", sym->Name, &und_name[0], displacement);
	}
	if (ret2) {
		fwprintf(fp, L"%hs:%d\n", line.FileName, line.LineNumber);
	}
	fwprintf(fp, L"========== Exception info end ==========\n");
	SymCleanup(process);
	free(sym);
	fclose(fp);
}

static void OutputCrashLog(PEXCEPTION_POINTERS pExceptionPtrs) {
	wchar_t cpath[10000], logPath[11000];
	FILE *fp;
	wchar_t *p;
	SYSTEMTIME st;

	GetModuleFileNameW(gInstance, cpath, sizeof(cpath) / sizeof(wchar_t));
	p = wcsrchr(cpath, L'\\');
	*p = L'\0';

	GetLocalTime(&st);

	_snwprintf_s(logPath, sizeof(logPath), L"%s\\..\\..\\..\\logs\\ALVR_CrashLog_%04d%02d%02d_%02d%02d%02d.log",
		cpath, st.wYear, st.wMonth, st.wDay,
		st.wHour, st.wMinute, st.wSecond);

	if (_wfopen_s(&fp, logPath, L"w, ccs=UTF-8")) {
		return;
	}

	fwprintf(fp, L"Exception: %s\n", gLastException.c_str());
	fwprintf(fp, L"OSVer: %s\n", GetWindowsOSVersion().c_str());
	fwprintf(fp, L"Module: %p\n", gInstance);
	fwprintf(fp, L"========== Startup Log ==========\n");

	gMutex.Wait();
	for (auto line : gStartupLog) {
		fputws(line.c_str(), fp);
		fputws(L"\n", fp);
	}
	fwprintf(fp, L"========== Tail Log 1 ==========\n");
	for (auto line : gTailLog[1 - gCurrentLog]) {
		fputws(line.c_str(), fp);
		fputws(L"\n", fp);
	}
	fwprintf(fp, L"========== Tail Log 2 ==========\n");
	for (auto line : gTailLog[gCurrentLog]) {
		fputws(line.c_str(), fp);
		fputws(L"\n", fp);
	}
	gMutex.Release();

	fclose(fp);

	if (pExceptionPtrs != NULL) {
		GenerateExceptionInfo(logPath, pExceptionPtrs);
	}
}

static void ReportError(PEXCEPTION_POINTERS pExceptionPtrs) {
	FlushLog();

	OutputCrashLog(pExceptionPtrs);

	ShellExecuteW(NULL, L"", GetCrashReportPath().c_str(), (L"\"" + gLastException + L"\"").c_str(), L"", SW_SHOWNORMAL);
}

static LONG WINAPI MyUnhandledExceptionFilter(PEXCEPTION_POINTERS pExceptionPtrs)
{
	LogException("Unhandled Exception.\nExceptionCode=%X\nAddress=%p (%p + %p)", pExceptionPtrs->ExceptionRecord->ExceptionCode, pExceptionPtrs->ExceptionRecord->ExceptionAddress
		, gInstance, (char*)pExceptionPtrs->ExceptionRecord->ExceptionAddress - (char*)gInstance);
	ReportError(pExceptionPtrs);
	return EXCEPTION_EXECUTE_HANDLER;
}

void InitCrashHandler() {
	SetUnhandledExceptionFilter(MyUnhandledExceptionFilter);
}

void OpenLog(const char *fileName) {
	if (!gOpened) {
		// ccs=UTF-8 converts wchar_t to UTF-8 on output.
		// _SH_DENYNO allows other process read log.
		gLogFile = _fsopen(fileName, "w, ccs=UTF-8", _SH_DENYNO);
	}
	gOpened = true;
}

void CloseLog() {
	if (gLogFile != nullptr) {
		fclose(gLogFile);
		gLogFile = nullptr;
	}
}

void LogS(const wchar_t *str)
{
	FILETIME ft;
	SYSTEMTIME st2, st;
	uint64_t q;

	GetSystemTimeAsFileTime(&ft);
	FileTimeToSystemTime(&ft, &st2);
	SystemTimeToTzSpecificLocalTime(NULL, &st2, &st);

	q = (((uint64_t)ft.dwHighDateTime) << 32) | ft.dwLowDateTime;
	q /= 10;

	wchar_t buf[100];
	_snwprintf_s(buf, sizeof(buf) / sizeof(buf[0]), L"[%02d:%02d:%02d.%03lld %03lld] ",
		st.wHour, st.wMinute, st.wSecond, q / 1000 % 1000, q % 1000);

	std::wstring line = std::wstring(buf) + str;

	gMutex.Wait();
	// Store log into list for crash log.
	if (gStartupLog.size() < STARTUP_LOG_SIZE) {
		gStartupLog.push_back(line);
	}
	else {
		if (gTailLog[gCurrentLog].size() < TAIL_LOG_SIZE) {
			gTailLog[gCurrentLog].push_back(line);
		}
		else {
			gCurrentLog = 1 - gCurrentLog;
			gTailLog[gCurrentLog].clear();
			gTailLog[gCurrentLog].push_back(line);
		}
	}
	gMutex.Release();

	if (gLogFile == nullptr) {
		return;
	}

	gMutex.Wait();
	fputws(line.c_str(), gLogFile);
	fputws(L"\n", gLogFile);
	gMutex.Release();

	if (gLastRefresh / 1000000 != q / 1000000) {
		gLastRefresh = q;
		fflush(gLogFile);
	}
}

void LogS(const char *str)
{
	LogS(ToWstring(str).c_str());
}

void LogV(const wchar_t *format, va_list args, std::wstring *out) {
	wchar_t buf[10000];
	_vsnwprintf_s(buf, sizeof(buf) / sizeof(buf[0]), format, args);

	LogS(buf);
	if (out != nullptr) {
		*out = buf;
	}
}

void LogV(const char *format, va_list args, std::wstring *out) {
	char buf[10000];
	vsnprintf(buf, sizeof(buf), format, args);

	LogS(buf);
	if (out != nullptr) {
		*out = ToWstring(buf);
	}
}

void Log(const wchar_t *format, ...)
{
	va_list args;
	va_start(args, format);
	LogV(format, args, nullptr);
	va_end(args);
}

void Log(const char *format, ...)
{
	va_list args;
	va_start(args, format);
	LogV(format, args, nullptr);
	va_end(args);
}

void LogException(const wchar_t *format, ...)
{
	va_list args;
	va_start(args, format);
	LogV(format, args, &gLastException);
	va_end(args);
}

void LogException(const char *format, ...)
{
	va_list args;
	va_start(args, format);
	LogV(format, args, &gLastException);
	va_end(args);
}

void FatalLog(const wchar_t *format, ...) {
	va_list args;
	va_start(args, format);
	LogV(format, args, &gLastException);
	va_end(args);

	ReportError(NULL);
}

void FatalLog(const char *format, ...) {
	va_list args;
	va_start(args, format);
	LogV(format, args, &gLastException);
	va_end(args);

	ReportError(NULL);
}

Exception MakeException(const wchar_t *format, ...) {
	va_list args;
	va_start(args, format);
	Exception e = FormatExceptionV(format, args);
	va_end(args);

	LogS(e.what());
	gLastException = e.what();
	FlushLog();

	return e;
}

Exception MakeException(const char *format, ...) {
	va_list args;
	va_start(args, format);
	Exception e = FormatExceptionV(format, args);
	va_end(args);

	LogS(e.what());
	gLastException = e.what();
	FlushLog();

	return e;
}

void FlushLog() {
	if (gLogFile == nullptr) {
		return;
	}
	fflush(gLogFile);
}