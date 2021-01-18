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

#ifndef __AMFVariant_h__
#define __AMFVariant_h__
#pragma once
#pragma warning(disable: 4996)

#include "Interface.h"
#include <locale.h>

namespace amf
{
    //----------------------------------------------------------------------------------------------
    // variant types
    //----------------------------------------------------------------------------------------------
    enum AMF_VARIANT_TYPE
    {
        AMF_VARIANT_EMPTY       = 0,

        AMF_VARIANT_BOOL        = 1,
        AMF_VARIANT_INT64       = 2,
        AMF_VARIANT_DOUBLE      = 3,

        AMF_VARIANT_RECT        = 4,
        AMF_VARIANT_SIZE        = 5,
        AMF_VARIANT_POINT       = 6,
        AMF_VARIANT_RATE        = 7,
        AMF_VARIANT_RATIO       = 8,
        AMF_VARIANT_COLOR       = 9,

        AMF_VARIANT_STRING      = 10,  // value is char*
        AMF_VARIANT_WSTRING     = 11,  // value is wchar*
        AMF_VARIANT_INTERFACE   = 12,  // value is AMFInterface*
    };
    //----------------------------------------------------------------------------------------------
    // variant struct
   //----------------------------------------------------------------------------------------------
    struct AMFVariantStruct
    {
        AMF_VARIANT_TYPE    type;
        union
        {
            amf_bool        boolValue;
            amf_int64       int64Value;
            amf_double      doubleValue;
            char*           stringValue;
            wchar_t*        wstringValue;
            AMFInterface*   pInterface;
            AMFRect         rectValue;
            AMFSize         sizeValue;
            AMFPoint        pointValue;
            AMFRate         rateValue;
            AMFRatio        ratioValue;
            AMFColor        colorValue;
        };
    };
    //----------------------------------------------------------------------------------------------
    // variant accessors
    //----------------------------------------------------------------------------------------------

    inline AMF_VARIANT_TYPE     AMF_STD_CALL AMFVariantGetType(const AMFVariantStruct* _variant) { return (_variant)->type; }
    inline AMF_VARIANT_TYPE&    AMF_STD_CALL AMFVariantGetType(AMFVariantStruct* _variant) { return (_variant)->type; }
    inline amf_bool             AMF_STD_CALL AMFVariantGetBool(const AMFVariantStruct* _variant) { return (_variant)->boolValue; }
    inline amf_int64            AMF_STD_CALL AMFVariantGetInt64(const AMFVariantStruct* _variant) { return (_variant)->int64Value; }
    inline amf_double           AMF_STD_CALL AMFVariantGetDouble(const AMFVariantStruct* _variant) { return (_variant)->doubleValue; }
    inline const char*          AMF_STD_CALL AMFVariantGetString(const AMFVariantStruct* _variant) { return (_variant)->stringValue; }
    inline const wchar_t*       AMF_STD_CALL AMFVariantGetWString(const AMFVariantStruct* _variant) { return (_variant)->wstringValue; }
    inline const AMFInterface*  AMF_STD_CALL AMFVariantGetInterface(const AMFVariantStruct* _variant) { return (_variant)->pInterface; }
    inline AMFInterface*        AMF_STD_CALL AMFVariantGetInterface(AMFVariantStruct* _variant) { return (_variant)->pInterface; }

    inline const AMFRect &       AMF_STD_CALL AMFVariantGetRect (const AMFVariantStruct* _variant) { return (_variant)->rectValue; }
    inline const AMFSize &       AMF_STD_CALL AMFVariantGetSize (const AMFVariantStruct* _variant) { return (_variant)->sizeValue; }
    inline const AMFPoint&       AMF_STD_CALL AMFVariantGetPoint(const AMFVariantStruct* _variant) { return (_variant)->pointValue; }
    inline const AMFRate &       AMF_STD_CALL AMFVariantGetRate (const AMFVariantStruct* _variant) { return (_variant)->rateValue; }
    inline const AMFRatio&       AMF_STD_CALL AMFVariantGetRatio(const AMFVariantStruct* _variant) { return (_variant)->ratioValue; }
    inline const AMFColor&       AMF_STD_CALL AMFVariantGetColor(const AMFVariantStruct* _variant) { return (_variant)->colorValue; }

    #define AMFVariantEmpty(_variant)     0
    #define AMFVariantBool(_variant)      (_variant)->boolValue
    #define AMFVariantInt64(_variant)     (_variant)->int64Value
    #define AMFVariantDouble(_variant)    (_variant)->doubleValue

    #define AMFVariantRect(_variant)      (_variant)->rectValue
    #define AMFVariantSize(_variant)      (_variant)->sizeValue
    #define AMFVariantPoint(_variant)     (_variant)->pointValue
    #define AMFVariantRate(_variant)      (_variant)->rateValue
    #define AMFVariantRatio(_variant)     (_variant)->ratioValue
    #define AMFVariantColor(_variant)     (_variant)->colorValue

    #define AMFVariantString(_variant)    (_variant)->stringValue
    #define AMFVariantWString(_variant)   (_variant)->wstringValue
    #define AMFVariantInterface(_variant) (_variant)->pInterface
    //----------------------------------------------------------------------------------------------
    // variant hleper functions
    //----------------------------------------------------------------------------------------------
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantInit(AMFVariantStruct* pVariant);
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantClear(AMFVariantStruct* pVariant);
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantCompare(const AMFVariantStruct* pFirst, const AMFVariantStruct* pSecond, bool& equal);
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantCopy(AMFVariantStruct* pDest, const AMFVariantStruct* pSrc);
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantChangeType(AMFVariantStruct* pDest, const AMFVariantStruct* pSrc, AMF_VARIANT_TYPE newType);
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantAssignBool(AMFVariantStruct* pDest, bool value);
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantAssignInt64(AMFVariantStruct* pDest, amf_int64 value);
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantAssignDouble(AMFVariantStruct* pDest, amf_double value);
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantAssignString(AMFVariantStruct* pDest, const char* value);
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantAssignWString(AMFVariantStruct* pDest, const wchar_t* value);
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantAssignInterface(AMFVariantStruct* pDest, AMFInterface* value);
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantAssignRect(AMFVariantStruct* pDest, const AMFRect& value);
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantAssignSize(AMFVariantStruct* pDest, const AMFSize& value);
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantAssignPoint(AMFVariantStruct* pDest, const AMFPoint& value);
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantAssignRate(AMFVariantStruct* pDest, const AMFRate& value);
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantAssignRatio(AMFVariantStruct* pDest, const AMFRatio& value);
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantAssignColor(AMFVariantStruct* pDest, const AMFColor& value);
    inline char*      AMF_CDECL_CALL AMFVariantDuplicateString(const char* from);
    inline void       AMF_CDECL_CALL AMFVariantFreeString(char* from);
    inline wchar_t*   AMF_CDECL_CALL AMFVariantDuplicateWString(const wchar_t* from);
    inline void       AMF_CDECL_CALL AMFVariantFreeWString(wchar_t* from);

    //----------------------------------------------------------------------------------------------
    // inline Variant helper class
    //----------------------------------------------------------------------------------------------
    class AMFVariant : public AMFVariantStruct
    {
    public:
        class String;
        class WString;

    public:
        AMFVariant() {  AMFVariantInit(this); }
        explicit AMFVariant(const AMFVariantStruct& other) { AMFVariantInit(this); AMFVariantCopy(this, const_cast<AMFVariantStruct*>(&other)); }

        explicit AMFVariant(const AMFVariantStruct* pOther);
        template<typename T>
        explicit AMFVariant(const AMFInterfacePtr_T<T>& pValue);

        AMFVariant(const AMFVariant& other) { AMFVariantInit(this); AMFVariantCopy(this, const_cast<AMFVariantStruct*>(static_cast<const AMFVariantStruct*>(&other))); }

