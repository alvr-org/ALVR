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

#include "IOCapsImpl.h"

namespace amf
{
    ////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
    AMFIOCapsImpl::SurfaceFormat::SurfaceFormat() :
        m_Format(AMF_SURFACE_UNKNOWN),
        m_Native(false)
    {
    }

    AMFIOCapsImpl::SurfaceFormat::SurfaceFormat(AMF_SURFACE_FORMAT format, amf_bool native) :
        m_Format(format),
        m_Native(native)
    {
    }

    ////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
    AMFIOCapsImpl::MemoryType::MemoryType() :
        m_Type(AMF_MEMORY_UNKNOWN),
        m_Native(false)
    {
    }

    AMFIOCapsImpl::MemoryType::MemoryType(AMF_MEMORY_TYPE type, amf_bool native) :
        m_Type(type),
        m_Native(native)
    {
    }


    ////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

    AMFIOCapsImpl::AMFIOCapsImpl() :
        m_MinWidth(-1),
        m_MaxWidth(-1),
        m_MinHeight(-1),
        m_MaxHeight(-1),
        m_VertAlign(-1),
        m_InterlacedSupported(false)
    {
    }

    AMFIOCapsImpl::AMFIOCapsImpl(amf_int32 minWidth, amf_int32 maxWidth,
                      amf_int32 minHeight, amf_int32 maxHeight,
                      amf_int32 vertAlign, amf_bool interlacedSupport,
                      amf_int32 numOfNativeFormats, const AMF_SURFACE_FORMAT* nativeFormats,
                      amf_int32 numOfNonNativeFormats, const AMF_SURFACE_FORMAT* nonNativeFormats,
                      amf_int32 numOfNativeMemTypes, const AMF_MEMORY_TYPE* nativeMemTypes,
                      amf_int32 numOfNonNativeMemTypes, const AMF_MEMORY_TYPE* nonNativeMemTypes)
    {
        m_MinWidth = minWidth;
        m_MaxWidth = maxWidth;
        m_MinHeight = minHeight;
        m_MaxHeight = maxHeight;
        m_VertAlign = vertAlign;
        m_InterlacedSupported = interlacedSupport;
        PopulateSurfaceFormats(numOfNativeFormats, nativeFormats, true);
        PopulateSurfaceFormats(numOfNonNativeFormats, nonNativeFormats, false);
        PopulateMemoryTypes(numOfNativeMemTypes, nativeMemTypes, true);
        PopulateMemoryTypes(numOfNonNativeMemTypes, nonNativeMemTypes, false);
    }

    void AMFIOCapsImpl::PopulateSurfaceFormats(amf_int32 numOfFormats, const AMF_SURFACE_FORMAT* formats, amf_bool native)
    {
        if (formats != NULL)
        {
            for (amf_int32 i = 0; i < numOfFormats; i++)
            {
                bool found = false;
                for(amf_size exists_idx = 0; exists_idx < m_SurfaceFormats.size(); exists_idx++)
                {
                    if(m_SurfaceFormats[exists_idx].GetFormat() == formats[i])
                    {
                        found = true;
                    }
                }
                if(!found)
                {
                    m_SurfaceFormats.push_back(SurfaceFormat(formats[i], native));
                }
            }
        }
    }

    void AMFIOCapsImpl::PopulateMemoryTypes(amf_int32 numOfTypes, const AMF_MEMORY_TYPE* memTypes, amf_bool native)
    {
        if (memTypes != NULL)
        {
            for (amf_int32 i = 0; i < numOfTypes; i++)
            {
                bool found = false;
                for(amf_size exists_idx = 0; exists_idx < m_MemoryTypes.size(); exists_idx++)
                {
                    if(m_MemoryTypes[exists_idx].GetType() == memTypes[i])
                    {
                        found = true;
                    }
                }
                if(!found)
                {
                    m_MemoryTypes.push_back(MemoryType(memTypes[i], native));
                }
            }
        }
    }

    //  Get supported resolution ranges in pixels/lines:
    void AMF_STD_CALL AMFIOCapsImpl::GetWidthRange(amf_int32* minWidth, amf_int32* maxWidth) const
    {
        if (minWidth != NULL)
        {
            *minWidth = m_MinWidth;
        }
        if (maxWidth != NULL)
        {
            *maxWidth = m_MaxWidth;
        }
    }

    void AMF_STD_CALL AMFIOCapsImpl::GetHeightRange(amf_int32* minHeight, amf_int32* maxHeight) const
    {
        if (minHeight != NULL)
        {
            *minHeight = m_MinHeight;
        }
        if (maxHeight != NULL)
        {
            *maxHeight = m_MaxHeight;
        }
    }

    //  Get memory alignment in lines:
    //  Vertical aligmnent should be multiples of this number
    amf_int32 AMF_STD_CALL AMFIOCapsImpl::GetVertAlign() const
    {
        return m_VertAlign;
    }

    //  Enumerate supported surface pixel formats:
    amf_int32 AMF_STD_CALL AMFIOCapsImpl::GetNumOfFormats() const
    {
        return (amf_int32)m_SurfaceFormats.size();
    }

    AMF_RESULT AMF_STD_CALL AMFIOCapsImpl::GetFormatAt(amf_int32 index, AMF_SURFACE_FORMAT* format, bool* native) const
    {
        if (index >= 0 && index < static_cast<amf_int32>(m_SurfaceFormats.size()))
        {
            SurfaceFormat curFormat(m_SurfaceFormats.at(index));
            if (format != NULL)
            {
                *format = curFormat.GetFormat();
            }
            if (native != NULL)
            {
                *native = curFormat.IsNative();
            }
            return AMF_OK;
        }
        else
        {
            return AMF_INVALID_ARG;
        }
    }

    //  Enumerate supported surface formats:
    amf_int32 AMF_STD_CALL AMFIOCapsImpl::GetNumOfMemoryTypes() const
    {
        return (amf_int32)m_MemoryTypes.size();
    }

    AMF_RESULT AMF_STD_CALL AMFIOCapsImpl::GetMemoryTypeAt(amf_int32 index, AMF_MEMORY_TYPE* memType, bool* native) const
    {
        if (index >= 0 && index < static_cast<amf_int32>(m_MemoryTypes.size()))
        {
            MemoryType curType(m_MemoryTypes.at(index));
            if (memType != NULL)
            {
                *memType = curType.GetType();
            }
            if (native != NULL)
            {
                *native = curType.IsNative();
            }
            return AMF_OK;
        }
        else
        {
            return AMF_INVALID_ARG;
        }
    }

    //  interlaced support:
    amf_bool AMF_STD_CALL AMFIOCapsImpl::IsInterlacedSupported() const
    {
        return m_InterlacedSupported;
    }

    void AMFIOCapsImpl::SetResolution(amf_int32 minWidth, amf_int32 maxWidth, amf_int32 minHeight, amf_int32 maxHeight)
    {
        m_MinWidth = minWidth;
        m_MaxWidth = maxWidth;
        m_MinHeight = minHeight;
        m_MaxHeight = maxHeight;
    }

    void AMFIOCapsImpl::SetVertAlign(amf_int32 vertAlign)
    {
        m_VertAlign = vertAlign;
    }

    void AMFIOCapsImpl::SetInterlacedSupport(amf_bool interlaced)
    {
        m_InterlacedSupported = interlaced;
    }

}