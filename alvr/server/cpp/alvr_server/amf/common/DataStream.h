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

/**
 ***************************************************************************************************
 * @file  DataStream.h
 * @brief AMFDataStream declaration
 ***************************************************************************************************
 */
#ifndef AMF_DataStream_h
#define AMF_DataStream_h
#pragma once

#include "../include/core/Interface.h"

namespace amf
{
    // currently supports only
    // file://
    // memory://

    // eventually can be extended with:
    // rtsp://
    // rtmp://
    // http://
    // etc

    //----------------------------------------------------------------------------------------------
    enum AMF_STREAM_OPEN
    {
        AMFSO_READ              = 0,
        AMFSO_WRITE             = 1,
        AMFSO_READ_WRITE        = 2,
        AMFSO_APPEND            = 3,
    };
    //----------------------------------------------------------------------------------------------
    enum AMF_FILE_SHARE
    {
        AMFFS_EXCLUSIVE         = 0,
        AMFFS_SHARE_READ        = 1,
        AMFFS_SHARE_WRITE       = 2,
        AMFFS_SHARE_READ_WRITE  = 3,
    };
    //----------------------------------------------------------------------------------------------
    enum AMF_SEEK_ORIGIN
    {
        AMF_SEEK_BEGIN          = 0,
        AMF_SEEK_CURRENT        = 1,
        AMF_SEEK_END            = 2,
    };
    //----------------------------------------------------------------------------------------------
    // AMFDataStream interface
    //----------------------------------------------------------------------------------------------
    class AMF_NO_VTABLE AMFDataStream : public AMFInterface
    {
    public:
        AMF_DECLARE_IID(0xdb08fe70, 0xb743, 0x4c26, 0xb2, 0x77, 0xa5, 0xc8, 0xe8, 0x14, 0xda, 0x4)

        // interface
        virtual AMF_RESULT          AMF_STD_CALL Open(const wchar_t* pFileUrl, AMF_STREAM_OPEN eOpenType, AMF_FILE_SHARE eShareType) = 0;
        virtual AMF_RESULT          AMF_STD_CALL Close() = 0;
        virtual AMF_RESULT          AMF_STD_CALL Read(void* pData, amf_size iSize, amf_size* pRead) = 0;
        virtual AMF_RESULT          AMF_STD_CALL Write(const void* pData, amf_size iSize, amf_size* pWritten) = 0;
        virtual AMF_RESULT          AMF_STD_CALL Seek(AMF_SEEK_ORIGIN eOrigin, amf_int64 iPosition, amf_int64* pNewPosition) = 0;
        virtual AMF_RESULT          AMF_STD_CALL GetPosition(amf_int64* pPosition) = 0;
        virtual AMF_RESULT          AMF_STD_CALL GetSize(amf_int64* pSize) = 0;
        virtual bool                AMF_STD_CALL IsSeekable() = 0;

        static AMF_RESULT          AMF_STD_CALL OpenDataStream(const wchar_t* pFileUrl, AMF_STREAM_OPEN eOpenType, AMF_FILE_SHARE eShareType, AMFDataStream** str);

    };
    //----------------------------------------------------------------------------------------------
    // smart pointer
    //----------------------------------------------------------------------------------------------
    typedef AMFInterfacePtr_T<AMFDataStream> AMFDataStreamPtr;
    //----------------------------------------------------------------------------------------------
    
} //namespace amf

#endif // AMF_DataStream_h