        explicit inline AMFVariant(amf_bool value)          { AMFVariantInit(this); AMFVariantAssignBool(this, value); }
        explicit inline AMFVariant(amf_int64 value)         { AMFVariantInit(this); AMFVariantAssignInt64(this, value); }
        explicit inline AMFVariant(amf_uint64 value)        { AMFVariantInit(this); AMFVariantAssignInt64(this, (amf_int64)value); }
        explicit inline AMFVariant(amf_int32 value)         { AMFVariantInit(this); AMFVariantAssignInt64(this, value); }
        explicit inline AMFVariant(amf_uint32 value)        { AMFVariantInit(this); AMFVariantAssignInt64(this, value); }
        explicit inline AMFVariant(amf_double value)        { AMFVariantInit(this); AMFVariantAssignDouble(this, value); }
        explicit inline AMFVariant(const AMFRect & value)   { AMFVariantInit(this); AMFVariantAssignRect(this, value); }
        explicit inline AMFVariant(const AMFSize & value)   { AMFVariantInit(this); AMFVariantAssignSize(this, value); }
        explicit inline AMFVariant(const AMFPoint& value)   { AMFVariantInit(this); AMFVariantAssignPoint(this, value); }
        explicit inline AMFVariant(const AMFRate & value)   { AMFVariantInit(this); AMFVariantAssignRate(this, value); }
        explicit inline AMFVariant(const AMFRatio& value)   { AMFVariantInit(this); AMFVariantAssignRatio(this, value); }
        explicit inline AMFVariant(const AMFColor& value)   { AMFVariantInit(this); AMFVariantAssignColor(this, value); }
        explicit inline AMFVariant(const char* value)       { AMFVariantInit(this); AMFVariantAssignString(this, value); }
        explicit inline AMFVariant(const wchar_t* value)    { AMFVariantInit(this); AMFVariantAssignWString(this, value); }
        explicit inline AMFVariant(AMFInterface* pValue)    { AMFVariantInit(this); AMFVariantAssignInterface(this, pValue); }

        ~AMFVariant() { AMFVariantClear(this); }

        AMFVariant& operator=(const AMFVariantStruct& other);
        AMFVariant& operator=(const AMFVariantStruct* pOther);
        AMFVariant& operator=(const AMFVariant& other);

        AMFVariant& operator=(amf_bool          value)      { AMFVariantAssignBool(this, value); return *this;}
        AMFVariant& operator=(amf_int64         value)      { AMFVariantAssignInt64(this, value); return *this;}
        AMFVariant& operator=(amf_uint64        value)      { AMFVariantAssignInt64(this, (amf_int64)value);  return *this;}
        AMFVariant& operator=(amf_int32         value)      { AMFVariantAssignInt64(this, value);  return *this;}
        AMFVariant& operator=(amf_uint32        value)      { AMFVariantAssignInt64(this, value);  return *this;}
        AMFVariant& operator=(amf_double        value)      { AMFVariantAssignDouble(this, value);  return *this;}
        AMFVariant& operator=(const AMFRect &   value)      { AMFVariantAssignRect(this, value);  return *this;}
        AMFVariant& operator=(const AMFSize &   value)      { AMFVariantAssignSize(this, value);  return *this;}
        AMFVariant& operator=(const AMFPoint&   value)      { AMFVariantAssignPoint(this, value);  return *this;}
        AMFVariant& operator=(const AMFRate &   value)      { AMFVariantAssignRate(this, value);  return *this;}
        AMFVariant& operator=(const AMFRatio&   value)      { AMFVariantAssignRatio(this, value);  return *this;}
        AMFVariant& operator=(const AMFColor&   value)      { AMFVariantAssignColor(this, value);  return *this;}
        AMFVariant& operator=(const char*       value)      { AMFVariantAssignString(this, value);  return *this;}
        AMFVariant& operator=(const wchar_t*    value)      { AMFVariantAssignWString(this, value);  return *this;}
        AMFVariant& operator=(AMFInterface*     value)      { AMFVariantAssignInterface(this, value);  return *this;}

        template<typename T> AMFVariant& operator=(const AMFInterfacePtr_T<T>& value);

        operator amf_bool() const          { return ToBool();       }
        operator amf_int64() const         { return ToInt64();      }
        operator amf_uint64() const        { return ToUInt64();     }
        operator amf_int32() const         { return ToInt32();      }
        operator amf_uint32() const        { return ToUInt32();     }
        operator amf_double() const        { return ToDouble();     }
        operator amf_float() const         { return ToFloat();      }
        operator AMFRect () const          { return ToRect ();      }
        operator AMFSize () const          { return ToSize ();      }
        operator AMFPoint() const          { return ToPoint();      }
        operator AMFRate () const          { return ToRate ();      }
        operator AMFRatio() const          { return ToRatio();      }
        operator AMFColor() const          { return ToColor();      }
        operator AMFInterface*() const     { return ToInterface();  }

        inline amf_bool         ToBool() const      { return Empty() ? false        : GetValue<amf_bool,   AMF_VARIANT_BOOL>(AMFVariantGetBool); }
        inline amf_int64        ToInt64() const     { return Empty() ? 0            : GetValue<amf_int64,  AMF_VARIANT_INT64>(AMFVariantGetInt64); }
        inline amf_uint64       ToUInt64() const    { return Empty() ? 0            : GetValue<amf_uint64, AMF_VARIANT_INT64>(AMFVariantGetInt64); }
        inline amf_int32        ToInt32() const     { return Empty() ? 0            : GetValue<amf_int32,  AMF_VARIANT_INT64>(AMFVariantGetInt64); }
        inline amf_uint32       ToUInt32() const    { return Empty() ? 0            : GetValue<amf_uint32, AMF_VARIANT_INT64>(AMFVariantGetInt64); }
        inline amf_double       ToDouble() const    { return Empty() ? 0            : GetValue<amf_double, AMF_VARIANT_DOUBLE>(AMFVariantGetDouble); }
        inline amf_float        ToFloat() const     { return Empty() ? 0            : GetValue<amf_float,  AMF_VARIANT_DOUBLE>(AMFVariantGetDouble); }
        inline AMFRect          ToRect () const     { return Empty() ? AMFRect()    : GetValue<AMFRect,  AMF_VARIANT_RECT>(AMFVariantGetRect); }
        inline AMFSize          ToSize () const     { return Empty() ? AMFSize()    : GetValue<AMFSize,  AMF_VARIANT_SIZE>(AMFVariantGetSize); }
        inline AMFPoint         ToPoint() const     { return Empty() ? AMFPoint()   : GetValue<AMFPoint, AMF_VARIANT_POINT>(AMFVariantGetPoint); }
        inline AMFRate          ToRate () const     { return Empty() ? AMFRate()    : GetValue<AMFRate,  AMF_VARIANT_RATE>(AMFVariantGetRate); }
        inline AMFRatio         ToRatio() const     { return Empty() ? AMFRatio()   : GetValue<AMFRatio, AMF_VARIANT_RATIO>(AMFVariantGetRatio); }
        inline AMFColor         ToColor() const     { return Empty() ? AMFColor()   : GetValue<AMFColor, AMF_VARIANT_COLOR>(AMFVariantGetColor); }
        inline AMFInterface*    ToInterface() const { return AMFVariantGetType(this) == AMF_VARIANT_INTERFACE ? this->pInterface : NULL; }
        inline String           ToString() const;
        inline WString          ToWString() const;

        bool operator==(const AMFVariantStruct& other) const;
        bool operator==(const AMFVariantStruct* pOther) const;

        bool operator!=(const AMFVariantStruct& other) const;
        bool operator!=(const AMFVariantStruct* pOther) const;

        void Clear() { AMFVariantClear(this); }

        void Attach(AMFVariantStruct& variant);
        AMFVariantStruct Detach();

        AMFVariantStruct& GetVariant();

        void ChangeType(AMF_VARIANT_TYPE type, const AMFVariant* pSrc = NULL);

