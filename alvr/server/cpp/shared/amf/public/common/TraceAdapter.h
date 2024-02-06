//
// Notice Regarding Standards.  AMD does not provide a license or sublicense to
// any Intellectual Property Rights relating to any standards, including but not
// limited to any audio and/or video codec technologies such as MPEG-2, MPEG-4;
// AVC/H.264; HEVC/H.265; AAC decode/FFMPEG; AAC encode/FFMPEG; VC-1; and MP3
// (collectively, the "Media Technologies"). For clarity, you will pay any
// royalties due for such third party technologies, which may include the Media
// Technologies that are owed as a result of AMD providing the Software to you.
//
// MIT license
//
// Copyright (c) 2018 Advanced Micro Devices, Inc. All rights reserved.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.  IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.
//
///-------------------------------------------------------------------------
///  @file   TraceAdapter.h
///  @brief  AMFTrace interface
///-------------------------------------------------------------------------
#ifndef AMF_TraceAdapter_h
#define AMF_TraceAdapter_h
#pragma once

#include "../include/core/Debug.h"
#include "../include/core/Trace.h"
#include "../include/core/Result.h"
#include "../common/AMFFactory.h"
#include "AMFSTL.h"

#ifndef WIN32
#include <stdarg.h>
#endif
#include <assert.h>

//-----------------------------------
// Visual Studio memory leak report
#if defined(WIN32) && defined(_DEBUG) && defined(CRTDBG)

#include <crtdbg.h>

#if !defined(METRO_APP)

#ifdef _DEBUG
#define DEBUG_NEW new(_NORMAL_BLOCK, __FILE__, __LINE__)
#define new DEBUG_NEW
#endif
#endif
#endif
//-----------------------------------

#if defined(_DEBUG) && defined(__linux)
#include <sys/ptrace.h>
#include <signal.h>
#endif

