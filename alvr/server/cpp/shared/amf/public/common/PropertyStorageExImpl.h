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
///  @file   PropertyStorageExImpl.h
///  @brief  AMFPropertyStorageExImpl header
///-------------------------------------------------------------------------
#ifndef AMF_PropertyStorageExImpl_h
#define AMF_PropertyStorageExImpl_h
#pragma once

#include "../include/core/PropertyStorageEx.h"
#include "Thread.h"
#include "InterfaceImpl.h"
#include "ObservableImpl.h"
#include "TraceAdapter.h"
#include <limits.h>
#include <float.h>
#include <memory>

namespace amf
{

    AMF_RESULT CastVariantToAMFProperty(AMFVariantStruct* pDest, const AMFVariantStruct* pSrc, AMF_VARIANT_TYPE eType,
        AMF_PROPERTY_CONTENT_TYPE contentType,
        const AMFEnumDescriptionEntry* pEnumDescription = 0);

    //---------------------------------------------------------------------------------------------
    class AMFPropertyInfoImpl : public AMFPropertyInfo
    {
    private:
        amf_wstring m_name;
        amf_wstring m_desc;

        void Init(const wchar_t* name, const wchar_t* desc, AMF_VARIANT_TYPE type, AMF_PROPERTY_CONTENT_TYPE contentType,
            AMFVariantStruct defaultValue, AMFVariantStruct minValue, AMFVariantStruct maxValue, AMF_PROPERTY_ACCESS_TYPE accessType,
            const AMFEnumDescriptionEntry* pEnumDescription);

    public:
        AMFVariant  value;
        amf_bool    userModified = false;

    public:
        AMFPropertyInfoImpl(const wchar_t* name, const wchar_t* desc, AMF_VARIANT_TYPE type, AMF_PROPERTY_CONTENT_TYPE contentType,
            AMFVariantStruct defaultValue, AMFVariantStruct minValue, AMFVariantStruct maxValue, bool allowChangeInRuntime,
            const AMFEnumDescriptionEntry* pEnumDescription);
        AMFPropertyInfoImpl(const wchar_t* name, const wchar_t* desc, AMF_VARIANT_TYPE type, AMF_PROPERTY_CONTENT_TYPE contentType,
            AMFVariantStruct defaultValue, AMFVariantStruct minValue, AMFVariantStruct maxValue, AMF_PROPERTY_ACCESS_TYPE accessType,
            const AMFEnumDescriptionEntry* pEnumDescription);
        AMFPropertyInfoImpl();

        AMFPropertyInfoImpl(const AMFPropertyInfoImpl& propertyInfo);
        AMFPropertyInfoImpl& operator=(const AMFPropertyInfoImpl& propertyInfo);

        virtual ~AMFPropertyInfoImpl();

        virtual void  OnPropertyChanged() { }
    };

    typedef amf_map<amf_wstring, std::shared_ptr<AMFPropertyInfoImpl> >  PropertyInfoMap;