        bool Empty() const;
    private:
        template<class ReturnType, AMF_VARIANT_TYPE variantType, typename Getter>
        ReturnType GetValue(Getter getter) const;
    };
    //----------------------------------------------------------------------------------------------
    // helper String class
    //----------------------------------------------------------------------------------------------
    class AMFVariant::String
    {
        friend class AMFVariant;
    private:
        void Free()
        {
            if (m_Str != NULL)
            {
                AMFVariantFreeString(m_Str);
                m_Str = NULL;
            }
        }
    public:
        String() :m_Str(NULL){}
        String(const char* str) : m_Str(NULL)
        {
            m_Str = AMFVariantDuplicateString(str);
        }
        String(const String& p_other) : m_Str(NULL)
        {
            operator=(p_other);
        }

#if (__cplusplus == 201103L) || defined(__GXX_EXPERIMENTAL_CXX0X) || (_MSC_VER >= 1600)
        String(String&& p_other) : m_Str(NULL)
        {
            operator=(p_other);
        }
#endif
        ~String()
        {
            Free();
        }

        String& operator=(const String& p_other)
        {
            Free();
            m_Str = AMFVariantDuplicateString(p_other.m_Str);
            return *this;
        }
#if (__cplusplus == 201103L) || defined(__GXX_EXPERIMENTAL_CXX0X) || (_MSC_VER >= 1600)
        String& operator=(String&& p_other)
        {
            Free();
            m_Str = p_other.m_Str;
            p_other.m_Str = NULL;    //    Transfer the ownership
            return *this;
        }
#endif
        bool operator==(const String& p_other)
        {
            if(c_str() == 0)
            {
                return true;
            }
            if(p_other.c_str() == 0)
            {
                return false;
            }
            return strcmp(c_str(), p_other.c_str()) == 0;
        }
        const char* c_str() const { return m_Str; }
        size_t size() const
        {
            if(m_Str == NULL)
            {
                return 0;
            }
            return (size_t)strlen(m_Str);
        }
        void resize(size_t sizeAlloc)
        {
            if(sizeAlloc == 0)
            {
                Free();
                return;
            }
            char* str = (char*)amf_variant_alloc(sizeof(char)*(sizeAlloc + 1));
            if(m_Str != NULL)
            {
                size_t copySize = sizeAlloc;
                if(copySize > size())
                {
                    copySize = size();
                }
                memcpy(str, m_Str, copySize * sizeof(char));
                Free();
                str[sizeAlloc] = 0;
            }
            m_Str = str;
        }
    private:
        char*    m_Str;
    };
    //----------------------------------------------------------------------------------------------
    // helper WString class
    //----------------------------------------------------------------------------------------------
    class AMFVariant::WString
    {
        friend class AMFVariant;
    private:
        void Free()
        {
            if (m_Str != NULL)
            {
                AMFVariantFreeWString(m_Str);
                m_Str = NULL;
            }
        }
    public:
        WString() :m_Str(NULL){}
        WString(const wchar_t* str) : m_Str(NULL)
        {
            m_Str = AMFVariantDuplicateWString(str);
        }
        WString(const WString& p_other) : m_Str(NULL)
        {
            operator=(p_other);
        }
#if (__cplusplus == 201103L) || defined(__GXX_EXPERIMENTAL_CXX0X) || (_MSC_VER >= 1600)
        WString(WString&& p_other) : m_Str(NULL)
        {
            operator=(p_other);
        }
#endif
        ~WString()
        {
            Free();
        }

        WString& operator=(const WString& p_other)
        {
            Free();
            m_Str = AMFVariantDuplicateWString(p_other.m_Str);
            return *this;
        }
#if (__cplusplus == 201103L) || defined(__GXX_EXPERIMENTAL_CXX0X) || (_MSC_VER >= 1600)
        WString& operator=(WString&& p_other)
        {
            Free();
            m_Str = p_other.m_Str;
            p_other.m_Str = NULL;    //    Transfer the ownership
            return *this;
        }
#endif
        bool operator==(const WString& p_other)
        {
            if(c_str() == 0)
            {
                return true;
            }
            if(p_other.c_str() == 0)
            {
                return false;
            }
            return wcscmp(c_str(), p_other.c_str()) == 0;
        }

        const wchar_t* c_str() const { return m_Str; }
        size_t size()  const
        {
            if(m_Str == NULL)
            {
                return 0;
            }
            return (size_t)wcslen(m_Str);
        }
        void resize(size_t sizeAlloc)
        {
            if(sizeAlloc == 0)
            {
                Free();
                return;
            }
            wchar_t* str = (wchar_t*)amf_variant_alloc(sizeof(wchar_t)*(sizeAlloc + 1));
            if(m_Str != NULL)
            {
                size_t copySize = sizeAlloc;
                if(copySize > size())
                {
                    copySize = size();
                }
                memcpy(str, m_Str, copySize * sizeof(wchar_t));
                Free();
                str[sizeAlloc] = 0;
            }
            m_Str = str;
        }
    private:
        wchar_t*    m_Str;
    };
    //-------------------------------------------------------------------------------------------------
    AMFVariant::String       AMFVariant::ToString() const
    {
        String temp = GetValue<String, AMF_VARIANT_STRING>(AMFVariantGetString); 
        return String(temp.c_str());
    }
    //-------------------------------------------------------------------------------------------------
    AMFVariant::WString      AMFVariant::ToWString() const
    {
        WString temp = GetValue<WString, AMF_VARIANT_WSTRING>(AMFVariantGetWString);
        return WString(temp.c_str());
    }

