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

#include "AMFFactory.h"
#include "Thread.h"

#ifdef __clang__
    #pragma clang diagnostic push
    #pragma clang diagnostic ignored "-Wexit-time-destructors"
    #pragma clang diagnostic ignored "-Wglobal-constructors"
#endif

AMFFactoryHelper g_AMFFactory;
#ifdef __clang__
    #pragma clang diagnostic pop
#endif

#ifdef AMF_CORE_STATIC
extern "C"
{
    extern AMF_CORE_LINK AMF_RESULT AMF_CDECL_CALL AMFInit(amf_uint64 version, amf::AMFFactory **ppFactory);
}
#endif

//-------------------------------------------------------------------------------------------------
AMFFactoryHelper::AMFFactoryHelper() :
m_hDLLHandle(NULL),
m_pFactory(NULL),
m_pDebug(NULL),
m_pTrace(NULL),
m_AMFRuntimeVersion(0),
m_iRefCount(0)
{
}
//-------------------------------------------------------------------------------------------------
AMFFactoryHelper::~AMFFactoryHelper()
{
    Terminate();
}
//-------------------------------------------------------------------------------------------------
AMF_RESULT AMFFactoryHelper::Init(const wchar_t* dllName)
{
    dllName;

#ifndef AMF_CORE_STATIC
    if (m_hDLLHandle != NULL)
    {
        amf_atomic_inc(&m_iRefCount);
        return AMF_OK;
    }

    const wchar_t* dllName_ = dllName == NULL ? AMF_DLL_NAME : dllName;
#if defined (_WIN32) || defined (__APPLE__)
    m_hDLLHandle = amf_load_library(dllName_);
#else
    m_hDLLHandle = amf_load_library1(dllName_, false); //load with local flags
#endif
    if(m_hDLLHandle == NULL)
    {
        return AMF_FAIL;
    }

    AMFInit_Fn initFun = (AMFInit_Fn)::amf_get_proc_address(m_hDLLHandle, AMF_INIT_FUNCTION_NAME);
    if(initFun == NULL)
    {
        return AMF_FAIL;
    }
    AMF_RESULT res = initFun(AMF_FULL_VERSION, &m_pFactory);
    if(res != AMF_OK)
    {
        return res;
    }
    AMFQueryVersion_Fn versionFun = (AMFQueryVersion_Fn)::amf_get_proc_address(m_hDLLHandle, AMF_QUERY_VERSION_FUNCTION_NAME);
    if(versionFun == NULL)
    {
        return AMF_FAIL;
    }
    res = versionFun(&m_AMFRuntimeVersion);
    if(res != AMF_OK)
    {
        return res;
    }
#else
    AMF_RESULT res = AMFInit(AMF_FULL_VERSION, &m_pFactory);
    if (res != AMF_OK)
    {
        return res;
    }
    m_AMFRuntimeVersion = AMF_FULL_VERSION;
#endif
    m_pFactory->GetTrace(&m_pTrace);
    m_pFactory->GetDebug(&m_pDebug);

    amf_atomic_inc(&m_iRefCount);
    return AMF_OK;
}
//-------------------------------------------------------------------------------------------------
AMF_RESULT AMFFactoryHelper::Terminate()
{
    if(m_hDLLHandle != NULL)
    {
        amf_atomic_dec(&m_iRefCount);
        if(m_iRefCount == 0)
        {
            amf_free_library(m_hDLLHandle);
            m_hDLLHandle = NULL;
            m_pFactory= NULL;
            m_pDebug = NULL;
            m_pTrace = NULL;
        }
    }

    return AMF_OK;
}
//-------------------------------------------------------------------------------------------------
amf::AMFFactory* AMFFactoryHelper::GetFactory()
{
    return m_pFactory;
}
//-------------------------------------------------------------------------------------------------
amf::AMFDebug* AMFFactoryHelper::GetDebug()
{
    return m_pDebug;
}
//-------------------------------------------------------------------------------------------------
amf::AMFTrace* AMFFactoryHelper::GetTrace()
{
    return m_pTrace;
}
//-------------------------------------------------------------------------------------------------
amf_uint64 AMFFactoryHelper::AMFQueryVersion()
{
    return m_AMFRuntimeVersion;
}

//-------------------------------------------------------------------------------------------------
AMF_RESULT  AMFFactoryHelper::LoadExternalComponent(amf::AMFContext* pContext, const wchar_t* dll, const char* function, void* reserved, amf::AMFComponent** ppComponent)
{
    // check passed in parameters
    if (!pContext || !dll || !function)
    {
        return AMF_INVALID_ARG;
    }

    // check if DLL has already been loaded
    amf_handle  hDll = NULL;
    for (std::vector<ComponentHolder>::iterator it = m_extComponents.begin(); it != m_extComponents.end(); ++it)
    {
#if defined(_WIN32)
         if (wcsicmp(it->m_DLL.c_str(), dll) == 0) // ignore case on Windows
#elif defined(__linux) // Linux
        if (wcscmp(it->m_DLL.c_str(), dll) == 0) // case sensitive on Linux
#endif
        {
            if (it->m_hDLLHandle != NULL)
            {
                hDll = it->m_hDLLHandle;
                amf_atomic_inc(&it->m_iRefCount);
                break;
            }

            return AMF_UNEXPECTED;
        }
    }
    // DLL wasn't loaded before so load it now and
    // add it to the internal list
    if (hDll == NULL)
    {
        ComponentHolder component;
        component.m_iRefCount = 0;
        component.m_hDLLHandle = NULL;
        component.m_DLL = dll;

#if defined(_WIN32) || defined(__APPLE__)
        hDll = amf_load_library(dll);
#else
        hDll = amf_load_library1(dll, false); //global flag set to true
#endif
        if (hDll == NULL)
            return AMF_FAIL;

        // since LoadLibrary succeeded add the information
        // into the internal list so we can properly free
        // the DLL later on, even if we fail to get the
        // required information from it...
        component.m_hDLLHandle = hDll;
        amf_atomic_inc(&component.m_iRefCount);
        m_extComponents.push_back(component);
    }

    // look for function we want in the dll we just loaded
    typedef AMF_RESULT(AMF_CDECL_CALL *AMFCreateComponentFunc)(amf::AMFContext*, void* reserved, amf::AMFComponent**);
    AMFCreateComponentFunc  initFn = (AMFCreateComponentFunc)::amf_get_proc_address(hDll, function);
    if (initFn == NULL)
        return AMF_FAIL;

    return initFn(pContext, reserved, ppComponent);
}
//-------------------------------------------------------------------------------------------------
AMF_RESULT  AMFFactoryHelper::UnLoadExternalComponent(const wchar_t* dll)
{
    if (!dll)
    {
        return AMF_INVALID_ARG;
    }
    for (std::vector<ComponentHolder>::iterator it = m_extComponents.begin(); it != m_extComponents.end(); ++it)
    {
#if defined(_WIN32)
         if (wcsicmp(it->m_DLL.c_str(), dll) == 0) // ignore case on Windows
#elif defined(__linux) // Linux
        if (wcscmp(it->m_DLL.c_str(), dll) == 0) // case sensitive on Linux
#endif
        {
            if (it->m_hDLLHandle == NULL)
            {
                return AMF_UNEXPECTED;
            }
            amf_atomic_dec(&it->m_iRefCount);
            if (it->m_iRefCount == 0)
            {
                amf_free_library(it->m_hDLLHandle);
                m_extComponents.erase(it);
            }
            break;
        }
    }
    return AMF_OK;
}
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------