    //---------------------------------------------------------------------------------------------
    template<typename _TBase> class AMFPropertyStorageExImpl :
        public _TBase,
        public AMFObservableImpl<AMFPropertyStorageObserver>
    {
    protected:
        PropertyInfoMap   m_PropertiesInfo;

    public:
        AMFPropertyStorageExImpl()
        {
        }

        virtual ~AMFPropertyStorageExImpl()
        {
        }


        // interface access
        AMF_BEGIN_INTERFACE_MAP
            AMF_INTERFACE_ENTRY(AMFPropertyStorage)
            AMF_INTERFACE_ENTRY(AMFPropertyStorageEx)
        AMF_END_INTERFACE_MAP


        using _TBase::GetProperty;
        using _TBase::SetProperty;

        // interface
        //-------------------------------------------------------------------------------------------------
        virtual AMF_RESULT  AMF_STD_CALL Clear()
        {
            ResetDefaultValues();
            return AMF_OK;
        }
        //-------------------------------------------------------------------------------------------------
        virtual AMF_RESULT  AMF_STD_CALL AddTo(AMFPropertyStorage* pDest, bool overwrite, bool /*deep*/) const
        {
            AMF_RETURN_IF_INVALID_POINTER(pDest);

            if (pDest != this)
            {
                for (PropertyInfoMap::const_iterator it = m_PropertiesInfo.begin(); it != m_PropertiesInfo.end(); it++)
                {
                    if (!overwrite && pDest->HasProperty(it->first.c_str()))
                    {
                        continue;
                    }

                    AMF_RESULT err = pDest->SetProperty(it->first.c_str(), it->second->value);
                    if (err != AMF_INVALID_ARG) // not validated - skip it
                    {
                        AMF_RETURN_IF_FAILED(err, L"AddTo() - failed to copy property=%s", it->first.c_str());
                    }
                }
            }

            return AMF_OK;
        }
        //-------------------------------------------------------------------------------------------------
        virtual AMF_RESULT  AMF_STD_CALL CopyTo(AMFPropertyStorage* pDest, bool deep) const
        {
            AMF_RETURN_IF_INVALID_POINTER(pDest);

            if (pDest != this)
            {
                pDest->Clear();
                return AddTo(pDest, true, deep);
            }

            return AMF_OK;
        }
        //-------------------------------------------------------------------------------------------------
        virtual AMF_RESULT  AMF_STD_CALL SetProperty(const wchar_t* name, AMFVariantStruct value)
        {
            AMF_RETURN_IF_INVALID_POINTER(name);

            const AMFPropertyInfo* pParamInfo = NULL;
            AMF_RESULT err = GetPropertyInfo(name, &pParamInfo);
            if (err != AMF_OK)
            {
                return err;
            }

            if (pParamInfo && !pParamInfo->AllowedWrite())
            {
                return AMF_ACCESS_DENIED;
            }
            return SetPrivateProperty(name, value);
        }
        //-------------------------------------------------------------------------------------------------
        virtual AMF_RESULT  AMF_STD_CALL GetProperty(const wchar_t* name, AMFVariantStruct* pValue) const
        {
            AMF_RETURN_IF_INVALID_POINTER(name);
            AMF_RETURN_IF_INVALID_POINTER(pValue);

            const AMFPropertyInfo* pParamInfo = NULL;
            AMF_RESULT err = GetPropertyInfo(name, &pParamInfo);
            if (err != AMF_OK)
            {
                return err;
            }

            if (pParamInfo && !pParamInfo->AllowedRead())
            {
                return AMF_ACCESS_DENIED;
            }
            return GetPrivateProperty(name, pValue);
        }
        //-------------------------------------------------------------------------------------------------
        virtual bool        AMF_STD_CALL HasProperty(const wchar_t* name) const
        {
            const AMFPropertyInfo* pParamInfo = NULL;
            AMF_RESULT err = GetPropertyInfo(name, &pParamInfo);
            return (err != AMF_OK) ? false : true;
        }
        //-------------------------------------------------------------------------------------------------
        virtual amf_size    AMF_STD_CALL GetPropertyCount() const
        {
            return m_PropertiesInfo.size();
        }
        //-------------------------------------------------------------------------------------------------
        virtual AMF_RESULT  AMF_STD_CALL GetPropertyAt(amf_size index, wchar_t* name, amf_size nameSize, AMFVariantStruct* pValue) const
        {
            AMF_RETURN_IF_INVALID_POINTER(name);
            AMF_RETURN_IF_INVALID_POINTER(pValue);
            AMF_RETURN_IF_FALSE(nameSize != 0, AMF_INVALID_ARG);
            AMF_RETURN_IF_FALSE(index < m_PropertiesInfo.size(), AMF_INVALID_ARG);

            PropertyInfoMap::const_iterator found = m_PropertiesInfo.begin();
            for (amf_size i = 0; i < index; i++)
            {
                found++;
            }

            size_t copySize = AMF_MIN(nameSize-1, found->first.length());
            memcpy(name, found->first.c_str(), copySize * sizeof(wchar_t));
            name[copySize] = 0;
            AMFVariantCopy(pValue, &found->second->value);
            return AMF_OK;
        }
        //-------------------------------------------------------------------------------------------------
        virtual amf_size    AMF_STD_CALL GetPropertiesInfoCount() const
        {
            return m_PropertiesInfo.size();
        }
        //-------------------------------------------------------------------------------------------------
        virtual AMF_RESULT  AMF_STD_CALL GetPropertyInfo(amf_size szInd, const AMFPropertyInfo** ppParamInfo) const
        {
            AMF_RETURN_IF_INVALID_POINTER(ppParamInfo);
            AMF_RETURN_IF_FALSE(szInd < m_PropertiesInfo.size(), AMF_INVALID_ARG);

            PropertyInfoMap::const_iterator it = m_PropertiesInfo.begin();
            for (; szInd > 0; --szInd)
            {
                it++;
            }

            *ppParamInfo = it->second.get();
            return AMF_OK;
        }
        //-------------------------------------------------------------------------------------------------
        virtual AMF_RESULT  AMF_STD_CALL GetPropertyInfo(const wchar_t* name, const AMFPropertyInfo** ppParamInfo) const
        {
            AMF_RETURN_IF_INVALID_POINTER(name);
            AMF_RETURN_IF_INVALID_POINTER(ppParamInfo);

            PropertyInfoMap::const_iterator it = m_PropertiesInfo.find(name);
            if (it != m_PropertiesInfo.end())
            {
                *ppParamInfo = it->second.get();
                return AMF_OK;
            }

            return AMF_NOT_FOUND;
        }
        //-------------------------------------------------------------------------------------------------
        virtual AMF_RESULT  AMF_STD_CALL ValidateProperty(const wchar_t* name, AMFVariantStruct value, AMFVariantStruct* pOutValidated) const
        {
            AMF_RETURN_IF_INVALID_POINTER(name);
            AMF_RETURN_IF_INVALID_POINTER(pOutValidated);

            AMF_RESULT err = AMF_OK;
            const AMFPropertyInfo* pParamInfo = NULL;

            AMF_RETURN_IF_FAILED(GetPropertyInfo(name, &pParamInfo), L"Property=%s", name);
            AMF_RETURN_IF_FAILED(CastVariantToAMFProperty(pOutValidated, &value, pParamInfo->type, pParamInfo->contentType, pParamInfo->pEnumDescription), L"Property=%s", name);

            switch(pParamInfo->type)
            {
            case AMF_VARIANT_INT64:
                if((pParamInfo->minValue.type != AMF_VARIANT_EMPTY && AMFVariantGetInt64(pOutValidated) < AMFVariantGetInt64(&pParamInfo->minValue)) ||
                    (pParamInfo->maxValue.type != AMF_VARIANT_EMPTY && AMFVariantGetInt64(pOutValidated) > AMFVariantGetInt64(&pParamInfo->maxValue)) )
                {
                    err = AMF_OUT_OF_RANGE;
                }
                break;

            case AMF_VARIANT_DOUBLE:
                if((AMFVariantGetDouble(pOutValidated) < AMFVariantGetDouble(&pParamInfo->minValue)) ||
                   (AMFVariantGetDouble(pOutValidated) > AMFVariantGetDouble(&pParamInfo->maxValue)) )
                {
                    err = AMF_OUT_OF_RANGE;
                }
                break;
            case AMF_VARIANT_FLOAT:
                if ((AMFVariantGetFloat(pOutValidated) < AMFVariantGetFloat(&pParamInfo->minValue)) ||
                    (AMFVariantGetFloat(pOutValidated) > AMFVariantGetFloat(&pParamInfo->maxValue)))
                {
                    err = AMF_OUT_OF_RANGE;
                }
                break;
            case AMF_VARIANT_RATE:
                {
                    // NOTE: denominator can't be 0
                    const AMFRate& validatedSize = AMFVariantGetRate(pOutValidated);
                          AMFRate  minSize       = AMFConstructRate(0, 1);
                          AMFRate  maxSize       = AMFConstructRate(INT_MAX, INT_MAX);
                    if (pParamInfo->minValue.type != AMF_VARIANT_EMPTY)
                    {
                        minSize = AMFVariantGetRate(&pParamInfo->minValue);
                    }
                    if (pParamInfo->maxValue.type != AMF_VARIANT_EMPTY)
                    {
                        maxSize = AMFVariantGetRate(&pParamInfo->maxValue);
                    }
                    if (validatedSize.num < minSize.num || validatedSize.num > maxSize.num ||
                        validatedSize.den < minSize.den || validatedSize.den > maxSize.den)
                    {
                        err = AMF_OUT_OF_RANGE;
                    }
                }
                break;
            case AMF_VARIANT_SIZE:
                {
                    AMFSize validatedSize = AMFVariantGetSize(pOutValidated);
                    AMFSize minSize = AMFConstructSize(0, 0);
                    AMFSize maxSize = AMFConstructSize(INT_MAX, INT_MAX);
                    if (pParamInfo->minValue.type != AMF_VARIANT_EMPTY)
                    {
                        minSize = AMFVariantGetSize(&pParamInfo->minValue);
                    }
                    if (pParamInfo->maxValue.type != AMF_VARIANT_EMPTY)
                    {
                        maxSize = AMFVariantGetSize(&pParamInfo->maxValue);
                    }
                    if (validatedSize.width < minSize.width || validatedSize.height < minSize.height ||
                        validatedSize.width > maxSize.width || validatedSize.height > maxSize.height)
                    {
                        err = AMF_OUT_OF_RANGE;
                    }
                }
                break;
            case AMF_VARIANT_FLOAT_SIZE:
                {
                    AMFFloatSize validatedSize = AMFVariantGetFloatSize(pOutValidated);
                    AMFFloatSize minSize = AMFConstructFloatSize(0, 0);
                    AMFFloatSize maxSize = AMFConstructFloatSize(FLT_MIN, FLT_MAX);
                    if (pParamInfo->minValue.type != AMF_VARIANT_EMPTY)
                    {
                        minSize = AMFVariantGetFloatSize(&pParamInfo->minValue);
                    }
                    if (pParamInfo->maxValue.type != AMF_VARIANT_EMPTY)
                    {
                        maxSize = AMFVariantGetFloatSize(&pParamInfo->maxValue);
                    }
                    if (validatedSize.width < minSize.width || validatedSize.height < minSize.height ||
                        validatedSize.width > maxSize.width || validatedSize.height > maxSize.height)
                    {
                        err = AMF_OUT_OF_RANGE;
                    }
                }
                break;
            default:    //  GK: Clang issues a warning when not every value of an enum is handled in a switch-case
                break;
            }
            return err;
        }
        //-------------------------------------------------------------------------------------------------
        virtual void        AMF_STD_CALL OnPropertyChanged(const wchar_t* /*name*/){ }
        //-------------------------------------------------------------------------------------------------
        virtual void        AMF_STD_CALL AddObserver(AMFPropertyStorageObserver* pObserver) { AMFObservableImpl<AMFPropertyStorageObserver>::AddObserver(pObserver); }
        //-------------------------------------------------------------------------------------------------
        virtual void        AMF_STD_CALL RemoveObserver(AMFPropertyStorageObserver* pObserver) { AMFObservableImpl<AMFPropertyStorageObserver>::RemoveObserver(pObserver); }
        //-------------------------------------------------------------------------------------------------
    protected:
        //-------------------------------------------------------------------------------------------------
        AMF_RESULT SetAccessType(const wchar_t* name, AMF_PROPERTY_ACCESS_TYPE accessType)
        {
            AMF_RETURN_IF_INVALID_POINTER(name);

            PropertyInfoMap::iterator found = m_PropertiesInfo.find(name);
            AMF_RETURN_IF_FALSE(found != m_PropertiesInfo.end(), AMF_NOT_FOUND);

            if (found->second->accessType == accessType)
            {
                return AMF_OK;
            }

            found->second->accessType = accessType;
            OnPropertyChanged(name);
            NotifyObservers<const wchar_t*>(&AMFPropertyStorageObserver::OnPropertyChanged, name);
            return AMF_OK;
        }
        //-------------------------------------------------------------------------------------------------
        AMF_RESULT SetPrivateProperty(const wchar_t* name, AMFVariantStruct value)
        {
            AMF_RETURN_IF_INVALID_POINTER(name);

            AMFVariant validatedValue;
            AMF_RESULT validateResult = ValidateProperty(name, value, &validatedValue);
            if (validateResult != AMF_OK)
            {
                return validateResult;
            }

            PropertyInfoMap::iterator found = m_PropertiesInfo.find(name);
            if (found == m_PropertiesInfo.end())
            {
                return AMF_NOT_FOUND;
            }

            if (found->second->value == validatedValue)
            {
                return AMF_OK;
            }

            found->second->value = validatedValue;
            found->second->OnPropertyChanged();
            OnPropertyChanged(name);
            NotifyObservers<const wchar_t*>(&AMFPropertyStorageObserver::OnPropertyChanged, name);

            return AMF_OK;
        }
        //-------------------------------------------------------------------------------------------------
        AMF_RESULT GetPrivateProperty(const wchar_t* name, AMFVariantStruct* pValue) const
        {
            AMF_RETURN_IF_INVALID_POINTER(name);
            AMF_RETURN_IF_INVALID_POINTER(pValue);

            PropertyInfoMap::const_iterator found = m_PropertiesInfo.find(name);
            if (found != m_PropertiesInfo.end())
            {
                AMFVariantCopy(pValue, &found->second->value);
                return AMF_OK;
            }

            // NOTE: needed for internal components that don't automatically
            //       expose their properties in the main map...
            const AMFPropertyInfo* pParamInfo;
            if (GetPropertyInfo(name, &pParamInfo) == AMF_OK)
            {
                AMFVariantCopy(pValue, &pParamInfo->defaultValue);
                return AMF_OK;
            }

            return AMF_NOT_FOUND;
        }
        //-------------------------------------------------------------------------------------------------
        template<typename _T>
        AMF_RESULT          AMF_STD_CALL SetPrivateProperty(const wchar_t* name, const _T& value)
        {
            AMF_RESULT err = SetPrivateProperty(name, static_cast<const AMFVariantStruct&>(AMFVariant(value)));
            return err;
        }
        //-------------------------------------------------------------------------------------------------
        template<typename _T>
        AMF_RESULT          AMF_STD_CALL GetPrivateProperty(const wchar_t* name, _T* pValue) const
        {
            AMFVariant var;
            AMF_RESULT err = GetPrivateProperty(name, static_cast<AMFVariantStruct*>(&var));
            if(err == AMF_OK)
            {
                *pValue = static_cast<_T>(var);
            }
            return err;
        }
        //-------------------------------------------------------------------------------------------------
        bool HasPrivateProperty(const wchar_t* name) const
        {
            return m_PropertiesInfo.find(name) != m_PropertiesInfo.end();
        }
        //-------------------------------------------------------------------------------------------------
        bool  IsRuntimeChange(const wchar_t* name) const
        {
            PropertyInfoMap::const_iterator it = m_PropertiesInfo.find(name);
            return (it != m_PropertiesInfo.end()) ? it->second->AllowedChangeInRuntime() : false;
        }
        //-------------------------------------------------------------------------------------------------
        void  ResetDefaultValues()
        {
            // copy defaults to property storage
            for (PropertyInfoMap::iterator it = m_PropertiesInfo.begin(); it != m_PropertiesInfo.end(); ++it)
            {
                AMFPropertyInfoImpl*  info = it->second.get();

                info->value = info->defaultValue;
                info->userModified = false;
            }
        }
        //-------------------------------------------------------------------------------------------------

    private:
        AMFPropertyStorageExImpl(const AMFPropertyStorageExImpl&);
        AMFPropertyStorageExImpl& operator=(const AMFPropertyStorageExImpl&);
    };
    extern AMFCriticalSection ms_csAMFPropertyStorageExImplMaps;
    //---------------------------------------------------------------------------------------------


#define AMFPrimitivePropertyInfoMapBegin \
        { \
            amf::AMFPropertyInfoImpl* s_PropertiesInfo[] = \
            { 

#define AMFPrimitivePropertyInfoMapEnd \
            }; \
            for (amf_size i = 0; i < sizeof(s_PropertiesInfo) / sizeof(s_PropertiesInfo[0]); ++i) \
            { \
                amf::AMFPropertyInfoImpl* pPropInfo = s_PropertiesInfo[i]; \
                m_PropertiesInfo[pPropInfo->name].reset(pPropInfo); \
            } \
    } 


