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

#ifndef AMF_IOCapsImpl_h
#define AMF_IOCapsImpl_h

#pragma once

#include "InterfaceImpl.h"
#include "../include/components/ComponentCaps.h"
#include <vector>

namespace amf
{
    class AMFIOCapsImpl : public AMFInterfaceImpl<AMFIOCaps>
    {
    protected:
        class SurfaceFormat
        {
        public:
            typedef std::vector<SurfaceFormat>  Collection;
        public:
            SurfaceFormat();
            SurfaceFormat(AMF_SURFACE_FORMAT format, amf_bool native);

            inline AMF_SURFACE_FORMAT GetFormat() const throw() { return m_Format; }
            inline amf_bool IsNative() const throw() { return m_Native; }
        private:
            AMF_SURFACE_FORMAT  m_Format;
            amf_bool    m_Native;
        };

        class MemoryType
        {
        public:
            typedef std::vector<MemoryType>  Collection;
        public:
            MemoryType();
            MemoryType(AMF_MEMORY_TYPE type, amf_bool native);

            inline AMF_MEMORY_TYPE GetType() const throw() { return m_Type; }
            inline amf_bool IsNative() const throw() { return m_Native; }
        private:
            AMF_MEMORY_TYPE m_Type;
            amf_bool    m_Native;
        };

        struct Resolution
        {
            amf_int32   m_Width;
            amf_int32   m_Height;
        };

    protected:
        AMFIOCapsImpl();
        AMFIOCapsImpl(amf_int32 minWidth, amf_int32 maxWidth, 
                      amf_int32 minHeight, amf_int32 maxHeight,
                      amf_int32 vertAlign, amf_bool interlacedSupport,
                      amf_int32 numOfNativeFormats, const AMF_SURFACE_FORMAT* nativeFormats,
                      amf_int32 numOfNonNativeFormats, const AMF_SURFACE_FORMAT* nonNativeFormats,
                      amf_int32 numOfNativeMemTypes, const AMF_MEMORY_TYPE* nativeMemTypes,
                      amf_int32 numOfNonNativeMemTypes, const AMF_MEMORY_TYPE* nonNativeMemTypes);

    public:
        //  Get supported resolution ranges in pixels/lines:
        virtual void AMF_STD_CALL GetWidthRange(amf_int32* minWidth, amf_int32* maxWidth) const;
        virtual void AMF_STD_CALL GetHeightRange(amf_int32* minHeight, amf_int32* maxHeight) const;

        //  Get memory alignment in lines:
        //  Vertical aligmnent should be multiples of this number
        virtual amf_int32 AMF_STD_CALL GetVertAlign() const;
        
        //  Enumerate supported surface pixel formats:
        virtual amf_int32 AMF_STD_CALL GetNumOfFormats() const;
        virtual  AMF_RESULT AMF_STD_CALL GetFormatAt(amf_int32 index, AMF_SURFACE_FORMAT* format, amf_bool* native) const;

        //  Enumerate supported surface formats:
        virtual amf_int32 AMF_STD_CALL GetNumOfMemoryTypes() const;
        virtual AMF_RESULT AMF_STD_CALL GetMemoryTypeAt(amf_int32 index, AMF_MEMORY_TYPE* memType, amf_bool* native) const;

        //  interlaced support:
        virtual amf_bool AMF_STD_CALL IsInterlacedSupported() const;

    protected:
        void SetResolution(amf_int32 minWidth, amf_int32 maxWidth, amf_int32 minHeight, amf_int32 maxHeight);
        void SetVertAlign(amf_int32 alignment);
        void SetInterlacedSupport(amf_bool interlaced);
        void PopulateSurfaceFormats(amf_int32 numOfFormats, const AMF_SURFACE_FORMAT* formats, amf_bool native);
        void PopulateMemoryTypes(amf_int32 numOfTypes, const AMF_MEMORY_TYPE* memTypes, amf_bool native);


    protected:
        amf_int32   m_MinWidth;
        amf_int32   m_MaxWidth;
        amf_int32   m_MinHeight;
        amf_int32   m_MaxHeight;
        amf_int32   m_VertAlign;
        amf_bool    m_InterlacedSupported;
        SurfaceFormat::Collection   m_SurfaceFormats;
        MemoryType::Collection      m_MemoryTypes;
    };
}
#endif // AMF_IOCapsImpl_h