namespace amf
{
/**
*******************************************************************************
*   AMFTraceEnableAsync
*
*   @brief
*       Enable or disable async mode
*
*  There are 2 modes trace can work in:
*  Synchronous - every Trace call immediately goes to writers: console, windows, file, ...
*  Asynchronous - trace message go to thread local queues; separate thread passes them to writes
*  Asynchronous mode offers no synchronization between working threads which are writing traces
*  and high performance.
*  Asynchronous mode is not enabled always as that dedicated thread (started in Media SDK module) cannot be
*  terminated safely. See msdn ExitProcess description: it terminates all threads without notifications.
*  ExitProcess is called after exit from main() -> before module static variables destroyed and before atexit
*  notifiers are called -> no way to finish trace dedicated thread.
*
*  Therefore here is direct enable of asynchronous mode.
*  AMFTraceEnableAsync(true) increases internal asynchronous counter by 1; AMFTraceEnableAsync(false) decreases by 1
*  when counter becomes > 0 mode - switches to async; when becomes 0 - switches to sync
*
*  Tracer must be switched to sync mode before quit application, otherwise async writing thread will be force terminated by OS (at lease Windows)
*  See MSDN ExitProcess article for details.
*******************************************************************************
*/
extern "C"
{
AMF_RESULT AMF_CDECL_CALL AMFTraceEnableAsync(bool enable);

/**
*******************************************************************************
*   AMFDebugSetDebugger
*
*   @brief
*       it is used to set a local debugger, or set NULL to remove
*
*******************************************************************************
*/
AMF_RESULT AMF_CDECL_CALL AMFSetCustomDebugger(AMFDebug *pDebugger);

/**
*******************************************************************************
*   AMFTraceSetTracer
*
*   @brief
*       it is used to set a local tracer, or set NULL to remove
*
*******************************************************************************
*/
AMF_RESULT AMF_CDECL_CALL AMFSetCustomTracer(AMFTrace *pTrace);

/**
*******************************************************************************
*   AMFTraceFlush
*
*   @brief
*       Enforce trace writers flush
*
*******************************************************************************
*/
AMF_RESULT AMF_CDECL_CALL AMFTraceFlush();

/**
*******************************************************************************
*   EXPAND
*
*   @brief
*       Auxilary Macro used to evaluate __VA_ARGS__ from 1 macro argument into list of them
*
*   It is needed for COUNT_ARGS macro
*
*******************************************************************************
*/
#define EXPAND(x) x

/**
*******************************************************************************
*   GET_TENTH_ARG
*
*   @brief
*       Auxilary Macro for COUNT_ARGS macro
*
*******************************************************************************
*/
#define GET_TENTH_ARG(a, b, c, d, e, f, g, h, i, j, name, ...) name

/**
*******************************************************************************
*   COUNT_ARGS
*
*   @brief
*       Macro returns number of arguments actually passed into it
*
*   COUNT_ARGS macro works ok for 1..10 arguments
*   It is needed to distinguish macro call with optional parameters and without them
*******************************************************************************
*/
#define COUNT_ARGS(...) EXPAND(GET_TENTH_ARG(__VA_ARGS__, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1))

/**
*******************************************************************************
*   AMFTraceW
*
*   @brief
*       General trace function with all possible parameters
*******************************************************************************
*/
void AMF_CDECL_CALL AMFTraceW(const wchar_t* src_path, amf_int32 line, amf_int32 level, const wchar_t* scope,
        amf_int32 countArgs, const wchar_t* format, ...);

/**
*******************************************************************************
*   AMF_UNICODE
*
*   @brief
*       Macro to convert string constant into wide char string constant
*
*   Auxilary AMF_UNICODE_ macro is needed as otherwise it is not possible to use AMF_UNICODE(__FILE__)
*   Microsoft macro _T also uses 2 passes to accomplish that
*******************************************************************************
*/
#define AMF_UNICODE(s) AMF_UNICODE_(s)
#define AMF_UNICODE_(s) L ## s

/**
*******************************************************************************
*   AMFTrace
*
*   @brief
*       Most general macro for trace, incapsulates passing source file and line
*******************************************************************************
*/
#define AMFTrace(level, scope, /*format, */...) amf::AMFTraceW(AMF_UNICODE(__FILE__), __LINE__, level, scope, COUNT_ARGS(__VA_ARGS__) - 1, __VA_ARGS__)

/**
*******************************************************************************
*   AMFTraceError
*
*   @brief
*       Shortened macro to trace exactly error.
*
*   Similar macroses are: AMFTraceWarning, AMFTraceInfo, AMFTraceDebug
*******************************************************************************
*/
#define AMFTraceError(scope, /*format, */...)   amf::AMFTraceW(AMF_UNICODE(__FILE__), __LINE__, AMF_TRACE_ERROR, scope, COUNT_ARGS(__VA_ARGS__) - 1, __VA_ARGS__)
#define AMFTraceWarning(scope, /*format, */...) amf::AMFTraceW(AMF_UNICODE(__FILE__), __LINE__, AMF_TRACE_WARNING, scope, COUNT_ARGS(__VA_ARGS__) - 1, __VA_ARGS__)
#define AMFTraceInfo(scope, /*format, */...)    amf::AMFTraceW(AMF_UNICODE(__FILE__), __LINE__, AMF_TRACE_INFO, scope, COUNT_ARGS(__VA_ARGS__) - 1, __VA_ARGS__)
#define AMFTraceDebug(scope, /*format, */...)   amf::AMFTraceW(AMF_UNICODE(__FILE__), __LINE__, AMF_TRACE_DEBUG, scope, COUNT_ARGS(__VA_ARGS__) - 1, __VA_ARGS__)

/**
*******************************************************************************
*   AMFDebugHitEvent
*
*   @brief
*       Designed to determine how many are specific events take place
*******************************************************************************
*/
void      AMF_CDECL_CALL AMFDebugHitEvent(const wchar_t* scope, const wchar_t* eventName);
/**
*******************************************************************************
*   AMFDebugGetEventsCount
*
*   @brief
*       Designed to acquire counter of events reported by call AMFDebugHitEvent
*******************************************************************************
*/
amf_int64 AMF_CDECL_CALL AMFDebugGetEventsCount(const wchar_t* scope, const wchar_t* eventName);

/**
*******************************************************************************
*   AMFAssertsEnabled
*
*   @brief
*       Returns bool values indicating if asserts were enabled or not
*******************************************************************************
*/
bool AMF_CDECL_CALL AMFAssertsEnabled();

/**
*******************************************************************************
*   AMFTraceEnterScope
*
*   @brief
*       Increase trace indentation value by 1
*
*   Indentation value is thread specific
*******************************************************************************
*/
void AMF_CDECL_CALL AMFTraceEnterScope();
/**
*******************************************************************************
*   AMFTraceExitScope
*
*   @brief
*       Decrease trace indentation value by 1
*
*   Indentation value is thread specific
*******************************************************************************
*/
void AMF_CDECL_CALL AMFTraceExitScope();

/**
*******************************************************************************
*   AMF_FACILITY
*
*   @brief
*       Default value for AMF_FACILITY, this NULL leads to generate facility from source file name
*
*   This AMF_FACILITY could be overloaded locally with #define AMF_FACILITY L"LocalScope"
*******************************************************************************
*/
static const wchar_t* AMF_FACILITY = NULL;
} //extern "C"

/**
*******************************************************************************
*   AMFDebugBreak
*
*   @brief
*       Macro for switching to debug of application
*******************************************************************************
*/
#if defined(_DEBUG)
#if defined(_WIN32)
#define AMFDebugBreak  {if(amf::AMFAssertsEnabled()) {__debugbreak();} \
}                                                                                //{  }
#elif defined(__linux)
//    #define AMFDebugBreak ((void)0)
#define AMFDebugBreak  {if(amf::AMFAssertsEnabled() && ptrace(PTRACE_TRACEME, 0, 1, 0) < 0) {raise(SIGTRAP);} \
}//{  }
#elif defined(__APPLE__)
#define AMFDebugBreak  {if(amf::AMFAssertsEnabled()) {assert(0);} \
}
#endif
#else
#define AMFDebugBreak
#endif

