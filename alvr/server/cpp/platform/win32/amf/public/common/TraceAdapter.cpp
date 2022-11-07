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

#include "../include/core/Factory.h"
#include "Thread.h"
#include "TraceAdapter.h"

#pragma warning(disable: 4251)
#pragma warning(disable: 4996)

using namespace amf;

#if defined(AMF_CORE_STATIC) || defined(AMF_RUNTIME) || defined(AMF_LITE)
extern "C"
{
    extern AMF_CORE_LINK AMF_RESULT AMF_CDECL_CALL AMFInit(amf_uint64 version, amf::AMFFactory **ppFactory);
}
#else 
 #include "AMFFactory.h"
#endif

//------------------------------------------------------------------------------------------------
static AMFTrace *s_pTrace = NULL;
//------------------------------------------------------------------------------------------------
static AMFTrace *GetTrace()
{
    if (s_pTrace == NULL)
    {
#if defined(AMF_CORE_STATIC) || defined(AMF_RUNTIME) || defined(AMF_LITE)
        AMFFactory *pFactory = NULL;
        AMFInit(AMF_FULL_VERSION, &pFactory);
        pFactory->GetTrace(&s_pTrace);
#else
        s_pTrace = g_AMFFactory.GetTrace();
        if (s_pTrace == nullptr) 
        {
            g_AMFFactory.Init(); // last resort, should not happen
            s_pTrace = g_AMFFactory.GetTrace();
            g_AMFFactory.Terminate();
        }
#endif
    }
    return s_pTrace;
}
//------------------------------------------------------------------------------------------------
static AMFDebug *s_pDebug = NULL;
//------------------------------------------------------------------------------------------------
static AMFDebug *GetDebug()
{
    if (s_pDebug == NULL)
    {
#if defined(AMF_CORE_STATIC) || defined(AMF_RUNTIME) || defined(AMF_LITE)
        AMFFactory *pFactory = NULL;
        AMFInit(AMF_FULL_VERSION, &pFactory);
        pFactory->GetDebug(&s_pDebug);
#else
        s_pDebug = g_AMFFactory.GetDebug();
        if (s_pDebug == nullptr)
        {
            g_AMFFactory.Init(); // last resort, should not happen
            s_pDebug = g_AMFFactory.GetDebug();
            g_AMFFactory.Terminate();
        }
#endif
    }
    return s_pDebug;
}
//------------------------------------------------------------------------------------------------
AMF_RESULT AMF_CDECL_CALL amf::AMFSetCustomDebugger(AMFDebug *pDebugger)
{
    s_pDebug = pDebugger;
    return AMF_OK;
}
//------------------------------------------------------------------------------------------------
AMF_RESULT AMF_CDECL_CALL amf::AMFSetCustomTracer(AMFTrace *pTracer)
{
    s_pTrace = pTracer;
    return AMF_OK;
}
//------------------------------------------------------------------------------------------------
AMF_RESULT AMF_CDECL_CALL amf::AMFTraceEnableAsync(bool enable)
{
    return GetTrace()->TraceEnableAsync(enable);
}
//------------------------------------------------------------------------------------------------
AMF_RESULT AMF_CDECL_CALL amf::AMFTraceFlush()
{
    return GetTrace()->TraceFlush();
}
//------------------------------------------------------------------------------------------------
void AMF_CDECL_CALL amf::AMFTraceW(const wchar_t* src_path, amf_int32 line, amf_int32 level, const wchar_t* scope,
            amf_int32 countArgs, const wchar_t* format, ...) // if countArgs <= 0 -> no args, formatting could be optimized then
{
    if(countArgs <= 0)
    {
        GetTrace()->Trace(src_path, line, level, scope, format, NULL);
    }
    else
    {
        va_list vl;
        va_start(vl, format);

        GetTrace()->Trace(src_path, line, level, scope, format, &vl);

        va_end(vl);
    }
}
//------------------------------------------------------------------------------------------------
AMF_RESULT AMF_CDECL_CALL amf::AMFTraceSetPath(const wchar_t* path)
{
    return GetTrace()->SetPath(path);
}
//------------------------------------------------------------------------------------------------
AMF_RESULT AMF_CDECL_CALL amf::AMFTraceGetPath(wchar_t* path, amf_size* pSize)
{
    return GetTrace()->GetPath(path, pSize);
}
//------------------------------------------------------------------------------------------------
bool AMF_CDECL_CALL amf::AMFTraceEnableWriter(const wchar_t* writerID, bool enable)
{
    return GetTrace()->EnableWriter(writerID, enable);
}
//------------------------------------------------------------------------------------------------
bool AMF_CDECL_CALL amf::AMFTraceWriterEnabled(const wchar_t* writerID)
{
    return GetTrace()->WriterEnabled(writerID);
}
//------------------------------------------------------------------------------------------------
amf_int32 AMF_CDECL_CALL amf::AMFTraceSetGlobalLevel(amf_int32 level)
{
    return GetTrace()->SetGlobalLevel(level);
}
//------------------------------------------------------------------------------------------------
amf_int32 AMF_CDECL_CALL amf::AMFTraceGetGlobalLevel()
{
    return GetTrace()->GetGlobalLevel();
}
//------------------------------------------------------------------------------------------------
amf_int32 AMF_CDECL_CALL amf::AMFTraceSetWriterLevel(const wchar_t* writerID, amf_int32 level)
{
    return GetTrace()->SetWriterLevel(writerID, level);
}
//------------------------------------------------------------------------------------------------
amf_int32 AMF_CDECL_CALL amf::AMFTraceGetWriterLevel(const wchar_t* writerID)
{
    return GetTrace()->GetWriterLevel(writerID);
}
//------------------------------------------------------------------------------------------------
amf_int32 AMF_CDECL_CALL amf::AMFTraceSetWriterLevelForScope(const wchar_t* writerID, const wchar_t* scope, amf_int32 level)
{
    return GetTrace()->SetWriterLevelForScope(writerID, scope, level);
}
//------------------------------------------------------------------------------------------------
amf_int32 AMF_CDECL_CALL amf::AMFTraceGetWriterLevelForScope(const wchar_t* writerID, const wchar_t* scope)
{
    return GetTrace()->GetWriterLevelForScope(writerID, scope);
}
//------------------------------------------------------------------------------------------------
void AMF_CDECL_CALL amf::AMFTraceRegisterWriter(const wchar_t* writerID, AMFTraceWriter* pWriter)
{
    GetTrace()->RegisterWriter(writerID, pWriter, true);
}