    #define AMFPropertyInfoBool(_name, _desc, _defaultValue, _AccessType) \
        new amf::AMFPropertyInfoImpl(_name, _desc, amf::AMF_VARIANT_BOOL, 0, amf::AMFVariant(_defaultValue), \
                                     amf::AMFVariant(), amf::AMFVariant(), _AccessType, 0)

    #define AMFPropertyInfoEnum(_name, _desc, _defaultValue, pEnumDescription, _AccessType) \
        new amf::AMFPropertyInfoImpl(_name, _desc, amf::AMF_VARIANT_INT64, 0, amf::AMFVariant(amf_int64(_defaultValue)), \
                                     amf::AMFVariant(), amf::AMFVariant(), _AccessType, pEnumDescription)

    #define AMFPropertyInfoInt64(_name, _desc, _defaultValue, _minValue, _maxValue, _AccessType) \
        new amf::AMFPropertyInfoImpl(_name, _desc, amf::AMF_VARIANT_INT64, 0, amf::AMFVariant(amf_int64(_defaultValue)), \
                                     amf::AMFVariant(amf_int64(_minValue)), amf::AMFVariant(amf_int64(_maxValue)), _AccessType, 0)

    #define AMFPropertyInfoDouble(_name, _desc, _defaultValue, _minValue, _maxValue, _AccessType) \
        new amf::AMFPropertyInfoImpl(_name, _desc, amf::AMF_VARIANT_DOUBLE, 0, amf::AMFVariant(amf_double(_defaultValue)), \
                                     amf::AMFVariant(amf_double(_minValue)), amf::AMFVariant(amf_double(_maxValue)), _AccessType, 0)

