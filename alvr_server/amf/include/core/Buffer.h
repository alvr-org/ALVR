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

#ifndef __AMFBuffer_h__
#define __AMFBuffer_h__
#pragma once

#include "Data.h"

#pragma warning( push )
#pragma warning(disable : 4263)
#pragma warning(disable : 4264)

namespace amf
{
    //----------------------------------------------------------------------------------------------
    // AMFBufferObserver interface - callback
    //----------------------------------------------------------------------------------------------
    class AMFBuffer;
    class AMF_NO_VTABLE AMFBufferObserver
    {
    public:
        virtual void                AMF_STD_CALL OnBufferDataRelease(AMFBuffer* pBuffer) = 0;
    };
    //----------------------------------------------------------------------------------------------
    // AMFBuffer interface
    //----------------------------------------------------------------------------------------------
    class AMF_NO_VTABLE AMFBuffer : public AMFData
    {
    public:
        AMF_DECLARE_IID(0xb04b7248, 0xb6f0, 0x4321, 0xb6, 0x91, 0xba, 0xa4, 0x74, 0xf, 0x9f, 0xcb)

        virtual AMF_RESULT          AMF_STD_CALL SetSize(amf_size newSize) = 0;
        virtual amf_size            AMF_STD_CALL GetSize() = 0;
        virtual void*               AMF_STD_CALL GetNative() = 0;

        // Observer management
        virtual void                AMF_STD_CALL AddObserver(AMFBufferObserver* pObserver) = 0;
        virtual void                AMF_STD_CALL RemoveObserver(AMFBufferObserver* pObserver) = 0;
    };
    //----------------------------------------------------------------------------------------------
    // smart pointer
    //----------------------------------------------------------------------------------------------
    typedef AMFInterfacePtr_T<AMFBuffer> AMFBufferPtr;
    //----------------------------------------------------------------------------------------------
} // namespace
#pragma warning( pop )

#endif //#ifndef __AMFBuffer_h__