/**
*******************************************************************************
*   __FormatMessage
*
*   @brief
*       Auxilary function to select from 2 messages and preformat message if any arguments are specified
*******************************************************************************
*/
inline amf_wstring __FormatMessage(int /*argsCount*/, const wchar_t* expression)
{
    return amf_wstring(expression); // the only expression is provided - return this one
}

inline amf_wstring __FormatMessage(int argsCount, const wchar_t* /*expression*/, const wchar_t* message, ...)
{
    // this version of __FormatMessage for case when descriptive message is provided with optional args
    if(argsCount <= 0)
    {
        return amf_wstring(message);
    }
    else
    {
        va_list arglist;
        va_start(arglist, message);
        amf_wstring result = amf::amf_string_formatVA(message, arglist);
        va_end(arglist);
        return result;
    }
}

/**
*******************************************************************************
*   AMF_FIRST_VALUE
*
*   @brief
*       Auxilary macro: extracts first argument from the list
*******************************************************************************
*/
#define AMF_FIRST_VALUE(x, ...) x

/**
*******************************************************************************
*   AMF_BASE_RETURN
*
*   @brief
*       Base generic macro: checks expression for success, if failed: trace error, debug break and return an error
*
*       return_result is a parameter to return to upper level, could be hard-coded or
*           specified exp_res what means pass inner level error
*******************************************************************************
*/
#define AMF_BASE_RETURN(exp, exp_type, check_func, format_prefix, level, scope, return_result/*(could be exp_res)*/, /* optional message args*/ ...) \
    { \
        exp_type exp_res = (exp_type)(exp); \
        if(!check_func(exp_res)) \
        { \
            amf_wstring message = format_prefix(exp_res) + amf::__FormatMessage(COUNT_ARGS(__VA_ARGS__) - 2, __VA_ARGS__); \
            EXPAND(amf::AMFTraceW(AMF_UNICODE(__FILE__), __LINE__, level, scope, 0, message.c_str()) ); \
            AMFDebugBreak; \
            return return_result; \
        } \
    }

