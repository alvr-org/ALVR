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

#ifndef AMF_InterfaceImpl_h
#define AMF_InterfaceImpl_h

#pragma once

#include "../include/core/Interface.h"
#include "Thread.h"

#pragma warning(disable : 4511)
namespace amf
{
    #define AMF_BEGIN_INTERFACE_MAP  \
        virtual AMF_RESULT AMF_STD_CALL QueryInterface(const amf::AMFGuid & interfaceID, void** ppInterface) \
        {  \
            AMF_RESULT err = AMF_NO_INTERFACE; \


    #define AMF_INTERFACE_ENTRY(T)  \
        if(AMFCompareGUIDs(interfaceID, T::IID())) \
        { \
            *ppInterface = (void*)static_cast<T*>(this); \
            this->Acquire(); \
            err = AMF_OK; \
        } \
        else \

    #define AMF_INTERFACE_ENTRY_THIS(T, _TI)  \
        if(AMFCompareGUIDs(interfaceID, T::IID())) \
        { \
            *ppInterface = (void*)static_cast<T*>(static_cast<_TI*>(this)); \
            this->Acquire(); \
            err = AMF_OK; \
        } \
        else \

    #define AMF_INTERFACE_MULTI_ENTRY(T)  \
        if(AMFCompareGUIDs(interfaceID, T::IID())) \
        { \
            *ppInterface = (void*)static_cast<T*>(this); \
            AcquireInternal(); \
            err = AMF_OK; \
        } \
        else \

    #define AMF_INTERFACE_CHAIN_ENTRY(T)  \
        if(static_cast<T&>(*this).T::QueryInterface(interfaceID, ppInterface) == AMF_OK) \
        {err = AMF_OK;} \
        else \

    //good as an example but we should not use aggregate pattern without big reason - very hard to debug
    #define AMF_INTERFACE_AGREGATED_ENTRY(T, _Ptr)  \
        if(AMFCompareGUIDs(interfaceID, T::IID())) \
        { \
            T* ptr = static_cast<T*>(_Ptr); \
            *ppInterface = (void*)ptr; \
            ptr->Acquire(); \
            err = AMF_OK; \
        } \
        else \

    #define AMF_INTERFACE_CHAIN_AGREGATED_ENTRY(T, _Ptr)  \
        if(err = static_cast<T*>(_Ptr)->QueryInterface(interfaceID, ppInterface)) { \
        } \
        else \

    #define AMF_END_INTERFACE_MAP \
        {} \
        return err; \
        } \


    //---------------------------------------------------------------
    class AMFInterfaceBase
    {
    protected:
        amf_long m_refCount;
        virtual ~AMFInterfaceBase()
#if __GNUC__ == 11 //WORKAROUND for gcc-11 bug
        __attribute__ ((noinline))
#endif
        {}
    public:
        AMFInterfaceBase() : m_refCount(0)
        {}
        virtual amf_long AMF_STD_CALL AcquireInternal()
        {
            amf_long newVal = amf_atomic_inc(&m_refCount);
            return newVal;
        }
        virtual amf_long AMF_STD_CALL ReleaseInternal()
        {
            amf_long newVal = amf_atomic_dec(&m_refCount);
            if(newVal == 0)
            {
                delete this;
            }
            return newVal;
        }
        virtual amf_long AMF_STD_CALL RefCountInternal()
        {
            return m_refCount;
        }
    };
    //---------------------------------------------------------------
    template<class _Base , typename _Param1 = int, typename _Param2 = int, typename _Param3 = int>
    class AMFInterfaceImpl : public _Base, public AMFInterfaceBase
    {
    protected:
        virtual ~AMFInterfaceImpl()
        {}
    public:
        AMFInterfaceImpl(_Param1 param1, _Param2 param2, _Param3 param3) : _Base(param1, param2, param3)
        {}
        AMFInterfaceImpl(_Param1 param1, _Param2 param2) : _Base(param1, param2)
        {}
        AMFInterfaceImpl(_Param1 param1) : _Base(param1)
        {}
        AMFInterfaceImpl()
        {}
        virtual amf_long AMF_STD_CALL Acquire()
        {
            return AMFInterfaceBase::AcquireInternal();
        }
        virtual amf_long AMF_STD_CALL Release()
        {
            return AMFInterfaceBase::ReleaseInternal();
        }
        virtual amf_long AMF_STD_CALL RefCount()
        {
            return AMFInterfaceBase::RefCountInternal();
        }

        AMF_BEGIN_INTERFACE_MAP
            AMF_INTERFACE_ENTRY(AMFInterface)
            AMF_INTERFACE_ENTRY(_Base)
        AMF_END_INTERFACE_MAP
    };

    //---------------------------------------------------------------
    template<class _Base, class _BaseInterface, typename _Param1 = int, typename _Param2 = int, typename _Param3 = int, typename _Param4 = int, typename _Param5 = int, typename _Param6 = int>
    class AMFInterfaceMultiImpl : public _Base
    {
    protected:
        virtual ~AMFInterfaceMultiImpl()
        {}
    public:
        AMFInterfaceMultiImpl(_Param1 param1, _Param2 param2, _Param3 param3, _Param4 param4, _Param5 param5, _Param6 param6) : _Base(param1, param2, param3, param4, param5, param6)
        {}
        AMFInterfaceMultiImpl(_Param1 param1, _Param2 param2, _Param3 param3, _Param4 param4, _Param5 param5) : _Base(param1, param2, param3, param4, param5)
        {}
        AMFInterfaceMultiImpl(_Param1 param1, _Param2 param2, _Param3 param3, _Param4 param4) : _Base(param1, param2, param3, param4)
        {}
        AMFInterfaceMultiImpl(_Param1 param1, _Param2 param2, _Param3 param3) : _Base(param1, param2, param3)
        {}
        AMFInterfaceMultiImpl(_Param1 param1, _Param2 param2) : _Base(param1, param2)
        {}
        AMFInterfaceMultiImpl(_Param1 param1) : _Base(param1)
        {}
        AMFInterfaceMultiImpl()
        {}
        virtual amf_long AMF_STD_CALL Acquire()
        {
            return AMFInterfaceBase::AcquireInternal();
        }
        virtual amf_long AMF_STD_CALL Release()
        {
            return AMFInterfaceBase::ReleaseInternal();
        }
        virtual amf_long AMF_STD_CALL RefCount()
        {
            return AMFInterfaceBase::RefCountInternal();
        }

        AMF_BEGIN_INTERFACE_MAP
            AMF_INTERFACE_ENTRY_THIS(AMFInterface, _BaseInterface)
            AMF_INTERFACE_CHAIN_ENTRY(_Base)
        AMF_END_INTERFACE_MAP
    };


} // namespace amf
#endif // AMF_InterfaceImpl_h