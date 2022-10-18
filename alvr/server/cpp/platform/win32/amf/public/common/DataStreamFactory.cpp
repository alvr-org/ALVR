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

#include "DataStream.h"
#include "DataStreamMemory.h"
#include "DataStreamFile.h"
#include "TraceAdapter.h"
#include <string>

using namespace amf;


//-------------------------------------------------------------------------------------------------
AMF_RESULT AMF_STD_CALL amf::AMFDataStream::OpenDataStream(const wchar_t* pFileUrl, AMF_STREAM_OPEN eOpenType, AMF_FILE_SHARE eShareType, AMFDataStream** str)
{
    AMF_RETURN_IF_FALSE(pFileUrl != NULL, AMF_INVALID_ARG);

    AMF_RESULT res = AMF_NOT_SUPPORTED;
    std::wstring url(pFileUrl);

    std::wstring protocol;
    std::wstring path;
    std::wstring::size_type found_pos = url.find(L"://", 0);
    if(found_pos != std::wstring::npos)
    {
        protocol = url.substr(0, found_pos);
        path = url.substr(found_pos + 3);
    }
    else
    {
        protocol = L"file";
        path = url;
    }
    AMFDataStreamPtr ptr = NULL;
    if(protocol == L"file")
    {
        ptr = new AMFDataStreamFileImpl;
        res = AMF_OK;
    }
    if(protocol == L"memory")
    {
        ptr = new AMFDataStreamMemoryImpl();
        res = AMF_OK;
    }
    if( res == AMF_OK )
    {
        res = ptr->Open(path.c_str(), eOpenType, eShareType);
        if( res != AMF_OK )
        {
            return res;
        }
        *str = ptr.Detach();
        return AMF_OK;
    }
    return res;
}
//-------------------------------------------------------------------------------------------------
