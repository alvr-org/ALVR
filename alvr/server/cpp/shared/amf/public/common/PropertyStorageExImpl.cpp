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

#include <climits>
#include "PropertyStorageExImpl.h"
#include "PropertyStorageImpl.h"
#include "TraceAdapter.h"

#pragma warning(disable: 4996)

using namespace amf;

#define AMF_FACILITY L"AMFPropertyStorageExImpl"
#ifdef __clang__
    #pragma clang diagnostic push
    #pragma clang diagnostic ignored "-Wexit-time-destructors"
    #pragma clang diagnostic ignored "-Wglobal-constructors"
#endif

amf::AMFCriticalSection amf::ms_csAMFPropertyStorageExImplMaps;
#ifdef __clang__
    #pragma clang diagnostic pop
#endif

//-------------------------------------------------------------------------------------------------
AMF_RESULT amf::CastVariantToAMFProperty(amf::AMFVariantStruct* pDest, const amf::AMFVariantStruct* pSrc, amf::AMF_VARIANT_TYPE eType,
        amf::AMF_PROPERTY_CONTENT_TYPE /*contentType*/,
        const amf::AMFEnumDescriptionEntry* pEnumDescription)
{
    AMF_RETURN_IF_INVALID_POINTER(pDest);

    AMF_RESULT err = AMF_OK;
    switch (eType)
    {
    case AMF_VARIANT_INTERFACE:
        if (pSrc->type == eType)
        {
            err = AMFVariantCopy(pDest, pSrc);
        }
        else
        {
            pDest->type = AMF_VARIANT_INTERFACE;
            pDest->pInterface = nullptr;
        }
        break;

    case AMF_VARIANT_INT64:
    {
        if(pEnumDescription)
        {
            const AMFEnumDescriptionEntry* pEnumDescriptionCache = pEnumDescription;
            err = AMFVariantChangeType(pDest, pSrc, AMF_VARIANT_INT64);
            bool found = false;
            if(err == AMF_OK)
            {
                //mean numeric came. validating
                while(pEnumDescriptionCache->name)
                {
                    if(pEnumDescriptionCache->value == AMFVariantGetInt64(pDest))
                    {
                        AMFVariantAssignInt64(pDest, pEnumDescriptionCache->value);
                        found = true;
                        break;
                    }
                    pEnumDescriptionCache++;
                }
                err = found ? AMF_OK : AMF_INVALID_ARG;
            }
            if(!found)
            {
                pEnumDescriptionCache = pEnumDescription;
                err = AMFVariantChangeType(pDest, pSrc, AMF_VARIANT_WSTRING);
                if(err == AMF_OK)
                {
                    //string came. validating and assigning numeric
                    found = false;
                    while(pEnumDescriptionCache->name)
                    {
                        if(amf_wstring(pEnumDescriptionCache->name) == AMFVariantGetWString(pDest))
                        {
                            AMFVariantAssignInt64(pDest, pEnumDescriptionCache->value);
                            found = true;
                            break;
                        }
                        pEnumDescriptionCache++;
                    }
                    err = found ? AMF_OK : AMF_INVALID_ARG;
                }
            }
        }
        else
        {
            err = AMFVariantChangeType(pDest, pSrc, AMF_VARIANT_INT64);
        }
    }
    break;

    default:
        err = AMFVariantChangeType(pDest, pSrc, eType);
    break;
    }
    return err;
}
//-------------------------------------------------------------------------------------------------
AMFPropertyInfoImpl::AMFPropertyInfoImpl(const wchar_t* name, const wchar_t* desc, AMF_VARIANT_TYPE type, AMF_PROPERTY_CONTENT_TYPE contentType,
        AMFVariantStruct defaultValue, AMFVariantStruct minValue, AMFVariantStruct maxValue, bool allowChangeInRuntime,
        const AMFEnumDescriptionEntry* pEnumDescription) : m_name(), m_desc()
{
    AMF_PROPERTY_ACCESS_TYPE accessTypeTmp = allowChangeInRuntime ? AMF_PROPERTY_ACCESS_FULL : AMF_PROPERTY_ACCESS_READ_WRITE;
    Init(name, desc, type, contentType, defaultValue, minValue, maxValue, accessTypeTmp, pEnumDescription);
}
//-------------------------------------------------------------------------------------------------
AMFPropertyInfoImpl::AMFPropertyInfoImpl(const wchar_t* name, const wchar_t* desc, AMF_VARIANT_TYPE type, AMF_PROPERTY_CONTENT_TYPE contentType,
        AMFVariantStruct defaultValue, AMFVariantStruct minValue, AMFVariantStruct maxValue, AMF_PROPERTY_ACCESS_TYPE accessType,
        const AMFEnumDescriptionEntry* pEnumDescription) : m_name(), m_desc()
{
    Init(name, desc, type, contentType, defaultValue, minValue, maxValue, accessType, pEnumDescription);
}
//-------------------------------------------------------------------------------------------------
AMFPropertyInfoImpl::AMFPropertyInfoImpl() : m_name(), m_desc()
{
    AMFVariantInit(&this->defaultValue);
    AMFVariantInit(&this->minValue);
    AMFVariantInit(&this->maxValue);

    name = L"";
    desc = L"";
    type = AMF_VARIANT_EMPTY;
    contentType = AMF_PROPERTY_CONTENT_TYPE(-1);
    accessType = AMF_PROPERTY_ACCESS_FULL;
}
//-------------------------------------------------------------------------------------------------
void AMFPropertyInfoImpl::Init(const wchar_t* name_, const wchar_t* desc_, AMF_VARIANT_TYPE type_, AMF_PROPERTY_CONTENT_TYPE contentType_,
        AMFVariantStruct defaultValue_, AMFVariantStruct minValue_, AMFVariantStruct maxValue_, AMF_PROPERTY_ACCESS_TYPE accessType_,
        const AMFEnumDescriptionEntry* pEnumDescription_)
{
    m_name = name_;
    name = m_name.c_str();

    m_desc = desc_;
    desc = m_desc.c_str();

    type = type_;
    contentType = contentType_;
    accessType = accessType_;
    AMFVariantInit(&defaultValue);
    AMFVariantInit(&minValue);
    AMFVariantInit(&maxValue);
    pEnumDescription = pEnumDescription_;

    switch(type)
    {
    case AMF_VARIANT_BOOL:
    {
        if(CastVariantToAMFProperty(&defaultValue, &defaultValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignBool(&defaultValue, false);
        }
    }
    break;
    case AMF_VARIANT_RECT:
    {
        if(CastVariantToAMFProperty(&defaultValue, &defaultValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignRect(&defaultValue, AMFConstructRect(0, 0, 0, 0));
        }
    }
    break;
    case AMF_VARIANT_SIZE:
    {
        if(CastVariantToAMFProperty(&defaultValue, &defaultValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignSize(&defaultValue, AMFConstructSize(0, 0));
        }
        if (CastVariantToAMFProperty(&minValue, &minValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignSize(&minValue, AMFConstructSize(INT_MIN, INT_MIN));
        }
        if (CastVariantToAMFProperty(&maxValue, &maxValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignSize(&maxValue, AMFConstructSize(INT_MAX, INT_MAX));
        }
    }
    break;
    case AMF_VARIANT_POINT:
    {
        if(CastVariantToAMFProperty(&defaultValue, &defaultValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignPoint(&defaultValue, AMFConstructPoint(0, 0));
        }
        if (CastVariantToAMFProperty(&minValue, &minValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignPoint(&minValue, AMFConstructPoint(INT_MIN, INT_MIN));
        }
        if (CastVariantToAMFProperty(&maxValue, &maxValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignPoint(&maxValue, AMFConstructPoint(INT_MAX, INT_MAX));
        }
    }
    break;
    case AMF_VARIANT_RATE:
    {
        if(CastVariantToAMFProperty(&defaultValue, &defaultValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignRate(&defaultValue, AMFConstructRate(0, 0));
        }
        if (CastVariantToAMFProperty(&this->minValue, &minValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignRate(&this->minValue, AMFConstructRate(0, 1));
        }
        if (CastVariantToAMFProperty(&this->maxValue, &maxValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignRate(&this->maxValue, AMFConstructRate(INT_MAX, INT_MAX));
        }
    }
    break;
    case AMF_VARIANT_RATIO:
    {
        if(CastVariantToAMFProperty(&defaultValue, &defaultValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignRatio(&defaultValue, AMFConstructRatio(0, 0));
        }
    }
    break;
    case AMF_VARIANT_COLOR:
    {
        if(CastVariantToAMFProperty(&defaultValue, &defaultValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignColor(&defaultValue, AMFConstructColor(0, 0, 0, 255));
        }
    }
    break;

    case AMF_VARIANT_INT64:
    {
        if(pEnumDescription)
        {
            if(CastVariantToAMFProperty(&defaultValue, &defaultValue_, type, contentType, pEnumDescription) != AMF_OK)
            {
                AMFVariantAssignInt64(&defaultValue, pEnumDescription->value);
            }
        }
        else //AMF_PROPERTY_CONTENT_DEFAULT
        {
            if(CastVariantToAMFProperty(&defaultValue, &defaultValue_, type, contentType, pEnumDescription) != AMF_OK)
            {
                AMFVariantAssignInt64(&defaultValue, 0);
            }
            if(CastVariantToAMFProperty(&minValue, &minValue_, type, contentType, pEnumDescription) != AMF_OK)
            {
                AMFVariantAssignInt64(&minValue, INT_MIN);
            }
            if(CastVariantToAMFProperty(&maxValue, &maxValue_, type, contentType, pEnumDescription) != AMF_OK)
            {
                AMFVariantAssignInt64(&maxValue, INT_MAX);
            }
        }
    }
    break;

    case AMF_VARIANT_DOUBLE:
    {
        if(CastVariantToAMFProperty(&defaultValue, &defaultValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignDouble(&defaultValue, 0);
        }
        if(CastVariantToAMFProperty(&minValue, &minValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignDouble(&minValue, DBL_MIN);
        }
        if(CastVariantToAMFProperty(&maxValue, &maxValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignDouble(&maxValue, DBL_MAX);
        }
    }
    break;

    case AMF_VARIANT_STRING:
    {
        if(CastVariantToAMFProperty(&defaultValue, &defaultValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignString(&maxValue, "");
        }
    }
    break;

    case AMF_VARIANT_WSTRING:
    {
        if(CastVariantToAMFProperty(&defaultValue, &defaultValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignWString(&maxValue, L"");
        }
    }
    break;

    case AMF_VARIANT_INTERFACE:
        if(CastVariantToAMFProperty(&defaultValue, &defaultValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignWString(&maxValue, L"");
        }
        break;
    case AMF_VARIANT_FLOAT:
    {
        if (CastVariantToAMFProperty(&defaultValue, &defaultValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignFloat(&defaultValue, 0);
        }
        if (CastVariantToAMFProperty(&minValue, &minValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignFloat(&minValue, FLT_MIN);
        }
        if (CastVariantToAMFProperty(&maxValue, &maxValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignFloat(&maxValue, FLT_MAX);
        }
    }
    break;
    case AMF_VARIANT_FLOAT_SIZE:
    {
        if (CastVariantToAMFProperty(&defaultValue, &defaultValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignFloatSize(&defaultValue, AMFConstructFloatSize(0, 0));
        }
        if (CastVariantToAMFProperty(&minValue, &minValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignFloatSize(&minValue, AMFConstructFloatSize(FLT_MIN, FLT_MIN));
        }
        if (CastVariantToAMFProperty(&maxValue, &maxValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignFloatSize(&maxValue, AMFConstructFloatSize(FLT_MAX, FLT_MAX));
        }
    }
    break;
    case AMF_VARIANT_FLOAT_POINT2D:
    {
        if (CastVariantToAMFProperty(&defaultValue, &defaultValue, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignFloatPoint2D(&defaultValue, AMFConstructFloatPoint2D(0, 0));
        }
        if (CastVariantToAMFProperty(&minValue, &minValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignFloatPoint2D(&minValue, AMFConstructFloatPoint2D(FLT_MIN, FLT_MIN));
        }
        if (CastVariantToAMFProperty(&maxValue, &maxValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignFloatPoint2D(&maxValue, AMFConstructFloatPoint2D(FLT_MAX, FLT_MAX));
        }
    }
    break;
    case AMF_VARIANT_FLOAT_POINT3D:
    {
        if (CastVariantToAMFProperty(&defaultValue, &defaultValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignFloatPoint3D(&defaultValue, AMFConstructFloatPoint3D(0, 0, 0));
        }
        if (CastVariantToAMFProperty(&minValue, &minValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignFloatPoint3D(&minValue, AMFConstructFloatPoint3D(FLT_MIN, FLT_MIN, FLT_MIN));
        }
        if (CastVariantToAMFProperty(&maxValue, &maxValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignFloatPoint3D(&maxValue, AMFConstructFloatPoint3D(FLT_MAX, FLT_MAX, FLT_MAX));
        }
    }
    break;
    case AMF_VARIANT_FLOAT_VECTOR4D:
    {
        if (CastVariantToAMFProperty(&defaultValue, &defaultValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignFloatVector4D(&defaultValue, AMFConstructFloatVector4D(0, 0, 0, 0));
        }
        if (CastVariantToAMFProperty(&minValue, &minValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignFloatVector4D(&minValue, AMFConstructFloatVector4D(FLT_MIN, FLT_MIN, FLT_MIN, FLT_MIN));
        }
        if (CastVariantToAMFProperty(&maxValue, &maxValue_, type, contentType, pEnumDescription) != AMF_OK)
        {
            AMFVariantAssignFloatVector4D(&maxValue, AMFConstructFloatVector4D(FLT_MAX, FLT_MAX, FLT_MAX, FLT_MAX));
        }
    }
    break;
    default:
        break;
    }

    value = defaultValue;
}

AMFPropertyInfoImpl::AMFPropertyInfoImpl(const AMFPropertyInfoImpl& propertyInfo) : AMFPropertyInfo(), m_name(), m_desc()
{
    Init(propertyInfo.name, propertyInfo.desc, propertyInfo.type, propertyInfo.contentType, propertyInfo.defaultValue, propertyInfo.minValue, propertyInfo.maxValue, propertyInfo.accessType, propertyInfo.pEnumDescription);
}
//-------------------------------------------------------------------------------------------------
AMFPropertyInfoImpl& AMFPropertyInfoImpl::operator=(const AMFPropertyInfoImpl& propertyInfo)
{
    // store name and desc inside instance in m_sName and m_sDesc recpectively;
    // m_pName and m_pDesc are pointed to our local copies
    this->m_name = propertyInfo.name;
    this->m_desc = propertyInfo.desc;
    this->name = m_name.c_str();
    this->desc = m_desc.c_str();

    this->type = propertyInfo.type;
    this->contentType = propertyInfo.contentType;
    this->accessType = propertyInfo.accessType;
    AMFVariantCopy(&this->defaultValue, &propertyInfo.defaultValue);
    AMFVariantCopy(&this->minValue, &propertyInfo.minValue);
    AMFVariantCopy(&this->maxValue, &propertyInfo.maxValue);
    this->pEnumDescription = propertyInfo.pEnumDescription;

    this->value = propertyInfo.value;
    this->userModified = propertyInfo.userModified;

    return *this;
}
//-------------------------------------------------------------------------------------------------
AMFPropertyInfoImpl::~AMFPropertyInfoImpl()
{
    AMFVariantClear(&this->defaultValue);
    AMFVariantClear(&this->minValue);
    AMFVariantClear(&this->maxValue);
}
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