    //----------------------------------------------------------------------------------------------
    // inline implementation of helper functions 
    //----------------------------------------------------------------------------------------------
    #define AMF_VARIANT_RETURN_IF_INVALID_POINTER(p) \
       { \
            if(p == NULL) \
                    { \
                 return AMF_INVALID_POINTER; \
            } \
       }
    //-------------------------------------------------------------------------------------------------
    AMF_RESULT AMF_CDECL_CALL AMFVariantInit(AMFVariantStruct* pVariant)
    {
        AMF_VARIANT_RETURN_IF_INVALID_POINTER(pVariant);
        AMFVariantGetType(pVariant) = AMF_VARIANT_EMPTY;
        return AMF_OK;
    }
    //-------------------------------------------------------------------------------------------------
    AMF_RESULT AMF_CDECL_CALL AMFVariantClear(AMFVariantStruct* pVariant)
    {
        AMF_VARIANT_RETURN_IF_INVALID_POINTER(pVariant);
        AMF_RESULT errRet = AMF_OK;

        switch(AMFVariantGetType(pVariant))
        {
        case AMF_VARIANT_STRING:
            amf_variant_free(AMFVariantString(pVariant));
            AMFVariantGetType(pVariant) = AMF_VARIANT_EMPTY;
            break;

        case AMF_VARIANT_WSTRING:
            amf_variant_free(AMFVariantWString(pVariant));
            AMFVariantGetType(pVariant) = AMF_VARIANT_EMPTY;
            break;

        case AMF_VARIANT_INTERFACE:
            if(AMFVariantInterface(pVariant) != NULL)
            {
                AMFVariantInterface(pVariant)->Release();
                AMFVariantInterface(pVariant) = NULL;
            }
            AMFVariantGetType(pVariant) = AMF_VARIANT_EMPTY;
            break;

        default:
            AMFVariantGetType(pVariant) = AMF_VARIANT_EMPTY;
            break;
        }
        return errRet;
    }
    //-------------------------------------------------------------------------------------------------
    AMF_RESULT AMF_CDECL_CALL AMFVariantCompare(const AMFVariantStruct* pFirst, const AMFVariantStruct* pSecond, bool& bEqual)
    {
        AMF_VARIANT_RETURN_IF_INVALID_POINTER(pFirst);
        AMF_VARIANT_RETURN_IF_INVALID_POINTER(pSecond);
        AMF_RESULT errRet = AMF_OK;
    
        if(pFirst == pSecond)
        {
            bEqual = true;
        }
        else if(AMFVariantGetType(pFirst) != AMFVariantGetType(pSecond))
        {
            bEqual = false;
        }
        else
        {
            switch(AMFVariantGetType(pFirst))
            {
            case AMF_VARIANT_EMPTY:
                bEqual = true;
                break;
            case AMF_VARIANT_BOOL:
                bEqual = AMFVariantGetBool(pFirst) == AMFVariantBool(pSecond);
                break;
            case AMF_VARIANT_INT64:
                bEqual = AMFVariantGetInt64(pFirst) == AMFVariantInt64(pSecond);
                break;
            case AMF_VARIANT_DOUBLE:
                bEqual = AMFVariantGetDouble(pFirst) == AMFVariantDouble(pSecond);
                break;
            case AMF_VARIANT_RECT:
                bEqual = AMFVariantGetRect(pFirst) == AMFVariantGetRect(pSecond);
                break;
            case AMF_VARIANT_SIZE:
                bEqual = AMFVariantGetSize(pFirst) == AMFVariantGetSize(pSecond);
                break;
            case AMF_VARIANT_POINT:
                bEqual = AMFVariantGetPoint(pFirst) == AMFVariantGetPoint(pSecond);
                break;
            case AMF_VARIANT_RATE:
                bEqual = AMFVariantGetRate(pFirst) == AMFVariantGetRate(pSecond);
                break;
            case AMF_VARIANT_RATIO:
                bEqual = AMFVariantGetRatio(pFirst) == AMFVariantGetRatio(pSecond);
                break;
            case AMF_VARIANT_COLOR:
                bEqual = AMFVariantGetColor(pFirst) == AMFVariantGetColor(pSecond);
                break;
            case AMF_VARIANT_STRING:
                bEqual = strcmp(AMFVariantString(pFirst), AMFVariantString(pSecond)) == 0;
                break;
            case AMF_VARIANT_WSTRING:
                bEqual = wcscmp(AMFVariantWString(pFirst), AMFVariantWString(pSecond)) == 0;
                break;
            case AMF_VARIANT_INTERFACE:
                bEqual = AMFVariantInterface(pFirst) == AMFVariantInterface(pSecond);
                break;
            default:
                errRet = AMF_INVALID_ARG;
                break;
            }
        }
        return errRet;
    }
    //-------------------------------------------------------------------------------------------------
    AMF_RESULT AMF_CDECL_CALL AMFVariantCopy(AMFVariantStruct* pDest, const AMFVariantStruct* pSrc)
    {
        AMF_VARIANT_RETURN_IF_INVALID_POINTER(pDest);
        AMF_VARIANT_RETURN_IF_INVALID_POINTER(pSrc);
        AMF_RESULT errRet = AMF_OK;
        if(pDest != pSrc)
        {
            switch(AMFVariantGetType(pSrc))
            {
            case AMF_VARIANT_EMPTY:
                errRet = AMFVariantClear(pDest);
                break;
            case AMF_VARIANT_BOOL:
                errRet = AMFVariantAssignBool(pDest, AMFVariantBool(pSrc));
                break;
            case AMF_VARIANT_INT64:
                errRet = AMFVariantAssignInt64(pDest, AMFVariantInt64(pSrc));
                break;
            case AMF_VARIANT_DOUBLE:
                errRet = AMFVariantAssignDouble(pDest, AMFVariantDouble(pSrc));
                break;
            case AMF_VARIANT_RECT:
                errRet = AMFVariantAssignRect(pDest, AMFVariantGetRect(pSrc));
                break;
            case AMF_VARIANT_SIZE:
                errRet = AMFVariantAssignSize(pDest, AMFVariantGetSize(pSrc));
                break;
            case AMF_VARIANT_POINT:
                errRet = AMFVariantAssignPoint(pDest, AMFVariantGetPoint(pSrc));
                break;
            case AMF_VARIANT_RATE:
                errRet = AMFVariantAssignRate(pDest, AMFVariantGetRate(pSrc));
                break;
            case AMF_VARIANT_RATIO:
                errRet = AMFVariantAssignRatio(pDest, AMFVariantGetRatio(pSrc));
                break;
            case AMF_VARIANT_COLOR:
                errRet = AMFVariantAssignColor(pDest, AMFVariantGetColor(pSrc));
                break;
            case AMF_VARIANT_STRING:
                errRet = AMFVariantAssignString(pDest, AMFVariantString(pSrc));
                break;
            case AMF_VARIANT_WSTRING:
                errRet = AMFVariantAssignWString(pDest, AMFVariantWString(pSrc));
                break;
            case AMF_VARIANT_INTERFACE:
                errRet = AMFVariantAssignInterface(pDest, AMFVariantInterface(pSrc));
                break;
            default:
                errRet = AMF_INVALID_ARG;
                break;
            }
        }
        return errRet;
    }
    #define AMFVariantTypeEmpty      AMF_VARIANT_EMPTY

    #define AMFVariantTypeBool      AMF_VARIANT_BOOL
    #define AMFVariantTypeInt64     AMF_VARIANT_INT64
    #define AMFVariantTypeDouble    AMF_VARIANT_DOUBLE

    #define AMFVariantTypeRect      AMF_VARIANT_RECT
    #define AMFVariantTypeSize      AMF_VARIANT_SIZE
    #define AMFVariantTypePoint     AMF_VARIANT_POINT
    #define AMFVariantTypeRate      AMF_VARIANT_RATE
    #define AMFVariantTypeRatio     AMF_VARIANT_RATIO
    #define AMFVariantTypeColor     AMF_VARIANT_COLOR

    #define AMFVariantTypeString    AMF_VARIANT_STRING
    #define AMFVariantTypeWString   AMF_VARIANT_WSTRING
    #define AMFVariantTypeInterface  AMF_VARIANT_INTERFACE

    
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantAssignString(AMFVariantStruct* pDest, const AMFVariant::String& value)
    {
        return AMFVariantAssignString(pDest, value.c_str());
    }
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantAssignWString(AMFVariantStruct* pDest, const AMFVariant::WString& value)
    {
        return AMFVariantAssignWString(pDest, value.c_str());
    }
    
    inline amf_bool AMFConvertEmptyToBool(void*, AMF_RESULT& res) { res = AMF_OK; return false; }
    inline amf_int64 AMFConvertEmptyToInt64(void*, AMF_RESULT& res) {res = AMF_OK; return 0; }
    inline amf_double AMFConvertEmptyToDouble(void*, AMF_RESULT& res) {res = AMF_OK; return 0; }
    
    
    inline AMFVariant::String AMFConvertEmptyToString(void*, AMF_RESULT& res) {res = AMF_OK; return ""; }
    inline AMFVariant::WString AMFConvertEmptyToWString(void*, AMF_RESULT& res) {res = AMF_OK; return L""; }
    inline amf_int64 AMFConvertBoolToInt64(bool value, AMF_RESULT& res){res = AMF_OK; return value ? 1 : 0;}
    inline amf_double AMFConvertBoolToDouble(bool value, AMF_RESULT& res){res = AMF_OK; return value ? 1 : 0;}
    inline AMFVariant::String AMFConvertBoolToString(bool value, AMF_RESULT& res){res = AMF_OK; return value ? "true" : "false";}
    inline AMFVariant::WString AMFConvertBoolToWString(bool value, AMF_RESULT& res){res = AMF_OK; return value ? L"true" : L"false";}
    inline bool AMFConvertInt64ToBool(amf_int64 value, AMF_RESULT& res){res = AMF_OK;return value != 0;}
    inline amf_double AMFConvertInt64ToDouble(amf_int64 value, AMF_RESULT& res){res = AMF_OK;return (amf_double)value;}
    inline AMFVariant::String AMFConvertInt64ToString(amf_int64 value, AMF_RESULT& res)
    {
        res = AMF_OK;
        char buff[0xFF];
        sprintf(buff, "%" AMFPRId64, value);
        return buff;
    }
    inline AMFVariant::WString AMFConvertInt64ToWString(amf_int64 value, AMF_RESULT& res)
    {
        res = AMF_OK;
        wchar_t buff[0xFF];
        swprintf(buff, 0xFF, L"%" LPRId64, value);
        return buff;
    }

