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

#ifndef __AMFContext_h__
#define __AMFContext_h__
#pragma once

#include "Buffer.h"
#include "AudioBuffer.h"
#include "Surface.h"
#include "Compute.h"
#include "ComputeFactory.h"

namespace amf
{
    //----------------------------------------------------------------------------------------------
    // AMFContext interface
    //----------------------------------------------------------------------------------------------
    class AMF_NO_VTABLE AMFContext : public AMFPropertyStorage
    {
    public:
        AMF_DECLARE_IID(0xa76a13f0, 0xd80e, 0x4fcc, 0xb5, 0x8, 0x65, 0xd0, 0xb5, 0x2e, 0xd9, 0xee)
        
        // Cleanup
        virtual AMF_RESULT          AMF_STD_CALL Terminate() = 0;

        // DX9
        virtual AMF_RESULT          AMF_STD_CALL InitDX9(void* pDX9Device) = 0;
        virtual void*               AMF_STD_CALL GetDX9Device(AMF_DX_VERSION dxVersionRequired = AMF_DX9) = 0;
        virtual AMF_RESULT          AMF_STD_CALL LockDX9() = 0;
        virtual AMF_RESULT          AMF_STD_CALL UnlockDX9() = 0;
        class AMFDX9Locker;

        // DX11
        virtual AMF_RESULT          AMF_STD_CALL InitDX11(void* pDX11Device, AMF_DX_VERSION dxVersionRequired = AMF_DX11_0) = 0;
        virtual void*               AMF_STD_CALL GetDX11Device(AMF_DX_VERSION dxVersionRequired = AMF_DX11_0) = 0;
        virtual AMF_RESULT          AMF_STD_CALL LockDX11() = 0;
        virtual AMF_RESULT          AMF_STD_CALL UnlockDX11() = 0;
        class AMFDX11Locker;

        // OpenCL
        virtual AMF_RESULT          AMF_STD_CALL InitOpenCL(void* pCommandQueue = NULL) = 0;
        virtual void*               AMF_STD_CALL GetOpenCLContext() = 0;
        virtual void*               AMF_STD_CALL GetOpenCLCommandQueue() = 0;
        virtual void*               AMF_STD_CALL GetOpenCLDeviceID() = 0;
        virtual AMF_RESULT          AMF_STD_CALL GetOpenCLComputeFactory(AMFComputeFactory **ppFactory) = 0; // advanced compute - multiple queries
        virtual AMF_RESULT          AMF_STD_CALL InitOpenCLEx(AMFComputeDevice *pDevice) = 0;
        virtual AMF_RESULT          AMF_STD_CALL LockOpenCL() = 0;
        virtual AMF_RESULT          AMF_STD_CALL UnlockOpenCL() = 0;
        class AMFOpenCLLocker;

        // OpenGL
        virtual AMF_RESULT          AMF_STD_CALL InitOpenGL(amf_handle hOpenGLContext, amf_handle hWindow, amf_handle hDC) = 0;
        virtual amf_handle          AMF_STD_CALL GetOpenGLContext() = 0;
        virtual amf_handle          AMF_STD_CALL GetOpenGLDrawable() = 0;
        virtual AMF_RESULT          AMF_STD_CALL LockOpenGL() = 0;
        virtual AMF_RESULT          AMF_STD_CALL UnlockOpenGL() = 0;
        class AMFOpenGLLocker;

        // XV - Linux
        virtual AMF_RESULT          AMF_STD_CALL InitXV(void* pXVDevice) = 0;
        virtual void*               AMF_STD_CALL GetXVDevice() = 0;
        virtual AMF_RESULT          AMF_STD_CALL LockXV() = 0;
        virtual AMF_RESULT          AMF_STD_CALL UnlockXV() = 0;
        class AMFXVLocker;

        // Gralloc - Android
        virtual AMF_RESULT          AMF_STD_CALL InitGralloc(void* pGrallocDevice) = 0;
        virtual void*               AMF_STD_CALL GetGrallocDevice() = 0;
        virtual AMF_RESULT          AMF_STD_CALL LockGralloc() = 0;
        virtual AMF_RESULT          AMF_STD_CALL UnlockGralloc() = 0;
        class AMFGrallocLocker;

