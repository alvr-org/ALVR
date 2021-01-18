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

#ifndef __AMFComponentCaps_h__
#define __AMFComponentCaps_h__

#pragma once

#include "../core/Interface.h"
#include "../core/PropertyStorage.h"
#include "../core/Surface.h"

namespace amf
{
    enum AMF_ACCELERATION_TYPE
    {
        AMF_ACCEL_NOT_SUPPORTED = -1,
        AMF_ACCEL_HARDWARE,
        AMF_ACCEL_GPU,
        AMF_ACCEL_SOFTWARE
    };
    //----------------------------------------------------------------------------------------------
    // AMFIOCaps interface
    //----------------------------------------------------------------------------------------------
    class AMF_NO_VTABLE AMFIOCaps : public AMFInterface
    {
    public:
        //  Get supported resolution ranges in pixels/lines:
        virtual void AMF_STD_CALL GetWidthRange(amf_int32* minWidth, amf_int32* maxWidth) const = 0;
        virtual void AMF_STD_CALL GetHeightRange(amf_int32* minHeight, amf_int32* maxHeight) const = 0;

        //  Get memory alignment in lines: Vertical aligmnent should be multiples of this number
        virtual amf_int32 AMF_STD_CALL GetVertAlign() const = 0;
        
        //  Enumerate supported surface pixel formats
        virtual amf_int32 AMF_STD_CALL GetNumOfFormats() const = 0;
        virtual  AMF_RESULT AMF_STD_CALL GetFormatAt(amf_int32 index, AMF_SURFACE_FORMAT* format, amf_bool* native) const = 0;

        //  Enumerate supported memory types
        virtual amf_int32 AMF_STD_CALL GetNumOfMemoryTypes() const = 0;
        virtual AMF_RESULT AMF_STD_CALL GetMemoryTypeAt(amf_int32 index, AMF_MEMORY_TYPE* memType, amf_bool* native) const = 0;

        virtual amf_bool AMF_STD_CALL IsInterlacedSupported() const = 0;
    };
    //----------------------------------------------------------------------------------------------
    // smart pointer
    //----------------------------------------------------------------------------------------------
    typedef AMFInterfacePtr_T<AMFIOCaps>    AMFIOCapsPtr;
    
    //----------------------------------------------------------------------------------------------
    // AMFCaps interface - base interface for every h/w module supported by Capability Manager
    //----------------------------------------------------------------------------------------------
    class AMF_NO_VTABLE AMFCaps : public AMFPropertyStorage
    {
    public:
        virtual AMF_ACCELERATION_TYPE AMF_STD_CALL GetAccelerationType() const = 0;
        virtual AMF_RESULT AMF_STD_CALL GetInputCaps(AMFIOCaps** input) = 0;
        virtual AMF_RESULT AMF_STD_CALL GetOutputCaps(AMFIOCaps** output) = 0;
    };
    //----------------------------------------------------------------------------------------------
    // smart pointer
    //----------------------------------------------------------------------------------------------
    typedef AMFInterfacePtr_T<AMFCaps>  AMFCapsPtr;
    //----------------------------------------------------------------------------------------------
}

#endif //#ifndef __AMFComponentCaps_h__