    inline bool AMFConvertDoubleToBool(amf_double value, AMF_RESULT& res){res = AMF_OK;return value != 0;}
    inline amf_int64 AMFConvertDoubleToInt64(amf_double value, AMF_RESULT& res){res = AMF_OK;return amf_int64(value);}
    inline AMFVariant::String AMFConvertDoubleToString(amf_double value, AMF_RESULT& res)
    {
        res = AMF_OK;
        char buff[0xFF];
        sprintf(buff, "%lf", value);
        return buff;
    }
    inline AMFVariant::WString AMFConvertDoubleToWString(amf_double value, AMF_RESULT& res)
    {
        res = AMF_OK;
        wchar_t buff[0xFF];
        swprintf(buff, 0xFF, L"%lf", value);
        return buff;
    }

    inline bool AMFConvertStringToBool(const AMFVariant::String& value, AMF_RESULT& res)
    {
        res = AMF_OK;
        AMFVariant::String tmp = value;
        if(( tmp == "true") || ( tmp == "True") || ( tmp == "TRUE") || ( tmp == "1") )
        {
            return true;
        }
        else
        {
            if(( tmp == "false") || ( tmp == "False") || ( tmp == "FALSE") || ( tmp == "0") )
            {
                return false;
            }
        }
        res = AMF_INVALID_ARG;
        return false;
    }

    inline amf_int64 AMFConvertStringToInt64(const AMFVariant::String& value, AMF_RESULT& res)
    {
        res = AMF_OK;
        amf_int64 tmp = 0;
        int readElements = 0;

        if(value.size() > 2 && ( value.c_str()[0] == '0') && ( value.c_str()[1] == 'x') )
        {
            readElements = sscanf(value.c_str(), "0x%" AMFPRIx64, &tmp);
        }
        else if(value.size() > 0)
        {
            readElements = sscanf(value.c_str(), "%" AMFPRId64, &tmp);
        }
        if(readElements)
        {
            return tmp;
        }
        res = AMF_INVALID_ARG;
        return 0;
    }

    inline amf_double AMFConvertStringToDouble(const AMFVariant::String& value, AMF_RESULT& res)
    {
        res = AMF_OK;
        amf_double tmp = 0;
        int readElements = 0;
        if(value.size() > 0)
        { 
            readElements = sscanf(value.c_str(), "%lf", &tmp);
        }
        if(readElements)
        {
            return tmp;
        }
        res = AMF_INVALID_ARG;
        return 0;
    }

    inline AMFVariant::WString AMFConvertStringToWString(const AMFVariant::String& value, AMF_RESULT& res)
    {
        res = AMF_OK;
//        return amf_from_utf8_to_unicode(value);
        AMFVariant::WString result;
        if(0 == value.size())
        {
            return result;
        }
        const char* pUtf8Buff = value.c_str();

#if defined(_WIN32)
        _configthreadlocale(_ENABLE_PER_THREAD_LOCALE);
        int UnicodeBuffSize = ::MultiByteToWideChar(CP_UTF8, 0, pUtf8Buff, -1, NULL, 0);
        if(0 == UnicodeBuffSize)
        {
            return result;
        }
        UnicodeBuffSize += 8; // get some extra space
        result.resize(UnicodeBuffSize);
        UnicodeBuffSize = ::MultiByteToWideChar(CP_UTF8, 0, pUtf8Buff, -1, (LPWSTR)result.c_str(), UnicodeBuffSize);
        UnicodeBuffSize--;

#elif defined(__ANDROID__)
        // on android mbstowcs cannot be used to define length
        char* old_locale = setlocale(LC_CTYPE, "en_US.UTF8");

        mbstate_t mbs;
        mbrlen(NULL, 0, &mbs);
        int len = value.size();
        const char* pt = pUtf8Buff;
        int UnicodeBuffSize = 0;
        while(len > 0)
        {
            size_t length = mbrlen (pt, len, &mbs); //MM TODO Android always return 1
            if((length == 0) || (length > len))
            {
                break;
            }
            UnicodeBuffSize++;
            len -= length;
            pt += length;
        }
        UnicodeBuffSize += 8; // get some extra space
        result.resize(UnicodeBuffSize);

        mbrlen (NULL, 0, &mbs);
        len = value.size();
        pt = pUtf8Buff;
        UnicodeBuffSize = 0;
        while(len > 0)
        {
            size_t length = mbrlen (pt, len, &mbs);
            if((length == 0) || (length > len))
            {
                break;
            }
            mbrtowc(&result[UnicodeBuffSize], pt, length, &mbs);     //MM TODO Android always return 1 char
            UnicodeBuffSize++;
            len -= length;
            pt += length;
        }
        setlocale(LC_CTYPE, old_locale);

 #else
        char* old_locale = setlocale(LC_CTYPE, "en_US.UTF8");
        int UnicodeBuffSize = mbstowcs(NULL, pUtf8Buff, 0);
        if(0 == UnicodeBuffSize)
        {
            return result;
        }
        UnicodeBuffSize += 8; // get some extra space
        result.resize(UnicodeBuffSize);
        UnicodeBuffSize = mbstowcs(&result[0], pUtf8Buff, UnicodeBuffSize);
        setlocale(LC_CTYPE, old_locale);
#endif
        result.resize(UnicodeBuffSize);
        return result;
    }
    inline AMFVariant::String AMFConvertWStringToString(const AMFVariant::WString& value, AMF_RESULT& res)
    {
        res = AMF_OK;
//      return amf_from_unicode_to_utf8(value);
        AMFVariant::String result;
        if(0 == value.size())
        {
            return result;
        }

        const wchar_t* pwBuff = value.c_str();

#if defined(_WIN32)
        _configthreadlocale(_ENABLE_PER_THREAD_LOCALE);
        int Utf8BuffSize = ::WideCharToMultiByte(CP_UTF8, 0, pwBuff, -1, NULL, 0, NULL, NULL);
        if(0 == Utf8BuffSize)
        {
            return result;
        }
        Utf8BuffSize += 8; // get some extra space
        result.resize(Utf8BuffSize);
        Utf8BuffSize = ::WideCharToMultiByte(CP_UTF8, 0, pwBuff, -1, (LPSTR)result.c_str(), Utf8BuffSize, NULL, NULL);
        Utf8BuffSize--;
#elif defined(__ANDROID__)
        char* old_locale = setlocale(LC_CTYPE, "en_US.UTF8");
        int Utf8BuffSize = value.length();
        if(0 == Utf8BuffSize)
        {
            return result;
        }
        Utf8BuffSize += 8; // get some extra space
        result.resize(Utf8BuffSize);

        mbstate_t mbs;
        mbrlen(NULL, 0, &mbs);

        Utf8BuffSize = 0;
        for( int i = 0; i < value.length(); i++)
        {
            //MM TODO Android - not implemented
            //int written = wcrtomb(&result[Utf8BuffSize], pwBuff[i], &mbs);
            result[Utf8BuffSize] = (char)(pwBuff[i]);
            int written = 1;
            // temp replacement
            Utf8BuffSize += written;
        }
        setlocale(LC_CTYPE, old_locale);

#else
        char* old_locale = setlocale(LC_CTYPE, "en_US.UTF8");
        int Utf8BuffSize = wcstombs(NULL, pwBuff, 0);
        if(0 == Utf8BuffSize)
        {
            return result;
        }
        Utf8BuffSize += 8; // get some extra space
        result.resize(Utf8BuffSize);
        Utf8BuffSize = wcstombs(&result[0], pwBuff, Utf8BuffSize);

        setlocale(LC_CTYPE, old_locale);
#endif
        result.resize(Utf8BuffSize);
        return result;
    }


    inline bool AMFConvertWStringToBool(const AMFVariant::WString& value, AMF_RESULT& res)
    {
        return AMFConvertStringToBool(AMFConvertWStringToString(value, res), res);
    }
    inline amf_int64 AMFConvertWStringToInt64(const AMFVariant::WString& value, AMF_RESULT& res)
    {
        return AMFConvertStringToInt64(AMFConvertWStringToString(value, res), res);
    }
    inline amf_double AMFConvertWStringToDouble(const AMFVariant::WString& value, AMF_RESULT& res)
    {
        return AMFConvertStringToDouble(AMFConvertWStringToString(value, res), res);
    }