        // Allocation
        virtual AMF_RESULT          AMF_STD_CALL AllocBuffer(AMF_MEMORY_TYPE type, amf_size size, AMFBuffer** ppBuffer) = 0;
        virtual AMF_RESULT          AMF_STD_CALL AllocSurface(AMF_MEMORY_TYPE type, AMF_SURFACE_FORMAT format, amf_int32 width, amf_int32 height, AMFSurface** ppSurface) = 0;
        virtual AMF_RESULT          AMF_STD_CALL AllocAudioBuffer(AMF_MEMORY_TYPE type, AMF_AUDIO_FORMAT format, amf_int32 samples, amf_int32 sampleRate, amf_int32 channels, 
                                                    AMFAudioBuffer** ppAudioBuffer) = 0;

        // Wrap existing objects
        virtual AMF_RESULT          AMF_STD_CALL CreateBufferFromHostNative(void* pHostBuffer, amf_size size, AMFBuffer** ppBuffer, AMFBufferObserver* pObserver) = 0;
        virtual AMF_RESULT          AMF_STD_CALL CreateSurfaceFromHostNative(AMF_SURFACE_FORMAT format, amf_int32 width, amf_int32 height, amf_int32 hPitch, amf_int32 vPitch, void* pData, 
                                                     AMFSurface** ppSurface, AMFSurfaceObserver* pObserver) = 0;
        virtual AMF_RESULT          AMF_STD_CALL CreateSurfaceFromDX9Native(void* pDX9Surface, AMFSurface** ppSurface, AMFSurfaceObserver* pObserver) = 0;
        virtual AMF_RESULT          AMF_STD_CALL CreateSurfaceFromDX11Native(void* pDX11Surface, AMFSurface** ppSurface, AMFSurfaceObserver* pObserver) = 0;
        virtual AMF_RESULT          AMF_STD_CALL CreateSurfaceFromOpenGLNative(AMF_SURFACE_FORMAT format, amf_handle hGLTextureID, AMFSurface** ppSurface, AMFSurfaceObserver* pObserver) = 0;
        virtual AMF_RESULT          AMF_STD_CALL CreateSurfaceFromGrallocNative(amf_handle hGrallocSurface, AMFSurface** ppSurface, AMFSurfaceObserver* pObserver) = 0;
        virtual AMF_RESULT          AMF_STD_CALL CreateSurfaceFromOpenCLNative(AMF_SURFACE_FORMAT format, amf_int32 width, amf_int32 height, void** pClPlanes, 
                                                     AMFSurface** ppSurface, AMFSurfaceObserver* pObserver) = 0;
        virtual AMF_RESULT          AMF_STD_CALL CreateBufferFromOpenCLNative(void* pCLBuffer, amf_size size, AMFBuffer** ppBuffer) = 0;

        // Access to AMFCompute interface - AMF_MEMORY_OPENCL, AMF_MEMORY_COMPUTE_FOR_DX9, AMF_MEMORY_COMPUTE_FOR_DX11 are currently supported
        virtual AMF_RESULT          AMF_STD_CALL GetCompute(AMF_MEMORY_TYPE eMemType, AMFCompute** ppCompute) = 0;
    };
    //----------------------------------------------------------------------------------------------
    // smart pointer
    //----------------------------------------------------------------------------------------------
    typedef AMFInterfacePtr_T<AMFContext> AMFContextPtr;
    //----------------------------------------------------------------------------------------------
    // Lockers
    //----------------------------------------------------------------------------------------------
    class AMFContext::AMFDX9Locker
    {
    public:
        AMFDX9Locker() : m_Context(NULL)
        {}
        AMFDX9Locker(AMFContext* resources) : m_Context(NULL)
        {
            Lock(resources);
        }
        ~AMFDX9Locker()
        {
            if(m_Context != NULL)
            {
                m_Context->UnlockDX9();
            }
        }
        void Lock(AMFContext* resources)
        {
            if(m_Context != NULL)
            {
                m_Context->UnlockDX9();
            }
            m_Context = resources;
            if(m_Context != NULL)
            {
                m_Context->LockDX9();
            }
        }
    protected:
        AMFContext* m_Context;