/**
*******************************************************************************
*   AMF_BASE_ASSERT
*
*   @brief
*       Base generic macro: checks expression for success, if failed: trace error, debug break
*******************************************************************************
*/
#define AMF_BASE_ASSERT(exp, exp_type, check_func, format_prefix, level, scope, return_result/*(could be exp_res)*/, /*optional message, optional message args*/ ...) \
    { \
        exp_type exp_res = (exp_type)(exp); \
        if(!check_func(exp_res)) \
        { \
            amf_wstring message = format_prefix(exp_res) + amf::__FormatMessage(COUNT_ARGS(__VA_ARGS__) - 2, __VA_ARGS__); \
            EXPAND(amf::AMFTraceW(AMF_UNICODE(__FILE__), __LINE__, level, scope, 0, message.c_str()) ); \
            AMFDebugBreak; \
        } \
    }

/**
*******************************************************************************
*   AMF_BASE_CALL
*
*   @brief
*       Macro supporting cascade call function returning AMF_RESULT from another
*
*       return_result is a parameter to return to upper level, could be hard-coded or
*           specified exp_res what means pass inner level error
*******************************************************************************
*/
#define AMF_BASE_CALL(exp, exp_type, check_func, format_prefix, level, scope, return_result/*(could be exp_res)*/, /*optional message, optional message args*/ ...) \
    { \
        amf_wstring function_name = amf::__FormatMessage(COUNT_ARGS(__VA_ARGS__) - 2, __VA_ARGS__); \
        amf::AMFTraceW(AMF_UNICODE(__FILE__), __LINE__, AMF_TRACE_DEBUG, scope, 0, function_name.c_str()); \
        amf::AMFTraceEnterScope(); \
        exp_type exp_res = (exp_type)(exp); \
        amf::AMFTraceExitScope(); \
        if(!check_func(exp_res)) \
        { \
            amf_wstring message = format_prefix(exp_res) + function_name; \
            EXPAND(amf::AMFTraceW(AMF_UNICODE(__FILE__), __LINE__, level, scope, 0, message.c_str()) ); \
            AMFDebugBreak; \
            return return_result; \
        } \
    }

/**
*******************************************************************************
*   AMFCheckExpression
*
*   @brief
*       Checks if result succeeds
*******************************************************************************
*/
inline bool AMFCheckExpression(int result) { return result != 0; }
/**
*******************************************************************************
*   AMFFormatAssert
*
*   @brief
*       Returns default assertion message
*******************************************************************************
*/
inline amf_wstring AMFFormatAssert(int result) { return result ? amf_wstring() : amf_wstring(L"Assertion failed:"); }

/**
*******************************************************************************
*   AMFOpenCLSucceeded
*
*   @brief
*       Checks cl_status for success
*******************************************************************************
*/
inline bool AMFOpenCLSucceeded(int result) { return result == 0; }
/**
*******************************************************************************
*   AMFFormatOpenCLError
*
*   @brief
*       Formats open CL error
*******************************************************************************
*/
inline amf_wstring AMFFormatOpenCLError(int result)  { return amf::amf_string_format(L"OpenCL failed, error = %d:", result); }
/**
*******************************************************************************
*   AMFResultIsOK
*
*   @brief
*       Checks if AMF_RESULT is OK
*******************************************************************************
*/
inline bool AMFResultIsOK(AMF_RESULT result) { return result == AMF_OK; }
/**
*******************************************************************************
*   AMFSucceeded
*
*   @brief
*       Checks if AMF_RESULT is succeeded
*******************************************************************************
*/
inline bool AMFSucceeded(AMF_RESULT result) { return result == AMF_OK || result == AMF_REPEAT; }
/**
*******************************************************************************
*   AMFFormatResult
*
*   @brief
*       Formats AMF_RESULT into descriptive string
*******************************************************************************
*/
amf_wstring AMF_CDECL_CALL  AMFFormatResult(AMF_RESULT result);

/**
*******************************************************************************
*   AMFHResultSucceded
*
*   @brief
*       Checks if HRESULT succeeded
*******************************************************************************
*/
inline bool AMFHResultSucceded(HRESULT result) { return SUCCEEDED(result); }
/**
*******************************************************************************
*   AMFFormatHResult
*
*   @brief
*       Formats HRESULT into descriptive string
*******************************************************************************
*/
inline amf_wstring AMFFormatHResult(HRESULT result)  { return amf::amf_string_format(L"COM failed, HR = %0X:", result); }

