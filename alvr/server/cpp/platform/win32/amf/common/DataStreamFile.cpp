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

#include "TraceAdapter.h"
#include "DataStreamFile.h"

#pragma warning(disable: 4996)
#if defined(_WIN32)
#include <io.h>
#endif

#include <fcntl.h>
#include <sys/types.h>
#include <sys/stat.h>

#if defined(_WIN32)
    #define amf_close _close
    #define amf_read _read
    #define amf_write _write
    #define amf_seek64 _lseeki64
#elif defined(__linux)// Linux
    #include <unistd.h>
    #define amf_close        close
    #define amf_read         read
    #define amf_write        write
    #define amf_seek64       lseek64
#elif defined(__APPLE__)
    #include <unistd.h>
    #define amf_close        close
    #define amf_read         read
    #define amf_write        write
    #define amf_seek64       lseek
#endif

using namespace amf;

#define AMF_FACILITY    L"AMFDataStreamFileImpl"

#define AMF_FILE_PROTOCOL L"file"

//-------------------------------------------------------------------------------------------------
AMFDataStreamFileImpl::AMFDataStreamFileImpl()
    : m_iFileDescriptor(-1), m_Path()
{}
//-------------------------------------------------------------------------------------------------
AMFDataStreamFileImpl::~AMFDataStreamFileImpl()
{
    Close();
}
//-------------------------------------------------------------------------------------------------
AMF_RESULT AMF_STD_CALL AMFDataStreamFileImpl::Close()
{
    AMF_RESULT err = AMF_OK;
    if(m_iFileDescriptor != -1)
    {
        const int status = amf_close(m_iFileDescriptor);
        if(status != 0)
        {
            err = AMF_FAIL;
        }
        m_iFileDescriptor = -1;
    }
    return err;
}
//-------------------------------------------------------------------------------------------------
AMF_RESULT AMF_STD_CALL AMFDataStreamFileImpl::Read(void* pData, amf_size iSize, amf_size* pRead)
{
    AMF_RETURN_IF_FALSE(m_iFileDescriptor != -1, AMF_FILE_NOT_OPEN, L"Read() - File not open");
    AMF_RESULT err = AMF_OK;

    int ready = amf_read(m_iFileDescriptor, pData, (amf_uint)iSize);

    if(pRead != NULL)
    {
        *pRead = ready;
    }
    if(ready == 0)  // eof
    {
        err = AMF_EOF;
    }
    else if(ready == -1)
    {
        err = AMF_FAIL;
    }
    return err;
}
//-------------------------------------------------------------------------------------------------
AMF_RESULT AMF_STD_CALL AMFDataStreamFileImpl::Write(const void* pData, amf_size iSize, amf_size* pWritten)
{
    AMF_RETURN_IF_FALSE(m_iFileDescriptor != -1, AMF_FILE_NOT_OPEN, L"Write() - File not Open");
    AMF_RESULT err = AMF_OK;
    amf_uint32 written = amf_write(m_iFileDescriptor, pData, (amf_uint)iSize);

    if(pWritten != NULL)
    {
        *pWritten = written;
    }
    if(written != iSize) // check errors
    {
        err = AMF_FAIL;
    }
    return err;
}
//-------------------------------------------------------------------------------------------------
AMF_RESULT AMF_STD_CALL AMFDataStreamFileImpl::Seek(AMF_SEEK_ORIGIN eOrigin, amf_int64 iPosition, amf_int64* pNewPosition)
{
    AMF_RETURN_IF_FALSE(m_iFileDescriptor != -1, AMF_FILE_NOT_OPEN, L"Seek() - File not Open");

    int org = 0;

    switch(eOrigin)
    {
    case AMF_SEEK_BEGIN:
        org = SEEK_SET;
        break;

    case AMF_SEEK_CURRENT:
        org = SEEK_CUR;
        break;

    case AMF_SEEK_END:
        org = SEEK_END;
        break;
    }
    amf_int64 new_pos = 0;

    new_pos = amf_seek64(m_iFileDescriptor, iPosition, org);
    if(new_pos == -1L) // check errors
    {
        return AMF_FAIL;
    }
    if(pNewPosition != NULL)
    {
        *pNewPosition = new_pos;
    }
    return AMF_OK;
}
//-------------------------------------------------------------------------------------------------
AMF_RESULT AMF_STD_CALL AMFDataStreamFileImpl::GetPosition(amf_int64* pPosition)
{
    AMF_RETURN_IF_FALSE(pPosition != NULL, AMF_INVALID_POINTER);
    AMF_RETURN_IF_FALSE(m_iFileDescriptor != -1, AMF_FILE_NOT_OPEN, L"GetPosition() - File not Open");
    *pPosition = amf_seek64(m_iFileDescriptor, 0, SEEK_CUR);
    if(*pPosition == -1L)
    {
        return AMF_FAIL;
    }
    return AMF_OK;
}
//-------------------------------------------------------------------------------------------------
AMF_RESULT AMF_STD_CALL AMFDataStreamFileImpl::GetSize(amf_int64* pSize)
{
    AMF_RETURN_IF_FALSE(pSize != NULL, AMF_INVALID_POINTER);
    AMF_RETURN_IF_FALSE(m_iFileDescriptor != -1, AMF_FILE_NOT_OPEN, L"GetSize() - File not open");

    amf_int64 cur_pos = amf_seek64(m_iFileDescriptor, 0, SEEK_CUR);
    *pSize = amf_seek64(m_iFileDescriptor, 0, SEEK_END);
    amf_seek64(m_iFileDescriptor, cur_pos, SEEK_SET);
    return AMF_OK;
}
//-------------------------------------------------------------------------------------------------
bool AMF_STD_CALL AMFDataStreamFileImpl::IsSeekable()
{
    return true;
}
//-------------------------------------------------------------------------------------------------
AMF_RESULT AMF_STD_CALL AMFDataStreamFileImpl::Open(const wchar_t* pFilePath, AMF_STREAM_OPEN eOpenType, AMF_FILE_SHARE eShareType)
{
    if(m_iFileDescriptor != -1)
    {
        Close();
    }
    AMF_RETURN_IF_FALSE(pFilePath != NULL, AMF_INVALID_ARG);

    m_Path = pFilePath;


#if defined(_WIN32)
    int access = _O_BINARY;
#else
    int access = 0;
#endif

    switch(eOpenType)
    {
    case AMFSO_READ:
        access |= O_RDONLY;
        break;

    case AMFSO_WRITE:
        access |= O_CREAT | O_TRUNC | O_WRONLY;
        break;

    case AMFSO_READ_WRITE:
        access |= O_CREAT | O_TRUNC | O_RDWR;
        break;

    case AMFSO_APPEND:
        access |= O_CREAT | O_APPEND | O_RDWR;
        break;
    }

#ifdef _WIN32
    int shflag = 0;
    switch(eShareType)
    {
    case AMFFS_EXCLUSIVE:
        shflag = _SH_DENYRW;
        break;

    case AMFFS_SHARE_READ:
        shflag = _SH_DENYWR;
        break;

    case AMFFS_SHARE_WRITE:
        shflag = _SH_DENYRD;
        break;

    case AMFFS_SHARE_READ_WRITE:
        shflag = _SH_DENYNO;
        break;
    }
#endif

#ifdef O_BINARY
    access |= O_BINARY;
#endif

#ifdef _WIN32
    m_iFileDescriptor = _wsopen(m_Path.c_str(), access, shflag, 0666);
#else
    amf_string str = amf_from_unicode_to_utf8(m_Path);
    m_iFileDescriptor = open(str.c_str(), access, 0666);
#endif

    if(m_iFileDescriptor == -1)
    {
        return AMF_FAIL;
    }
    return AMF_OK;
}
//-------------------------------------------------------------------------------------------------