    inline AMFVariant::String AMF_STD_CALL AMFConvertRectToString(const AMFRect& value, AMF_RESULT& res)
    {
        res = AMF_OK;
        char buff[0xFF];
        sprintf(buff, "%d,%d,%d,%d", value.left, value.top, value.right, value.bottom);
        return buff;
    }
    inline AMFVariant::String AMF_STD_CALL AMFConvertSizeToString(const AMFSize& value, AMF_RESULT& res)
    {
        res = AMF_OK;
        char buff[0xFF];
        sprintf(buff, "%d,%d", value.width, value.height);
        return buff;
    }
    inline AMFVariant::String AMF_STD_CALL AMFConvertPointToString(const AMFPoint& value, AMF_RESULT& res)
    {
        res = AMF_OK;
        char buff[0xFF];
        sprintf(buff, "%d,%d", value.x, value.y);
        return buff;
    }
    inline AMFVariant::String AMF_STD_CALL AMFConvertRateToString(const AMFRate& value, AMF_RESULT& res)
    {
        res = AMF_OK;
        char buff[0xFF];
        sprintf(buff, "%d,%d", value.num, value.den);
        return buff;
    }
    inline AMFVariant::String AMF_STD_CALL AMFConvertRatioToString(const AMFRatio& value, AMF_RESULT& res)
    {
        res = AMF_OK;
        char buff[0xFF];
        sprintf(buff, "%d,%d", value.num, value.den);
        return buff;
    }
    inline AMFVariant::String AMF_STD_CALL AMFConvertColorToString(const AMFColor& value, AMF_RESULT& res)
    {
        res = AMF_OK;
        char buff[0xFF];
        sprintf(buff, "%d,%d,%d,%d", value.r, value.g, value.b, value.a);
        return buff;
    }

    inline AMFRect  AMF_STD_CALL AMFConvertStringToRect(const AMFVariant::String& value, AMF_RESULT& res)
    {
        res = AMF_OK;
        AMFRect tmp = {};
        int readElements = 0;
        if(value.size() > 0)
        {
            readElements = sscanf(value.c_str(), "%d,%d,%d,%d", &tmp.left, &tmp.top, &tmp.right, &tmp.bottom);
        }
        if(readElements)
        {
            return tmp;
        }
        res = AMF_INVALID_ARG;
        return tmp;
    }

    inline AMFSize  AMF_STD_CALL AMFConvertStringToSize(const AMFVariant::String& value, AMF_RESULT& res)
    {
        res = AMF_OK;
        AMFSize tmp = {};
        int readElements = 0;
        if(value.size() > 0)
        {
            readElements = sscanf(value.c_str(), "%d,%d", &tmp.width, &tmp.height);
        }
        if(readElements)
        {
            return tmp;
        }
        res = AMF_INVALID_ARG;
        return tmp;
    }
    inline AMFPoint AMF_STD_CALL AMFConvertStringToPoint(const AMFVariant::String& value, AMF_RESULT& res)
    {
        res = AMF_OK;
        AMFPoint tmp = {};
        int readElements = 0;
        if(value.size() > 0)
        {
            readElements = sscanf(value.c_str(), "%d,%d", &tmp.x, &tmp.y);
        }
        if(readElements)
        {
            return tmp;
        }
        res = AMF_INVALID_ARG;
        return tmp;
    }
    inline AMFRate  AMF_STD_CALL AMFConvertStringToRate(const AMFVariant::String& value, AMF_RESULT& res)
    {
        res = AMF_OK;
        AMFRate tmp = {};
        int readElements = 0;
        if(value.size() > 0)
        {
            readElements = sscanf(value.c_str(), "%d,%d", &tmp.num, &tmp.den);
        }
        if(readElements)
        {
            return tmp;
        }
        res = AMF_INVALID_ARG;
        return tmp;
    }
    inline AMFRatio AMF_STD_CALL AMFConvertStringToRatio(const AMFVariant::String& value, AMF_RESULT& res)
    {
        res = AMF_OK;
        AMFRatio tmp = {};
        int readElements = 0;
        if(value.size() > 0)
        {
            readElements = sscanf(value.c_str(), "%d,%d", &tmp.num, &tmp.den);
        }
        if(readElements)
        {
            return tmp;
        }
        res = AMF_INVALID_ARG;
        return tmp;
    }
    inline AMFColor AMF_STD_CALL AMFConvertStringToColor(const AMFVariant::String& value, AMF_RESULT& res)
    {
        res = AMF_OK;
        int readElements = 0;
        amf_uint32 r = 0;
        amf_uint32 g = 0;
        amf_uint32 b = 0;
        amf_uint32 a = 0;
        if(value.size() > 0)
        { 
            readElements = sscanf(value.c_str(), "%u,%u,%u,%u", &r, &g, &b, &a);
        }
        if(readElements)
        {
            return AMFConstructColor((amf_uint8)r, (amf_uint8)g, (amf_uint8)b, (amf_uint8)a);
        }
        res = AMF_INVALID_ARG;
        return AMFConstructColor(0, 0, 0, 255);
    }
///////////////////////
    inline AMFVariant::WString AMF_STD_CALL AMFConvertRectToWString(const AMFRect& value, AMF_RESULT& res)
    {
        return AMFConvertStringToWString(AMFConvertRectToString(value, res), res);
    }
    inline AMFVariant::WString AMF_STD_CALL AMFConvertSizeToWString(const AMFSize& value, AMF_RESULT& res)
    {
        return AMFConvertStringToWString(AMFConvertSizeToString(value, res), res);
    }
    inline AMFVariant::WString AMF_STD_CALL AMFConvertPointToWString(const AMFPoint& value, AMF_RESULT& res)
    {
        return AMFConvertStringToWString(AMFConvertPointToString(value, res), res);
    }
    inline AMFVariant::WString AMF_STD_CALL AMFConvertRateToWString(const AMFRate& value, AMF_RESULT& res)
    {
        return AMFConvertStringToWString(AMFConvertRateToString(value, res), res);
    }
    inline AMFVariant::WString AMF_STD_CALL AMFConvertRatioToWString(const AMFRatio& value, AMF_RESULT& res)
    {
        return AMFConvertStringToWString(AMFConvertRatioToString(value, res), res);
    }
    inline AMFVariant::WString AMF_STD_CALL AMFConvertColorToWString(const AMFColor& value, AMF_RESULT& res)
    {
        return AMFConvertStringToWString(AMFConvertColorToString(value, res), res);
    }

    inline AMFRect  AMF_STD_CALL AMFConvertWStringToRect(const AMFVariant::WString& value, AMF_RESULT& res)
    {
        return AMFConvertStringToRect(AMFConvertWStringToString(value, res), res);
    }

    inline AMFSize  AMF_STD_CALL AMFConvertWStringToSize(const AMFVariant::WString& value, AMF_RESULT& res)
    {
        return AMFConvertStringToSize(AMFConvertWStringToString(value, res), res);
    }
    inline AMFPoint AMF_STD_CALL AMFConvertWStringToPoint(const AMFVariant::WString& value, AMF_RESULT& res)
    {
        return AMFConvertStringToPoint(AMFConvertWStringToString(value, res), res);
    }
    inline AMFRate  AMF_STD_CALL AMFConvertWStringToRate(const AMFVariant::WString& value, AMF_RESULT& res)
    {
        return AMFConvertStringToRate(AMFConvertWStringToString(value, res), res);
    }
    inline AMFRatio AMF_STD_CALL AMFConvertWStringToRatio(const AMFVariant::WString& value, AMF_RESULT& res)
    {
        return AMFConvertStringToRatio(AMFConvertWStringToString(value, res), res);
    }
    inline AMFColor AMF_STD_CALL AMFConvertWStringToColor(const AMFVariant::WString& value, AMF_RESULT& res)
    {
        return AMFConvertStringToColor(AMFConvertWStringToString(value, res), res);
    }