    #define AMFPropertyInfoFloat(_name, _desc, _defaultValue, _minValue, _maxValue, _AccessType) \
        new amf::AMFPropertyInfoImpl(_name, _desc, amf::AMF_VARIANT_FLOAT, 0, amf::AMFVariant(amf_float(_defaultValue)), \
                                     amf::AMFVariant(amf_float(_minValue)), amf::AMFVariant(amf_float(_maxValue)), _AccessType, 0)


    #define AMFPropertyInfoRect(_name, _desc, defaultLeft, defaultTop, defaultRight, defaultBottom, _AccessType) \
        new amf::AMFPropertyInfoImpl(_name, _desc, amf::AMF_VARIANT_RECT, 0, amf::AMFVariant(AMFConstructRect(defaultLeft, defaultTop, defaultRight, defaultBottom)), \
                                     amf::AMFVariant(), amf::AMFVariant(), _AccessType, 0)

    #define AMFPropertyInfoPoint(_name, _desc, defaultX, defaultY, _AccessType) \
        new amf::AMFPropertyInfoImpl(_name, _desc, amf::AMF_VARIANT_POINT, 0, amf::AMFVariant(AMFConstructPoint(defaultX, defaultY)), \
                                     amf::AMFVariant(), amf::AMFVariant(), _AccessType, 0)

