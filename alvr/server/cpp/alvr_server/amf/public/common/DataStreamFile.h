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

#ifndef AMF_DataStreamFile_h
#define AMF_DataStreamFile_h

#pragma once

#include "DataStream.h"
#include "InterfaceImpl.h"
#include "AMFSTL.h"
#include <string>

namespace amf
{
    class AMFDataStreamFileImpl : public AMFInterfaceImpl<AMFDataStream>
    {
    public:
        AMFDataStreamFileImpl();
        virtual ~AMFDataStreamFileImpl();
        // interface
        virtual AMF_RESULT AMF_STD_CALL Close();
        virtual AMF_RESULT AMF_STD_CALL Read(void* pData, amf_size iSize, amf_size* pRead);
        virtual AMF_RESULT AMF_STD_CALL Write(const void* pData, amf_size iSize, amf_size* pWritten);
        virtual AMF_RESULT AMF_STD_CALL Seek(AMF_SEEK_ORIGIN eOrigin, amf_int64 iPosition, amf_int64* pNewPosition);
        virtual AMF_RESULT AMF_STD_CALL GetPosition(amf_int64* pPosition);
        virtual AMF_RESULT AMF_STD_CALL GetSize(amf_int64* pSize);
        virtual bool       AMF_STD_CALL IsSeekable();

        // local
        // aways pass full URL just in case
        virtual AMF_RESULT AMF_STD_CALL Open(const wchar_t* pFilePath, AMF_STREAM_OPEN eOpenType, AMF_FILE_SHARE eShareType);
    protected:
        int m_iFileDescriptor;
        amf_wstring m_Path;
    };
} //namespace amf
#endif // AMF_DataStreamFile_h