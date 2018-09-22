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

#pragma once

#include "../include/core/Factory.h"
#include <string>
#include <vector>


class AMFFactoryHelper
{
public:
    AMFFactoryHelper();
    virtual ~AMFFactoryHelper();

    AMF_RESULT  Init();
    AMF_RESULT  Terminate();

    AMF_RESULT  LoadExternalComponent(amf::AMFContext* pContext, const wchar_t* dll, const char* function, void* reserved, amf::AMFComponent** ppComponent);
    AMF_RESULT  UnLoadExternalComponent(const wchar_t* dll);

    amf::AMFFactory* GetFactory();
    amf::AMFDebug* GetDebug();
    amf::AMFTrace* GetTrace();

    amf_uint64 AMFQueryVersion();
protected:
    struct ComponentHolder
    {
        HMODULE         m_hDLLHandle;
        amf_long        m_iRefCount;
        std::wstring    m_DLL;

        ComponentHolder()
        {
            m_hDLLHandle = NULL;
            m_iRefCount = 0;
        }
    };

    HMODULE             m_hDLLHandle;
    amf::AMFFactory*    m_pFactory;
    amf::AMFDebug*      m_pDebug;
    amf::AMFTrace*      m_pTrace;
    amf_uint64          m_AMFRuntimeVersion;

    amf_long            m_iRefCount;

    std::vector<ComponentHolder>  m_extComponents;
};

extern ::AMFFactoryHelper g_AMFFactory;