    #define AMFPropertyInfoSize(_name, _desc, _defaultValue, _minValue, _maxValue, _AccessType) \
        new amf::AMFPropertyInfoImpl(_name, _desc, amf::AMF_VARIANT_SIZE, 0, amf::AMFVariant(AMFSize(_defaultValue)), \
                                     amf::AMFVariant(AMFSize(_minValue)), amf::AMFVariant(AMFSize(_maxValue)), _AccessType, 0)

    #define AMFPropertyInfoFloatSize(_name, _desc, _defaultValue, _minValue, _maxValue, _AccessType) \
        new amf::AMFPropertyInfoImpl(_name, _desc, amf::AMF_VARIANT_FLOAT_SIZE, 0, amf::AMFVariant(AMFFloatSize(_defaultValue)), \
                                     amf::AMFVariant(AMFFloatSize(_minValue)), amf::AMFVariant(AMFFloatSize(_maxValue)), _AccessType, 0)

    #define AMFPropertyInfoRate(_name, _desc, defaultNum, defaultDen, _AccessType) \
        new amf::AMFPropertyInfoImpl(_name, _desc, amf::AMF_VARIANT_RATE, 0, amf::AMFVariant(AMFConstructRate(defaultNum, defaultDen)), \
                                     amf::AMFVariant(), amf::AMFVariant(), _AccessType, 0)