/**
*******************************************************************************
*   AMFVkResultSucceeded
*
*   @brief
*       Checks if VkResult succeeded
*******************************************************************************
*/
inline bool AMFVkResultSucceeded(int result) { return result == 0; }

/**
*******************************************************************************
*   AMFFormatVkResult
*
*   @brief
*       Formats VkResult into descriptive string
*******************************************************************************
*/
inline amf_wstring AMFFormatVkResult(int result) { return amf::amf_string_format(L"Vulkan failed, VkResult = %d:", result); }

/**
*******************************************************************************
*   AMF_CALL
*
*   @brief
*       Macro to call AMF_RESULT returning function from AMF_RESULT returning function
*
*   It does:
*       1) Trace (level == debug) function name (or message if specified)
*       2) Indent trace
*       3) Call function
*       4) Unindent trace
*       5) Checks its result
*       6) If not OK trace error, switch to debugger (if asserts enabled) and return that error code to upper level
*
*   Use cases:
*       A) AMF_CALL(Init("Name"));      // trace expression itself
*       B) AMF_CALL(Init("Name"), L"Initialize resources");  // trace desciptive message
*       C) AMF_CALL(Init(name), L"Initialize resources with %s", name);   // trace descriptive message with aditional arguments from runtime
*******************************************************************************
*/
#define AMF_CALL(exp, ... /*optional format, args*/) AMF_BASE_CALL(exp, AMF_RESULT, amf::AMFResultIsOK, amf::AMFFormatResult, AMF_TRACE_ERROR, AMF_FACILITY, exp_res, L###exp, ##__VA_ARGS__)

/**
*******************************************************************************
*   AMF_ASSERT_OK
*
*   @brief
*       Checks expression == AMF_OK, otherwise trace error and debug break
*
*       Could be used: A) with just expression B) with optinal descriptive message C) message + args for printf
*******************************************************************************
*/
#define AMF_ASSERT_OK(exp, ... /*optional format, args*/) AMF_BASE_ASSERT(exp, AMF_RESULT, amf::AMFResultIsOK, amf::AMFFormatResult, AMF_TRACE_ERROR, AMF_FACILITY, AMF_FAIL, L###exp, ##__VA_ARGS__)

/**
*******************************************************************************
*   AMF_ASSERT
*
*   @brief
*       Checks expression != 0, otherwise trace error and debug break
*
*       Could be used: A) with just expression B) with optinal descriptive message C) message + args for printf
*******************************************************************************
*/
#define AMF_ASSERT(exp, ...) AMF_BASE_ASSERT(exp, int, amf::AMFCheckExpression, amf::AMFFormatAssert, AMF_TRACE_ERROR, AMF_FACILITY, AMF_FAIL, L###exp, ##__VA_ARGS__)

/**
*******************************************************************************
*   AMF_RETURN_IF_FAILED
*
*   @brief
*       Checks expression != 0, otherwise trace error, debug break and return that error to upper level
*
*       Could be used: A) with just expression B) with optinal descriptive message C) message + args for printf
*******************************************************************************
*/
#define AMF_RETURN_IF_FAILED(exp, ...) AMF_BASE_RETURN(exp, AMF_RESULT, amf::AMFResultIsOK, amf::AMFFormatResult, AMF_TRACE_ERROR, AMF_FACILITY, exp_res, L###exp, ##__VA_ARGS__)

/**
*******************************************************************************
*   ASSERT_RETURN_IF_CL_FAILED
*
*   @brief
*       Checks cl error is ok, otherwise trace error, debug break and return that error to upper level
*
*       Could be used: A) with just expression B) with optinal descriptive message C) message + args for printf
*******************************************************************************
*/
#define ASSERT_RETURN_IF_CL_FAILED(exp, /*optional format, args,*/...) AMF_BASE_RETURN(exp, int, amf::AMFOpenCLSucceeded, amf::AMFFormatOpenCLError, AMF_TRACE_ERROR, AMF_FACILITY, AMF_OPENCL_FAILED, L###exp, ##__VA_ARGS__)
#define AMF_RETURN_IF_CL_FAILED(exp, /*optional format, args,*/...) AMF_BASE_RETURN(exp, int, amf::AMFOpenCLSucceeded, amf::AMFFormatOpenCLError, AMF_TRACE_ERROR, AMF_FACILITY, AMF_OPENCL_FAILED, L###exp, ##__VA_ARGS__)

