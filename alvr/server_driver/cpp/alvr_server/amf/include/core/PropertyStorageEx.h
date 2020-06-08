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

#ifndef __AMFPropertyStorageEx_h__
#define __AMFPropertyStorageEx_h__
#pragma once

#include "PropertyStorage.h"

namespace amf
{
    //----------------------------------------------------------------------------------------------
    enum AMF_PROPERTY_CONTENT_ENUM
    {
        AMF_PROPERTY_CONTENT_DEFAULT = 0,
        AMF_PROPERTY_CONTENT_XML,               // m_eType is AMF_VARIANT_STRING

        AMF_PROPERTY_CONTENT_FILE_OPEN_PATH,    // m_eType AMF_VARIANT_WSTRING
        AMF_PROPERTY_CONTENT_FILE_SAVE_PATH     // m_eType AMF_VARIANT_WSTRING
    };
    //----------------------------------------------------------------------------------------------
    enum AMF_PROPERTY_ACCESS_TYPE
    {
        AMF_PROPERTY_ACCESS_PRIVATE             = 0,
        AMF_PROPERTY_ACCESS_READ                = 0x1,
        AMF_PROPERTY_ACCESS_WRITE               = 0x2,
        AMF_PROPERTY_ACCESS_READ_WRITE          = (AMF_PROPERTY_ACCESS_READ | AMF_PROPERTY_ACCESS_WRITE),
        AMF_PROPERTY_ACCESS_WRITE_RUNTIME       = 0x4,
        AMF_PROPERTY_ACCESS_FULL                = 0xFF,
    };
    //----------------------------------------------------------------------------------------------
    struct AMFEnumDescriptionEntry
    {
        amf_int             value;
        const wchar_t*      name;
    };
    //----------------------------------------------------------------------------------------------
    typedef amf_uint32 AMF_PROPERTY_CONTENT_TYPE;

    struct AMFPropertyInfo
    {
        const wchar_t*                  name;
        const wchar_t*                  desc;
        AMF_VARIANT_TYPE                type;
        AMF_PROPERTY_CONTENT_TYPE       contentType;

        AMFVariantStruct                defaultValue;
        AMFVariantStruct                minValue;
        AMFVariantStruct                maxValue;
        AMF_PROPERTY_ACCESS_TYPE        accessType;
        const AMFEnumDescriptionEntry*  pEnumDescription;

        AMFPropertyInfo() :
            name(NULL),
            desc(NULL),
            type(),
            contentType(),
            defaultValue(),
            minValue(),
            maxValue(),
            accessType(AMF_PROPERTY_ACCESS_FULL),
            pEnumDescription(NULL)
        {}
        AMFPropertyInfo(const AMFPropertyInfo& propery) : name(propery.name),
            desc(propery.desc),
            type(propery.type),
            contentType(propery.contentType),
            defaultValue(propery.defaultValue),
            minValue(propery.minValue),
            maxValue(propery.maxValue),
            accessType(propery.accessType),
            pEnumDescription(propery.pEnumDescription)
        {}
        virtual ~AMFPropertyInfo(){}

        bool AMF_STD_CALL AllowedRead() const
        {
            return (accessType & AMF_PROPERTY_ACCESS_READ) != 0;
        }
        bool AMF_STD_CALL AllowedWrite() const
        {
            return (accessType & AMF_PROPERTY_ACCESS_WRITE) != 0;
        }
        bool AMF_STD_CALL AllowedChangeInRuntime() const
        {
            return (accessType & AMF_PROPERTY_ACCESS_WRITE_RUNTIME) != 0;
        }

        AMFPropertyInfo& operator=(const AMFPropertyInfo& propery)
        {
            desc = propery.desc;
            type = propery.type;
            contentType = propery.contentType;
            defaultValue = propery.defaultValue;
            minValue = propery.minValue;
            maxValue = propery.maxValue;
            accessType = propery.accessType;
            pEnumDescription = propery.pEnumDescription;

            return *this;
        }
    };
    //----------------------------------------------------------------------------------------------
    // AMFPropertyStorageEx interface
    //----------------------------------------------------------------------------------------------
    class AMF_NO_VTABLE AMFPropertyStorageEx : public AMFPropertyStorage
    {
    public:
        AMF_DECLARE_IID(0x16b8958d, 0xe943, 0x4a33, 0xa3, 0x5a, 0x88, 0x5a, 0xd8, 0x28, 0xf2, 0x67)

        virtual amf_size            AMF_STD_CALL GetPropertiesInfoCount() const = 0;
        virtual AMF_RESULT          AMF_STD_CALL GetPropertyInfo(amf_size index, const AMFPropertyInfo** ppInfo) const = 0;
        virtual AMF_RESULT          AMF_STD_CALL GetPropertyInfo(const wchar_t* name, const AMFPropertyInfo** ppInfo) const = 0;
        virtual AMF_RESULT          AMF_STD_CALL ValidateProperty(const wchar_t* name, AMFVariantStruct value, AMFVariantStruct* pOutValidated) const = 0;
    };
    //----------------------------------------------------------------------------------------------
    // smart pointer
    //----------------------------------------------------------------------------------------------
    typedef AMFInterfacePtr_T<AMFPropertyStorageEx> AMFPropertyStorageExPtr;
    //----------------------------------------------------------------------------------------------
} //namespace amf


#endif //#ifndef __AMFPropertyStorageEx_h__
