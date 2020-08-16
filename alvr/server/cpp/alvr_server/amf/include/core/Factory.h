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
// Copyright (c) 2016 Advanced Micro Devices, Inc. All rights reserved.
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

#ifndef __AMFFactory_h__
#define __AMFFactory_h__
#pragma once

#include "Platform.h"
#include "Version.h"
#include "Result.h"
#include "Context.h"
#include "Debug.h"
#include "Trace.h"
#include "Compute.h"

#include "../components/Component.h"

namespace amf
{
    //----------------------------------------------------------------------------------------------
    // AMFFactory interface - singleton
    //----------------------------------------------------------------------------------------------
    class AMF_NO_VTABLE AMFFactory
    {
    public:
        virtual AMF_RESULT          AMF_STD_CALL CreateContext(amf::AMFContext** ppContext) = 0;
        virtual AMF_RESULT          AMF_STD_CALL CreateComponent(amf::AMFContext* pContext, const wchar_t* id, amf::AMFComponent** ppComponent) = 0;
        virtual AMF_RESULT          AMF_STD_CALL SetCacheFolder(const wchar_t* path) = 0;
        virtual const wchar_t*      AMF_STD_CALL GetCacheFolder() = 0;
        virtual AMF_RESULT          AMF_STD_CALL GetDebug(amf::AMFDebug** ppDebug) = 0;
        virtual AMF_RESULT          AMF_STD_CALL GetTrace(amf::AMFTrace** ppTrace) = 0;
        virtual AMF_RESULT          AMF_STD_CALL GetPrograms(amf::AMFPrograms** ppPrograms) = 0;
   };
}

//----------------------------------------------------------------------------------------------
// DLL entry points
//----------------------------------------------------------------------------------------------

#define AMF_INIT_FUNCTION_NAME             "AMFInit"
#define AMF_QUERY_VERSION_FUNCTION_NAME    "AMFQueryVersion"

extern "C"
{
    typedef AMF_RESULT             (AMF_CDECL_CALL *AMFInit_Fn)(amf_uint64 version, amf::AMFFactory **ppFactory);
    typedef AMF_RESULT             (AMF_CDECL_CALL *AMFQueryVersion_Fn)(amf_uint64 *pVersion);
}

#if defined(_M_AMD64)
    #define AMF_DLL_NAME    L"amfrt64.dll"
#else
    #define AMF_DLL_NAME    L"amfrt32.dll"
#endif

//----------------------------------------------------------------------------------------------

#endif 