/**
*******************************************************************************
*   ASSERT_RETURN_IF_HR_FAILED
*
*   @brief
*       Obsolete macro: Checks HRESULT if succeeded, otherwise trace error, debug break and return specified error to upper level
*
*       Other macroses below are also obsolete
*******************************************************************************
*/
#define ASSERT_RETURN_IF_HR_FAILED(exp, reterr, /*optional format, args,*/...) AMF_BASE_RETURN(exp, HRESULT, amf::AMFHResultSucceded, amf::AMFFormatHResult, AMF_TRACE_ERROR, AMF_FACILITY, reterr, L###exp, ##__VA_ARGS__)

/**
*******************************************************************************
*   ASSERT_RETURN_IF_VK_FAILED
*
*   @brief
*       Checks VkResult if succeeded, otherwise trace error, debug break and return specified error to upper level
*
*       Could be used: A) with just expression B) with optinal descriptive message C) message + args for printf
*******************************************************************************
*/
#define ASSERT_RETURN_IF_VK_FAILED(exp, reterr, /*optional format, args,*/...) AMF_BASE_RETURN(exp, int, amf::AMFVkResultSucceeded, amf::AMFFormatVkResult, AMF_TRACE_ERROR, AMF_FACILITY, reterr, L###exp, ##__VA_ARGS__)


/**
*******************************************************************************
*   AMF_RETURN_IF_FALSE
*
*   @brief
*       Checks expression != 0, otherwise trace error, debug break and return that error to upper level
*
*       Could be used: A) with just expression B) with optinal descriptive message C) message + args for printf
*******************************************************************************
*/
#define AMF_RETURN_IF_FALSE(exp, ret_value, /*optional message,*/ ...) AMF_BASE_RETURN(exp, int, amf::AMFCheckExpression, amf::AMFFormatAssert, AMF_TRACE_ERROR, AMF_FACILITY, ret_value, L###exp, ##__VA_ARGS__)

/**
*******************************************************************************
*   AMF_RETURN_IF_INVALID_POINTER
*
*   @brief
*       Checks ptr != NULL, otherwise trace error, debug break and return that error to upper level
*
*******************************************************************************
*/
#define AMF_RETURN_IF_INVALID_POINTER(ptr, /*optional message,*/ ...) AMF_BASE_RETURN(ptr != NULL, int, amf::AMFCheckExpression, amf::AMFFormatAssert, AMF_TRACE_ERROR, AMF_FACILITY, AMF_INVALID_POINTER, L"invalid pointer : " L###ptr, ##__VA_ARGS__)

/**
*******************************************************************************
*   AMFTestEventObserver
*
*   @brief
*       Interface to subscribe on test events
*******************************************************************************
*/