    //-------------------------------------------------------------------------------------------------
    #define AMFConvertTool(srcType, dstType)\
        if(AMFVariantGetType(pSrc) == AMFVariantType##srcType && newType == AMFVariantType##dstType)\
        {\
            AMF_RESULT res = AMF_OK;\
            AMFVariantAssign##dstType(pDest, AMFConvert##srcType##To##dstType(AMFVariant##srcType(pSrc), res));\
            return res;\
        }\
    //-------------------------------------------------------------------------------------------------
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantChangeType(AMFVariantStruct* pDest, const AMFVariantStruct* pSrc, AMF_VARIANT_TYPE newType)
    {
        AMF_VARIANT_RETURN_IF_INVALID_POINTER(pDest);

        if(pSrc == 0)
        {
            pSrc = pDest;
        }

        if(AMFVariantGetType(pSrc) == newType)
        {
            if(pDest == pSrc)
            {
                return AMF_OK;
            }
            return AMFVariantCopy(pDest, pSrc);
        }
        AMFVariantClear(pDest);

        AMFConvertTool(Empty, Bool);
        AMFConvertTool(Empty, Int64);
        AMFConvertTool(Empty, Double);
        AMFConvertTool(Empty, String);
        AMFConvertTool(Empty, WString);

        AMFConvertTool(Bool, Int64);
        AMFConvertTool(Bool, Double);
        AMFConvertTool(Bool, String);
        AMFConvertTool(Bool, WString);

        AMFConvertTool(Int64, Bool);
        AMFConvertTool(Int64, Double);
        AMFConvertTool(Int64, String);
        AMFConvertTool(Int64, WString);

        AMFConvertTool(Double, Bool);
        AMFConvertTool(Double, Int64);
        AMFConvertTool(Double, String);
        AMFConvertTool(Double, String);

        AMFConvertTool(String, Bool);
        AMFConvertTool(String, Int64);
        AMFConvertTool(String, Double);
        AMFConvertTool(String, WString);

        AMFConvertTool(WString, Bool);
        AMFConvertTool(WString, Int64);
        AMFConvertTool(WString, Double);
        AMFConvertTool(WString, String);

        AMFConvertTool(String, Rect);
        AMFConvertTool(String, Size);
        AMFConvertTool(String, Point);
        AMFConvertTool(String, Rate);
        AMFConvertTool(String, Ratio);
        AMFConvertTool(String, Color);

        AMFConvertTool(Rect , String);
        AMFConvertTool(Size , String);
        AMFConvertTool(Point, String);
        AMFConvertTool(Rate , String);
        AMFConvertTool(Ratio, String);
        AMFConvertTool(Color, String);

        AMFConvertTool(WString, Rect);
        AMFConvertTool(WString, Size);
        AMFConvertTool(WString, Point);
        AMFConvertTool(WString, Rate);
        AMFConvertTool(WString, Ratio);
        AMFConvertTool(WString, Color);

        AMFConvertTool(Rect , WString);
        AMFConvertTool(Size , WString);
        AMFConvertTool(Point, WString);
        AMFConvertTool(Rate , WString);
        AMFConvertTool(Ratio, WString);
        AMFConvertTool(Color, WString);

        return AMF_INVALID_ARG;
    }
    //-------------------------------------------------------------------------------------------------
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantAssignBool(AMFVariantStruct* pDest, bool value)
    {
        AMF_VARIANT_RETURN_IF_INVALID_POINTER(pDest);
        AMF_RESULT errRet = AMF_OK;

        errRet = AMFVariantClear(pDest);
        if(errRet == AMF_OK)
        {
            AMFVariantGetType(pDest) = AMF_VARIANT_BOOL;
            AMFVariantBool(pDest) = value;
        }
        return errRet;
    }
    //-------------------------------------------------------------------------------------------------
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantAssignInt64(AMFVariantStruct* pDest, amf_int64 value)
    {
        AMF_VARIANT_RETURN_IF_INVALID_POINTER(pDest);
        AMF_RESULT errRet = AMF_OK;

        errRet = AMFVariantClear(pDest);
        if(errRet == AMF_OK)
        {
            AMFVariantGetType(pDest) = AMF_VARIANT_INT64;
            AMFVariantInt64(pDest) = value;
        }
        return errRet;
    }
    //-------------------------------------------------------------------------------------------------
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantAssignDouble(AMFVariantStruct* pDest, amf_double value)
    {
        AMF_VARIANT_RETURN_IF_INVALID_POINTER(pDest);
        AMF_RESULT errRet = AMF_OK;

        errRet = AMFVariantClear(pDest);
        if(errRet == AMF_OK)
        {
            AMFVariantGetType(pDest) = AMF_VARIANT_DOUBLE;
            AMFVariantDouble(pDest) = value;
        }
        return errRet;
    }
    //-------------------------------------------------------------------------------------------------
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantAssignString(AMFVariantStruct* pDest, const char* pValue)
    {
        AMF_VARIANT_RETURN_IF_INVALID_POINTER(pDest);
        AMF_VARIANT_RETURN_IF_INVALID_POINTER(pValue);
        AMF_RESULT errRet = AMF_OK;

        errRet = AMFVariantClear(pDest);
        if(errRet == AMF_OK)
        {
            AMFVariantGetType(pDest) = AMF_VARIANT_STRING;
            size_t size = (strlen(pValue) + 1);
            AMFVariantString(pDest) = (char*)amf_variant_alloc(size * sizeof(char));
            if(AMFVariantString(pDest))
            {
                strncpy(AMFVariantString(pDest), pValue, size);
            }
            else
            {
                errRet = AMF_OUT_OF_MEMORY;
            }
        }
        return errRet;
    }
    //-------------------------------------------------------------------------------------------------
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantAssignWString(AMFVariantStruct* pDest, const wchar_t* pValue)
    {
        AMF_VARIANT_RETURN_IF_INVALID_POINTER(pDest);
        AMF_VARIANT_RETURN_IF_INVALID_POINTER(pValue);
        AMF_RESULT errRet = AMF_OK;

        errRet = AMFVariantClear(pDest);
        if(errRet == AMF_OK)
        {
            AMFVariantGetType(pDest) = AMF_VARIANT_WSTRING;
            size_t size = (wcslen(pValue) + 1);
            AMFVariantWString(pDest) = (wchar_t*)amf_variant_alloc(size * sizeof(wchar_t));
            if(AMFVariantWString(pDest))
            {
                wcsncpy(AMFVariantWString(pDest), pValue, size);
            }
            else
            {
                errRet = AMF_OUT_OF_MEMORY;
            }
        }
        return errRet;
    }
    //-------------------------------------------------------------------------------------------------
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantAssignInterface(AMFVariantStruct* pDest, AMFInterface* pValue)
    {
        AMF_VARIANT_RETURN_IF_INVALID_POINTER(pDest);
        //AMF_VARIANT_RETURN_IF_INVALID_POINTER(pValue);//can be NULL

        AMF_RESULT errRet = AMF_OK;
        errRet = AMFVariantClear(pDest);
        if(errRet == AMF_OK)
        {
            AMFVariantGetType(pDest) = AMF_VARIANT_INTERFACE;
            AMFVariantInterface(pDest) = pValue;
            if(AMFVariantInterface(pDest))
            {
                AMFVariantInterface(pDest)->Acquire();
            }
        }
        return errRet;
    }
    //-------------------------------------------------------------------------------------------------
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantAssignRect (AMFVariantStruct* pDest, const AMFRect& value)
    {
        AMF_VARIANT_RETURN_IF_INVALID_POINTER(pDest);
        AMF_RESULT errRet = AMF_OK;

        errRet = AMFVariantClear(pDest);
        if(errRet == AMF_OK)
        {
            AMFVariantGetType(pDest) = AMF_VARIANT_RECT;
            AMFVariantRect(pDest) = value;
        }
        return errRet;
    }
    //-------------------------------------------------------------------------------------------------
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantAssignSize (AMFVariantStruct* pDest, const AMFSize& value)
    {
        AMF_VARIANT_RETURN_IF_INVALID_POINTER(pDest);
        AMF_RESULT errRet = AMF_OK;

        errRet = AMFVariantClear(pDest);
        if(errRet == AMF_OK)
        {
            AMFVariantGetType(pDest) = AMF_VARIANT_SIZE;
            AMFVariantSize(pDest) = value;
        }
        return errRet;
    }
    //-------------------------------------------------------------------------------------------------
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantAssignPoint(AMFVariantStruct* pDest, const AMFPoint& value)
    {
        AMF_VARIANT_RETURN_IF_INVALID_POINTER(pDest);
        AMF_RESULT errRet = AMF_OK;

        errRet = AMFVariantClear(pDest);
        if(errRet == AMF_OK)
        {
            AMFVariantGetType(pDest) = AMF_VARIANT_POINT;
            AMFVariantPoint(pDest) = value;
        }
        return errRet;
    }
    //-------------------------------------------------------------------------------------------------
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantAssignRate (AMFVariantStruct* pDest, const AMFRate& value)
    {
        AMF_VARIANT_RETURN_IF_INVALID_POINTER(pDest);
        AMF_RESULT errRet = AMF_OK;

        errRet = AMFVariantClear(pDest);
        if(errRet == AMF_OK)
        {
            AMFVariantGetType(pDest) = AMF_VARIANT_RATE;
            AMFVariantRate(pDest) = value;
        }
        return errRet;
    }
    //-------------------------------------------------------------------------------------------------
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantAssignRatio(AMFVariantStruct* pDest, const AMFRatio& value)
    {
        AMF_VARIANT_RETURN_IF_INVALID_POINTER(pDest);
        AMF_RESULT errRet = AMF_OK;

        errRet = AMFVariantClear(pDest);
        if(errRet == AMF_OK)
        {
            AMFVariantGetType(pDest) = AMF_VARIANT_RATIO;
            AMFVariantRatio(pDest) = value;
        }
        return errRet;
    }
    //-------------------------------------------------------------------------------------------------
    inline AMF_RESULT AMF_CDECL_CALL AMFVariantAssignColor(AMFVariantStruct* pDest, const AMFColor& value)
    {
        AMF_VARIANT_RETURN_IF_INVALID_POINTER(pDest);
        AMF_RESULT errRet = AMF_OK;

        errRet = AMFVariantClear(pDest);
        if(errRet == AMF_OK)
        {
            AMFVariantGetType(pDest) = AMF_VARIANT_COLOR;
            AMFVariantColor(pDest) = value;
        }
        return errRet;
    }
    //-------------------------------------------------------------------------------------------------
    inline char* AMF_CDECL_CALL AMFVariantDuplicateString(const char* from)
    {
        char* ret = 0;
        if(from)
        {
            ret = (char*)amf_variant_alloc(sizeof(char)*(strlen(from) + 1));
            if(ret)
            {
                strcpy(ret, from);
            }
        }
        return ret;
    }
    //-------------------------------------------------------------------------------------------------
    inline void AMF_CDECL_CALL AMFVariantFreeString(char* from)
    {
        amf_variant_free(from);
    }
    //-------------------------------------------------------------------------------------------------
    inline wchar_t* AMF_CDECL_CALL AMFVariantDuplicateWString(const wchar_t* from)
    {
        wchar_t* ret = 0;
        if(from)
        {
            ret = (wchar_t*)amf_variant_alloc(sizeof(wchar_t)*(wcslen(from) + 1));
            if(ret)
            {
                wcscpy(ret, from);
            }
        }
        return ret;
    }
    //-------------------------------------------------------------------------------------------------
    inline void AMF_CDECL_CALL AMFVariantFreeWString(wchar_t* from)
    {
        amf_variant_free(from);
    }
    //----------------------------------------------------------------------------------------------
    // inline implementation of AMFVariant class
    //----------------------------------------------------------------------------------------------
    inline AMFVariant::AMFVariant(const AMFVariantStruct* pOther)
    {
        AMFVariantInit(this);
        if(pOther != NULL)
        {
            AMFVariantCopy(this, const_cast<AMFVariantStruct*>(pOther));
        }
    }
    //-------------------------------------------------------------------------------------------------
    template<typename T>
    AMFVariant::AMFVariant(const AMFInterfacePtr_T<T>& pValue)
    {
        AMFVariantInit(this);
        AMFVariantAssignInterface(this, pValue);
    }
    //-------------------------------------------------------------------------------------------------
    template<class ReturnType, AMF_VARIANT_TYPE variantType, typename Getter>
    ReturnType AMFVariant::GetValue(Getter getter) const
    {
        ReturnType str = ReturnType();
        if(AMFVariantGetType(this) == variantType)
        {
            str = static_cast<ReturnType>(getter(this));
        }
        else
        {
            AMFVariant varDest;
            varDest.ChangeType(variantType, this);
            if(varDest.type != AMF_VARIANT_EMPTY)
            {
                str = static_cast<ReturnType>(getter(&varDest));
            }
        }
        return str;
    }
    //-------------------------------------------------------------------------------------------------
    inline AMFVariant& AMFVariant::operator=(const AMFVariantStruct& other)
    {
        AMFVariantCopy(this, const_cast<AMFVariantStruct*>(&other));
        return *this;
    }
    //-------------------------------------------------------------------------------------------------
    inline AMFVariant& AMFVariant::operator=(const AMFVariantStruct* pOther)
    {
        if(pOther != NULL)
        {
            AMFVariantCopy(this, const_cast<AMFVariantStruct*>(pOther));
        }
        return *this;
    }
    //-------------------------------------------------------------------------------------------------
    inline AMFVariant& AMFVariant::operator=(const AMFVariant& other)
    {
        AMFVariantCopy(this,
                const_cast<AMFVariantStruct*>(static_cast<const AMFVariantStruct*>(&other)));
        return *this;
    }
    //-------------------------------------------------------------------------------------------------
    template<typename T>
    AMFVariant& AMFVariant::operator=(const AMFInterfacePtr_T<T>& value)
    {
        AMFVariantAssignInterface(this, value);
        return *this;
    }
    //-------------------------------------------------------------------------------------------------
    inline bool AMFVariant::operator==(const AMFVariantStruct& other) const
    {
        return *this == &other;
    }
    //-------------------------------------------------------------------------------------------------
    inline bool AMFVariant::operator==(const AMFVariantStruct* pOther) const
    {
        //TODO: double check
        bool ret = false;
        if(pOther == NULL)
        {
            ret = false;
        }
        else
        {
            AMFVariantCompare(this, pOther, ret);
        }
        return ret;
    }
    //-------------------------------------------------------------------------------------------------
    inline bool AMFVariant::operator!=(const AMFVariantStruct& other) const
    {
        return !(*this == &other);
    }
    //-------------------------------------------------------------------------------------------------
    inline bool AMFVariant::operator!=(const AMFVariantStruct* pOther) const
    {
        return !(*this == pOther);
    }
    //-------------------------------------------------------------------------------------------------
    inline void AMFVariant::Attach(AMFVariantStruct& variant)
    {
        Clear();
        memcpy(this, &variant, sizeof(variant));
        AMFVariantGetType(&variant) = AMF_VARIANT_EMPTY;
    }
    //-------------------------------------------------------------------------------------------------
    inline AMFVariantStruct AMFVariant::Detach()
    {
        AMFVariantStruct varResult = *this;
        AMFVariantGetType(this) = AMF_VARIANT_EMPTY;
        return varResult;
    }
    //-------------------------------------------------------------------------------------------------
    inline AMFVariantStruct& AMFVariant::GetVariant()
    {
        return *static_cast<AMFVariantStruct*>(this);
    }
    //-------------------------------------------------------------------------------------------------
    inline void AMFVariant::ChangeType(AMF_VARIANT_TYPE newType, const AMFVariant* pSrc)
    {
        AMFVariantChangeType(this, pSrc, newType);
    }
    //-------------------------------------------------------------------------------------------------
    inline bool AMFVariant::Empty() const
    {
        return type == AMF_VARIANT_EMPTY;
    }
    //-------------------------------------------------------------------------------------------------
} //namespace amf

#endif //#ifndef __AMFVariant_h__
