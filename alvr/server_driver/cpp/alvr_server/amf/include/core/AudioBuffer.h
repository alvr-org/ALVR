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

#ifndef __AMFAudioBuffer_h__
#define __AMFAudioBuffer_h__
#pragma once

#include "Data.h"
#pragma warning( push )
#pragma warning(disable : 4263)
#pragma warning(disable : 4264)

namespace amf
{
    enum AMF_AUDIO_FORMAT
    {
        AMFAF_UNKNOWN   =-1,
        AMFAF_U8        = 0,               // amf_uint8
        AMFAF_S16       = 1,               // amf_int16
        AMFAF_S32       = 2,               // amf_int32
        AMFAF_FLT       = 3,               // amf_float
        AMFAF_DBL       = 4,               // amf_double

        AMFAF_U8P       = 5,               // amf_uint8
        AMFAF_S16P      = 6,               // amf_int16
        AMFAF_S32P      = 7,               // amf_int32
        AMFAF_FLTP      = 8,               // amf_float
        AMFAF_DBLP      = 9,               // amf_double
        AMFAF_FIRST     = AMFAF_U8,
        AMFAF_LAST      = AMFAF_DBLP,
    };

    //----------------------------------------------------------------------------------------------
    // AMFBufferObserver interface - callback
    //----------------------------------------------------------------------------------------------
    class AMFAudioBuffer;
    class AMF_NO_VTABLE AMFAudioBufferObserver
    {
    public:
        virtual void                AMF_STD_CALL OnBufferDataRelease(AMFAudioBuffer* pBuffer) = 0;
    };

    //----------------------------------------------------------------------------------------------
    // AudioBuffer interface
    //----------------------------------------------------------------------------------------------
    class AMF_NO_VTABLE AMFAudioBuffer : public AMFData
    {
    public:
        AMF_DECLARE_IID(0x2212ff8, 0x6107, 0x430b, 0xb6, 0x3c, 0xc7, 0xe5, 0x40, 0xe5, 0xf8, 0xeb)

        virtual amf_int32           AMF_STD_CALL GetSampleCount() = 0;
        virtual amf_int32           AMF_STD_CALL GetSampleRate() = 0;
        virtual amf_int32           AMF_STD_CALL GetChannelCount() = 0;
        virtual AMF_AUDIO_FORMAT    AMF_STD_CALL GetSampleFormat() = 0;
        virtual amf_int32           AMF_STD_CALL GetSampleSize() = 0;
        virtual amf_uint32          AMF_STD_CALL GetChannelLayout() = 0;
        virtual void*               AMF_STD_CALL GetNative() = 0;
        virtual amf_size            AMF_STD_CALL GetSize() = 0;

        // Observer management
        virtual void                AMF_STD_CALL AddObserver(AMFAudioBufferObserver* pObserver) = 0;
        virtual void                AMF_STD_CALL RemoveObserver(AMFAudioBufferObserver* pObserver) = 0;

    };
    //----------------------------------------------------------------------------------------------
    // smart pointer
    //----------------------------------------------------------------------------------------------
    typedef AMFInterfacePtr_T<AMFAudioBuffer> AMFAudioBufferPtr;
    //----------------------------------------------------------------------------------------------
} // namespace
#pragma warning( pop )

#endif //#ifndef __AMFAudioBuffer_h__