    #define AMFPropertyInfoRateEx(_name, _desc, _defaultValue, _minValue, _maxValue, _AccessType) \
        new amf::AMFPropertyInfoImpl(_name, _desc, amf::AMF_VARIANT_RATE, 0, amf::AMFVariant(_defaultValue), \
                                     amf::AMFVariant(_minValue), amf::AMFVariant(_maxValue), _AccessType, 0)

    #define AMFPropertyInfoRatio(_name, _desc, defaultNum, defaultDen, _AccessType) \
        new amf::AMFPropertyInfoImpl(_name, _desc, amf::AMF_VARIANT_RATIO, 0, amf::AMFVariant(AMFConstructRatio(defaultNum, defaultDen)), \
                                     amf::AMFVariant(), amf::AMFVariant(), _AccessType, 0)

    #define AMFPropertyInfoColor(_name, _desc, defaultR, defaultG, defaultB, defaultA, _AccessType) \
        new amf::AMFPropertyInfoImpl(_name, _desc, amf::AMF_VARIANT_COLOR, 0, amf::AMFVariant(AMFConstructColor(defaultR, defaultG, defaultB, defaultA)), \
                                     amf::AMFVariant(), amf::AMFVariant(), _AccessType, 0)


    #define AMFPropertyInfoString(_name, _desc, _defaultValue, _AccessType) \
        new amf::AMFPropertyInfoImpl(_name, _desc, amf::AMF_VARIANT_STRING, 0, amf::AMFVariant(_defaultValue), \
                                     amf::AMFVariant(), amf::AMFVariant(), _AccessType, 0)

