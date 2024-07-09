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
///  @file   PropertyStorageImpl.h
///  @brief  AMFPropertyStorageImpl header
///-------------------------------------------------------------------------
#ifndef AMF_PropertyStorageImpl_h
#define AMF_PropertyStorageImpl_h
#pragma once

#include "../include/core/PropertyStorage.h"
#include "Thread.h"
#include "InterfaceImpl.h"
#include "ObservableImpl.h"
#include "TraceAdapter.h"

namespace amf
{
    //---------------------------------------------------------------------------------------------
    template<typename _TBase> class AMFPropertyStorageImpl :
        public _TBase, 
        public AMFObservableImpl<AMFPropertyStorageObserver>
    {
    public:
        //-------------------------------------------------------------------------------------------------
        AMFPropertyStorageImpl() : m_PropertyValues()
        {
        }
        //-------------------------------------------------------------------------------------------------
        virtual ~AMFPropertyStorageImpl()
        {
        }
        //-------------------------------------------------------------------------------------------------
        // interface access
        AMF_BEGIN_INTERFACE_MAP
            AMF_INTERFACE_ENTRY(AMFPropertyStorage)
        AMF_END_INTERFACE_MAP
        //-------------------------------------------------------------------------------------------------
        virtual AMF_RESULT  AMF_STD_CALL SetProperty(const wchar_t* pName, AMFVariantStruct value)
        {
            AMF_RETURN_IF_INVALID_POINTER(pName);

            m_PropertyValues[pName] = value;
            OnPropertyChanged(pName);
            NotifyObservers<const wchar_t*>(&AMFPropertyStorageObserver::OnPropertyChanged, pName);
            return AMF_OK;
        }
        //-------------------------------------------------------------------------------------------------
        virtual AMF_RESULT  AMF_STD_CALL GetProperty(const wchar_t* pName, AMFVariantStruct* pValue) const
        {
            AMF_RETURN_IF_INVALID_POINTER(pName);
            AMF_RETURN_IF_INVALID_POINTER(pValue);

            amf_wstring name(pName);
            amf_map<amf_wstring, AMFVariant>::const_iterator found = m_PropertyValues.find(name);
            if(found != m_PropertyValues.end())
            {
                AMFVariantCopy(pValue, &found->second);
                return AMF_OK;
            }
            return AMF_NOT_FOUND;
        }
        //-------------------------------------------------------------------------------------------------
        virtual bool        AMF_STD_CALL HasProperty(const wchar_t* pName) const
        {
            AMF_ASSERT(pName != NULL);
            return m_PropertyValues.find(pName) != m_PropertyValues.end();
        }
        //-------------------------------------------------------------------------------------------------
        virtual amf_size    AMF_STD_CALL GetPropertyCount() const
        {
            return m_PropertyValues.size();
        }
        //-------------------------------------------------------------------------------------------------
        virtual AMF_RESULT  AMF_STD_CALL GetPropertyAt(amf_size index, wchar_t* pName, amf_size nameSize, AMFVariantStruct* pValue) const
        {
            AMF_RETURN_IF_INVALID_POINTER(pName);
            AMF_RETURN_IF_INVALID_POINTER(pValue);
            AMF_RETURN_IF_FALSE(nameSize != 0, AMF_INVALID_ARG);
            amf_map<amf_wstring, AMFVariant>::const_iterator found = m_PropertyValues.begin();
            if(found == m_PropertyValues.end())
            {
                return AMF_INVALID_ARG;
            }
            for( amf_size i = 0; i < index; i++)
            {
                found++;
                if(found == m_PropertyValues.end())
                {
                    return AMF_INVALID_ARG;
                }
            }
            size_t copySize = AMF_MIN(nameSize-1, found->first.length());
            memcpy(pName, found->first.c_str(), copySize * sizeof(wchar_t));
            pName[copySize] = 0;
            AMFVariantCopy(pValue, &found->second);
            return AMF_OK;
        }
        //-------------------------------------------------------------------------------------------------
        virtual AMF_RESULT  AMF_STD_CALL Clear()
        {
            m_PropertyValues.clear();
            return AMF_OK;
        }
        //-------------------------------------------------------------------------------------------------
        virtual AMF_RESULT  AMF_STD_CALL AddTo(AMFPropertyStorage* pDest, bool overwrite, bool /*deep*/) const
        {
            AMF_RETURN_IF_INVALID_POINTER(pDest);
            AMF_RESULT err = AMF_OK;
            amf_map<amf_wstring, AMFVariant>::const_iterator it = m_PropertyValues.begin();

            for(; it != m_PropertyValues.end(); it++)
            {
                if(!HasProperty(it->first.c_str())) // ignore properties which aren't accessible
                {
                    continue;
                }

                if(!overwrite)
                {
                    if(pDest->HasProperty(it->first.c_str()))
                    {
                        continue;
                    }
                }
                {
                    err = pDest->SetProperty(it->first.c_str(), it->second);
                }
                if(err == AMF_ACCESS_DENIED)
                {
                    continue;
                }
                AMF_RETURN_IF_FAILED(err, L"AddTo() - failed to copy property=%s", it->first.c_str());
            }
            return AMF_OK;
        }        
        //-------------------------------------------------------------------------------------------------
        virtual AMF_RESULT  AMF_STD_CALL CopyTo(AMFPropertyStorage* pDest, bool deep) const
        {
            AMF_RETURN_IF_INVALID_POINTER(pDest);
            if(pDest != this)
            {
                pDest->Clear();
                return AddTo(pDest, true, deep);
            }
            else
            {
                return AMF_OK;
            }
        }
        //-------------------------------------------------------------------------------------------------
        virtual void        AMF_STD_CALL OnPropertyChanged(const wchar_t* /*name*/) { }
        //-------------------------------------------------------------------------------------------------
        virtual void        AMF_STD_CALL AddObserver(AMFPropertyStorageObserver* pObserver) { AMFObservableImpl<AMFPropertyStorageObserver>::AddObserver(pObserver); }
        //-------------------------------------------------------------------------------------------------
        virtual void        AMF_STD_CALL RemoveObserver(AMFPropertyStorageObserver* pObserver) { AMFObservableImpl<AMFPropertyStorageObserver>::RemoveObserver(pObserver); }
        //-------------------------------------------------------------------------------------------------
    protected:
        //-------------------------------------------------------------------------------------------------
        amf_map<amf_wstring, AMFVariant> m_PropertyValues;
    };
    //---------------------------------------------------------------------------------------------
    //---------------------------------------------------------------------------------------------
}
#endif // AMF_PropertyStorageImpl_h