void AMF_CDECL_CALL amf::AMFTraceUnregisterWriter(const wchar_t* writerID)
{
    GetTrace()->UnregisterWriter(writerID);
}

#ifdef __clang__
    #pragma clang diagnostic push
    #pragma clang diagnostic ignored "-Wexit-time-destructors"
    #pragma clang diagnostic ignored "-Wglobal-constructors"
#endif

void AMF_CDECL_CALL amf::AMFTraceEnterScope()
{
    GetTrace()->Indent(1);
}

amf_uint32 AMF_CDECL_CALL AMFTraceGetScopeDepth()
{
    return GetTrace()->GetIndentation();
}

void AMF_CDECL_CALL amf::AMFTraceExitScope()
{
    GetTrace()->Indent(-1);
}

void AMF_CDECL_CALL  amf::AMFAssertsEnable(bool enable)
{
    GetDebug()->AssertsEnable(enable);
}
bool AMF_CDECL_CALL  amf::AMFAssertsEnabled()
{
    return GetDebug()->AssertsEnabled();
}
amf_wstring AMF_CDECL_CALL  amf::AMFFormatResult(AMF_RESULT result) 
{ 
    return amf::amf_string_format(L"AMF_ERROR %d : %s: ", result, GetTrace()->GetResultText(result)); 
}

const wchar_t* AMF_STD_CALL amf::AMFGetResultText(AMF_RESULT res)
{
    return GetTrace()->GetResultText(res);
}
const wchar_t* AMF_STD_CALL amf::AMFSurfaceGetFormatName(const AMF_SURFACE_FORMAT eSurfaceFormat)
{
    return GetTrace()->SurfaceGetFormatName(eSurfaceFormat);
}
AMF_SURFACE_FORMAT AMF_STD_CALL amf::AMFSurfaceGetFormatByName(const wchar_t* pwName)
{
    return GetTrace()->SurfaceGetFormatByName(pwName);
}
const wchar_t* AMF_STD_CALL amf::AMFGetMemoryTypeName(const AMF_MEMORY_TYPE memoryType)
{
    return GetTrace()->GetMemoryTypeName(memoryType);
}

AMF_MEMORY_TYPE AMF_STD_CALL amf::AMFGetMemoryTypeByName(const wchar_t* name)
{
    return GetTrace()->GetMemoryTypeByName(name);
}