    #define AMFPropertyInfoWString(_name, _desc, _defaultValue, _AccessType) \
        new amf::AMFPropertyInfoImpl(_name, _desc, amf::AMF_VARIANT_WSTRING, 0, amf::AMFVariant(_defaultValue), \
                                     amf::AMFVariant(), amf::AMFVariant(), _AccessType, 0)

    #define AMFPropertyInfoInterface(_name, _desc, _defaultValue, _AccessType) \
        new amf::AMFPropertyInfoImpl(_name, _desc, amf::AMF_VARIANT_INTERFACE, 0, amf::AMFVariant(amf::AMFInterfacePtr(_defaultValue)), \
                                     amf::AMFVariant(amf::AMFInterfacePtr()), amf::AMFVariant(amf::AMFInterfacePtr()), _AccessType, 0)


    #define AMFPropertyInfoXML(_name, _desc, _defaultValue, _AccessType) \
        new amf::AMFPropertyInfoImpl(_name, _desc, amf::AMF_VARIANT_STRING, AMF_PROPERTY_CONTENT_XML, amf::AMFVariant(_defaultValue), \
                                     amf::AMFVariant(), amf::AMFVariant(), _AccessType, 0)

    #define AMFPropertyInfoPath(_name, _desc, _defaultValue, _AccessType) \
        new amf::AMFPropertyInfoImpl(_name, _desc, amf::AMF_VARIANT_WSTRING, AMF_PROPERTY_CONTENT_FILE_OPEN_PATH, amf::AMFVariant(_defaultValue), \
                                     amf::AMFVariant(), amf::AMFVariant(), _AccessType, 0)

    #define AMFPropertyInfoSavePath(_name, _desc, _defaultValue, _AccessType) \
        new amf::AMFPropertyInfoImpl(_name, _desc, amf::AMF_VARIANT_WSTRING, AMF_PROPERTY_CONTENT_FILE_SAVE_PATH, amf::AMFVariant(_defaultValue), \
                                     amf::AMFVariant(), amf::AMFVariant(), _AccessType, 0)

    #define AMFPropertyInfoFloatVector4D(_name, _desc, _defaultValue, _minValue, _maxValue, _AccessType) \
        new amf::AMFPropertyInfoImpl(_name, _desc, amf::AMF_VARIANT_FLOAT_VECTOR4D, 0, amf::AMFVariant(_defaultValue), \
                                     amf::AMFVariant(_minValue), amf::AMFVariant(_maxValue), _AccessType, 0)

} // namespace amf

#endif // #ifndef AMF_PropertyStorageExImpl_h