    private:
        AMFDX9Locker(const AMFDX9Locker&);
        AMFDX9Locker& operator=(const AMFDX9Locker&);
    };
    //----------------------------------------------------------------------------------------------
    class AMFContext::AMFDX11Locker
    {
    public:
        AMFDX11Locker() : m_Context(NULL)
        {}
        AMFDX11Locker(AMFContext* resources) : m_Context(NULL)
        {
            Lock(resources);
        }
        ~AMFDX11Locker()
        {
            if(m_Context != NULL)
            {
                m_Context->UnlockDX11();
            }
        }
        void Lock(AMFContext* resources)
        {
            if(m_Context != NULL)
            {
                m_Context->UnlockDX11();
            }
            m_Context = resources;
            if(m_Context != NULL)
            {
                m_Context->LockDX11();
            }
        }
    protected:
        AMFContext* m_Context;

    private:
        AMFDX11Locker(const AMFDX11Locker&);
        AMFDX11Locker& operator=(const AMFDX11Locker&);
    };
    //----------------------------------------------------------------------------------------------
    class AMFContext::AMFOpenCLLocker
    {
    public:
        AMFOpenCLLocker() : m_Context(NULL)
        {}
        AMFOpenCLLocker(AMFContext* resources) : m_Context(NULL)
        {
            Lock(resources);
        }
        ~AMFOpenCLLocker()
        {
            if(m_Context != NULL)
            {
                m_Context->UnlockOpenCL();
            }
        }
        void Lock(AMFContext* resources)
        {
            if(m_Context != NULL)
            {
                m_Context->UnlockOpenCL();
            }
            m_Context = resources;
            if(m_Context != NULL)
            {
                m_Context->LockOpenCL();
            }
        }
    protected:
        AMFContext* m_Context;
    private:
        AMFOpenCLLocker(const AMFOpenCLLocker&);
        AMFOpenCLLocker& operator=(const AMFOpenCLLocker&);
    };
    //----------------------------------------------------------------------------------------------
    class AMFContext::AMFOpenGLLocker
    {
    public:
        AMFOpenGLLocker(AMFContext* pContext) : m_pContext(pContext),
            m_GLLocked(false)
        {
            if(m_pContext != NULL)
            {
                if(m_pContext->LockOpenGL() == AMF_OK)
                {
                    m_GLLocked = true;
                }
            }
        }
        ~AMFOpenGLLocker()
        {
            if(m_GLLocked)
            {
                m_pContext->UnlockOpenGL();
            }
        }
    private:
        AMFContext* m_pContext;
        amf_bool m_GLLocked; ///< AMFOpenGLLocker can be called when OpenGL is not initialized yet
                             ///< in this case don't call UnlockOpenGL
        AMFOpenGLLocker(const AMFOpenGLLocker&);
        AMFOpenGLLocker& operator=(const AMFOpenGLLocker&);
    };
    //----------------------------------------------------------------------------------------------
    class AMFContext::AMFXVLocker
    {
    public:
        AMFXVLocker() : m_pContext(NULL)
        {}
        AMFXVLocker(AMFContext* pContext) : m_pContext(NULL)
        {
            Lock(pContext);
        }
        ~AMFXVLocker()
        {
            if(m_pContext != NULL)
            {
                m_pContext->UnlockXV();
            }
        }
        void Lock(AMFContext* pContext)
        {
            if((pContext != NULL) && (pContext->GetXVDevice() != NULL))
            {
                m_pContext = pContext;
                m_pContext->LockXV();
            }
        }
    protected:
        AMFContext* m_pContext;
    private:
        AMFXVLocker(const AMFXVLocker&);
        AMFXVLocker& operator=(const AMFXVLocker&);
    };
    //----------------------------------------------------------------------------------------------
    class AMFContext::AMFGrallocLocker
    {
    public:
        AMFGrallocLocker() : m_pContext(NULL)
        {}
        AMFGrallocLocker(AMFContext* pContext) : m_pContext(NULL)
        {
            Lock(pContext);
        }
        ~AMFGrallocLocker()
        {
            if(m_pContext != NULL)
            {
                m_pContext->UnlockGralloc();
            }
        }
        void Lock(AMFContext* pContext)
        {
            if((pContext != NULL) && (pContext->GetGrallocDevice() != NULL))
            {
                m_pContext = pContext;
                m_pContext->LockGralloc();
            }
        }
    protected:
        AMFContext* m_pContext;
    private:
        AMFGrallocLocker(const AMFGrallocLocker&);
        AMFGrallocLocker& operator=(const AMFGrallocLocker&);
    };
    //----------------------------------------------------------------------------------------------
    //----------------------------------------------------------------------------------------------
    //----------------------------------------------------------------------------------------------
}

#endif //#ifndef __AMFContext_h__
