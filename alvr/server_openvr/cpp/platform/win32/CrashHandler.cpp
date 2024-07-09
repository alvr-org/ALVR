#include "../../alvr_server/bindings.h"

#include "../../alvr_server/Logger.h"
#include "../../shared/backward.hpp"
#include <Windows.h>
#include <ostream>

static LONG WINAPI handler(PEXCEPTION_POINTERS ptrs) {
    backward::StackTrace stacktrace;
    backward::Printer printer;
    std::ostringstream stream;

    stacktrace.load_from(ptrs->ExceptionRecord->ExceptionAddress);
    printer.print(stacktrace, stream);
    std::string str = stream.str();
    Error("Unhandled exception: %X\n%s", ptrs->ExceptionRecord->ExceptionCode, str.c_str());

    Sleep(2000);

    return EXCEPTION_EXECUTE_HANDLER;
}

void HookCrashHandler() { SetUnhandledExceptionFilter(handler); }
