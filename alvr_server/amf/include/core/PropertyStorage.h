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

#ifndef __AMFPropertyStorage_h__
#define __AMFPropertyStorage_h__
#pragma once

#include "Variant.h"

namespace amf
{
    //----------------------------------------------------------------------------------------------
    // AMFPropertyStorageObserver interface
    //----------------------------------------------------------------------------------------------
    class AMF_NO_VTABLE AMFPropertyStorageObserver
    {
    public:
        virtual void                AMF_STD_CALL OnPropertyChanged(const wchar_t* name) = 0;
    };
    //----------------------------------------------------------------------------------------------
    // AMFPropertyStorage interface
    //----------------------------------------------------------------------------------------------
    class AMF_NO_VTABLE AMFPropertyStorage : public AMFInterface
    {
    public:
        AMF_DECLARE_IID(0xc7cec05b, 0xcfb9, 0x48af, 0xac, 0xe3, 0xf6, 0x8d, 0xf8, 0x39, 0x5f, 0xe3)

        virtual AMF_RESULT          AMF_STD_CALL SetProperty(const wchar_t* name, AMFVariantStruct value) = 0;
        virtual AMF_RESULT          AMF_STD_CALL GetProperty(const wchar_t* name, AMFVariantStruct* pValue) const = 0;

        virtual bool                AMF_STD_CALL HasProperty(const wchar_t* name) const = 0;
        virtual amf_size            AMF_STD_CALL GetPropertyCount() const = 0;
        virtual AMF_RESULT          AMF_STD_CALL GetPropertyAt(amf_size index, wchar_t* name, amf_size nameSize, AMFVariantStruct* pValue) const = 0;

        virtual AMF_RESULT          AMF_STD_CALL Clear() = 0;
        virtual AMF_RESULT          AMF_STD_CALL AddTo(AMFPropertyStorage* pDest, bool overwrite, bool deep) const= 0;
        virtual AMF_RESULT          AMF_STD_CALL CopyTo(AMFPropertyStorage* pDest, bool deep) const = 0;

        virtual void                AMF_STD_CALL AddObserver(AMFPropertyStorageObserver* pObserver) = 0;
        virtual void                AMF_STD_CALL RemoveObserver(AMFPropertyStorageObserver* pObserver) = 0;

        template<typename _T>
        AMF_RESULT                  AMF_STD_CALL SetProperty(const wchar_t* name, const _T& value);
        template<typename _T>
        AMF_RESULT                  AMF_STD_CALL GetProperty(const wchar_t* name, _T* pValue) const;
        template<typename _T>
        AMF_RESULT                  AMF_STD_CALL GetPropertyString(const wchar_t* name, _T* pValue) const;
        template<typename _T>
        AMF_RESULT                  AMF_STD_CALL GetPropertyWString(const wchar_t* name, _T* pValue) const;

    };
    //----------------------------------------------------------------------------------------------
    // template methods implementations
    //----------------------------------------------------------------------------------------------
    template<typename _T> inline
    AMF_RESULT AMF_STD_CALL AMFPropertyStorage::SetProperty(const wchar_t* name, const _T& value)
    {
        AMF_RESULT err = SetProperty(name, static_cast<const AMFVariantStruct&>(AMFVariant(value)));
        return err;
    }
    //----------------------------------------------------------------------------------------------
    template<typename _T> inline
    AMF_RESULT AMF_STD_CALL AMFPropertyStorage::GetProperty(const wchar_t* name, _T* pValue) const
    {
        AMFVariant var;
        AMF_RESULT err = GetProperty(name, static_cast<AMFVariantStruct*>(&var));
        if(err == AMF_OK)
        {
            *pValue = static_cast<_T>(var);
        }
        return err;
    }
    //----------------------------------------------------------------------------------------------
    template<typename _T> inline
    AMF_RESULT AMF_STD_CALL AMFPropertyStorage::GetPropertyString(const wchar_t* name, _T* pValue) const
    {
        AMFVariant var;
        AMF_RESULT err = GetProperty(name, static_cast<AMFVariantStruct*>(&var));
        if(err == AMF_OK)
        {
            *pValue = var.ToString().c_str();
        }
        return err;
    }
    //----------------------------------------------------------------------------------------------
    template<typename _T> inline
    AMF_RESULT AMF_STD_CALL AMFPropertyStorage::GetPropertyWString(const wchar_t* name, _T* pValue) const
    {
        AMFVariant var;
        AMF_RESULT err = GetProperty(name, static_cast<AMFVariantStruct*>(&var));
        if(err == AMF_OK)
        {
            *pValue = var.ToWString().c_str();
        }
        return err;
    }
    //----------------------------------------------------------------------------------------------
    template<> inline
    AMF_RESULT AMF_STD_CALL AMFPropertyStorage::GetProperty(const wchar_t* name,
            AMFInterface** ppValue) const
    {
        AMFVariant var;
        AMF_RESULT err = GetProperty(name, static_cast<AMFVariantStruct*>(&var));
        if(err == AMF_OK)
        {
            *ppValue = static_cast<AMFInterface*>(var);
        }
        if(*ppValue)
        {
            (*ppValue)->Acquire();
        }
        return err;
    }
    //----------------------------------------------------------------------------------------------
    // smart pointer
    //----------------------------------------------------------------------------------------------
    typedef AMFInterfacePtr_T<AMFPropertyStorage> AMFPropertyStoragePtr;
    //----------------------------------------------------------------------------------------------
} //namespace amf

#endif // #ifndef __AMFPropertyStorage_h__