extern "C"
{

    /**
    *******************************************************************************
    *   AMFTraceSetPath
    *
    *   @brief
    *       Set Trace path
    *
    *       Returns AMF_OK if succeeded
    *******************************************************************************
    */
    AMF_RESULT AMF_CDECL_CALL  AMFTraceSetPath(const wchar_t* path);

    /**
    *******************************************************************************
    *   AMFTraceGetPath
    *
    *   @brief
    *       Get Trace path
    *
    *       Returns AMF_OK if succeeded
    *******************************************************************************
    */
    AMF_RESULT AMF_CDECL_CALL  AMFTraceGetPath(
        wchar_t* path, ///< [out] buffer able to hold *pSize symbols; path is copied there, at least part fitting the buffer, always terminator is copied
        amf_size* pSize ///< [in, out] size of buffer, returned needed size of buffer including zero terminator
    );

    /**
    *******************************************************************************
    *   AMFTraceEnableWriter
    *
    *   @brief
    *       Disable trace to registered writer
    *
    *       Returns previous state
    *******************************************************************************
    */
    bool AMF_CDECL_CALL  AMFTraceEnableWriter(const wchar_t* writerID, bool enable);

    /**
    *******************************************************************************
    *   AMFTraceWriterEnabled
    *
    *   @brief
    *       Return flag if writer enabled
    *******************************************************************************
    */
    bool AMF_CDECL_CALL  AMFTraceWriterEnabled(const wchar_t* writerID);

    /**
    *******************************************************************************
    *   AMFTraceSetGlobalLevel
    *
    *   @brief
    *       Sets trace level for writer and scope
    *
    *       Returns previous setting
    *******************************************************************************
    */
    amf_int32 AMF_CDECL_CALL  AMFTraceSetGlobalLevel(amf_int32 level);

    /**
    *******************************************************************************
    *   AMFTraceGetGlobalLevel
    *
    *   @brief
    *       Returns global level
    *******************************************************************************
    */
    amf_int32 AMF_CDECL_CALL  AMFTraceGetGlobalLevel();

    /**
    *******************************************************************************
    *   AMFTraceSetWriterLevel
    *
    *   @brief
    *       Sets trace level for writer
    *
    *       Returns previous setting
    *******************************************************************************
    */
    amf_int32 AMF_CDECL_CALL  AMFTraceSetWriterLevel(const wchar_t* writerID, amf_int32 level);

    /**
    *******************************************************************************
    *   AMFTraceGetWriterLevel
    *
    *   @brief
    *       Gets trace level for writer
    *******************************************************************************
    */
    amf_int32 AMF_CDECL_CALL  AMFTraceGetWriterLevel(const wchar_t* writerID);

    /**
    *******************************************************************************
    *   AMFTraceSetWriterLevelForScope
    *
    *   @brief
    *       Sets trace level for writer and scope
    *
    *       Returns previous setting
    *******************************************************************************
    */
    amf_int32 AMF_CDECL_CALL  AMFTraceSetWriterLevelForScope(const wchar_t* writerID, const wchar_t* scope, amf_int32 level);

    /**
    *******************************************************************************
    *   AMFTraceGetWriterLevelForScope
    *
    *   @brief
    *       Gets trace level for writer and scope
    *******************************************************************************
    */
    amf_int32 AMF_CDECL_CALL  AMFTraceGetWriterLevelForScope(const wchar_t* writerID, const wchar_t* scope);

    /**
    *******************************************************************************
    *   AMFTraceRegisterWriter
    *
    *   @brief
    *       Register custom trace writer
    *
    *******************************************************************************
    */
    void AMF_CDECL_CALL  AMFTraceRegisterWriter(const wchar_t* writerID, AMFTraceWriter* pWriter);

    /**
    *******************************************************************************
    *   AMFTraceUnregisterWriter
    *
    *   @brief
    *       Register custom trace writer
    *
    *******************************************************************************
    */
    void AMF_CDECL_CALL  AMFTraceUnregisterWriter(const wchar_t* writerID);
    /*
    *******************************************************************************
    *   AMFAssertsEnable
    *
    *   @brief
    *       Enable asserts in checks
    *
    *******************************************************************************
    */
    void AMF_CDECL_CALL  AMFAssertsEnable(bool enable);

    /**
    *******************************************************************************
    *   AMFAssertsEnabled
    *
    *   @brief
    *       Returns true if asserts in checks enabled
    *
    *******************************************************************************
    */
    bool AMF_CDECL_CALL  AMFAssertsEnabled();

    const wchar_t* AMF_STD_CALL AMFGetResultText(AMF_RESULT res);
    const wchar_t* AMF_STD_CALL AMFSurfaceGetFormatName(const AMF_SURFACE_FORMAT eSurfaceFormat);
    AMF_SURFACE_FORMAT AMF_STD_CALL AMFSurfaceGetFormatByName(const wchar_t* pwName);
    const wchar_t*  AMF_STD_CALL AMFGetMemoryTypeName(const AMF_MEMORY_TYPE memoryType);
    AMF_MEMORY_TYPE AMF_STD_CALL AMFGetMemoryTypeByName(const wchar_t* name);
} //extern "C"
} // namespace amf
#endif // AMF_TraceAdapter_h
