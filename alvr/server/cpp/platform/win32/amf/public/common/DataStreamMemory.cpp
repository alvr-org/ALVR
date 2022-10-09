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

#include "Thread.h"
#include "TraceAdapter.h"
#include "DataStreamMemory.h"

using namespace amf;

#define AMF_FACILITY    L"AMFDataStreamMemoryImpl"

//-------------------------------------------------------------------------------------------------
AMFDataStreamMemoryImpl::AMFDataStreamMemoryImpl()
    : m_pMemory(NULL),
    m_uiMemorySize(0),
    m_uiAllocatedSize(0),
    m_pos(0)
{}
//-------------------------------------------------------------------------------------------------
AMFDataStreamMemoryImpl::~AMFDataStreamMemoryImpl()
{
    Close();
}
//-------------------------------------------------------------------------------------------------
// interface
//-------------------------------------------------------------------------------------------------
AMF_RESULT AMF_STD_CALL AMFDataStreamMemoryImpl::Close()
{
    if(m_pMemory != NULL)
    {
        amf_virtual_free(m_pMemory);
    }
    m_pMemory = NULL,
    m_uiMemorySize = 0,
    m_uiAllocatedSize = 0,
    m_pos = 0;
    return AMF_OK;
}
//-------------------------------------------------------------------------------------------------
AMF_RESULT AMFDataStreamMemoryImpl::Realloc(amf_size iSize)
{
    if(iSize > m_uiMemorySize)
    {
        amf_uint8* pNewMemory = (amf_uint8*)amf_virtual_alloc(iSize);
        if(pNewMemory == NULL)
        {
            return AMF_OUT_OF_MEMORY;
        }
        m_uiAllocatedSize = iSize;
        if(m_pMemory != NULL)
        {
            memcpy(pNewMemory, m_pMemory, m_uiMemorySize);
            amf_virtual_free(m_pMemory);
        }

        m_pMemory = pNewMemory;
    }
    m_uiMemorySize = iSize;
    if(m_pos > m_uiMemorySize)
    {
        m_pos = m_uiMemorySize;
    }
    return AMF_OK;
}
//-------------------------------------------------------------------------------------------------
AMF_RESULT AMF_STD_CALL AMFDataStreamMemoryImpl::Read(void* pData, amf_size iSize, amf_size* pRead)
{
    AMF_RETURN_IF_FALSE(pData != NULL, AMF_INVALID_POINTER, L"Read() - pData==NULL");
    AMF_RETURN_IF_FALSE(m_pMemory != NULL, AMF_NOT_INITIALIZED, L"Read() - Stream is not allocated");

    amf_size toRead = AMF_MIN(iSize, m_uiMemorySize - m_pos);
    memcpy(pData, m_pMemory + m_pos, toRead);
    m_pos += toRead;
    if(pRead != NULL)
    {
        *pRead = toRead;
    }
    return AMF_OK;
}
//-------------------------------------------------------------------------------------------------
AMF_RESULT AMF_STD_CALL AMFDataStreamMemoryImpl::Write(const void* pData, amf_size iSize, amf_size* pWritten)
{
    AMF_RETURN_IF_FALSE(pData != NULL, AMF_INVALID_POINTER, L"Write() - pData==NULL");
    AMF_RETURN_IF_FAILED(Realloc(m_pos + iSize), L"Write() - Stream is not allocated");

    amf_size toWrite = AMF_MIN(iSize, m_uiMemorySize - m_pos);
    memcpy(m_pMemory + m_pos, pData, toWrite);
    m_pos += toWrite;
    if(pWritten != NULL)
    {
        *pWritten = toWrite;
    }
    return AMF_OK;
}
//-------------------------------------------------------------------------------------------------
AMF_RESULT AMF_STD_CALL AMFDataStreamMemoryImpl::Seek(AMF_SEEK_ORIGIN eOrigin, amf_int64 iPosition, amf_int64* pNewPosition)
{
    switch(eOrigin)
    {
    case AMF_SEEK_BEGIN:
        m_pos = (amf_size)iPosition;
        break;

    case AMF_SEEK_CURRENT:
        m_pos += (amf_size)iPosition;
        break;

    case AMF_SEEK_END:
        m_pos = m_uiMemorySize - (amf_size)iPosition;
        break;
    }

    if(m_pos > m_uiMemorySize)
    {
        m_pos = m_uiMemorySize;
    }
    if(pNewPosition != NULL)
    {
        *pNewPosition = m_pos;
    }
    return AMF_OK;
}
//-------------------------------------------------------------------------------------------------
AMF_RESULT AMF_STD_CALL AMFDataStreamMemoryImpl::GetPosition(amf_int64* pPosition)
{
    AMF_RETURN_IF_FALSE(pPosition != NULL, AMF_INVALID_POINTER, L"GetPosition() - pPosition==NULL");
    *pPosition = m_pos;
    return AMF_OK;
}
//-------------------------------------------------------------------------------------------------
AMF_RESULT AMF_STD_CALL AMFDataStreamMemoryImpl::GetSize(amf_int64* pSize)
{
    AMF_RETURN_IF_FALSE(pSize != NULL, AMF_INVALID_POINTER, L"GetPosition() - pSize==NULL");
    *pSize = m_uiMemorySize;
    return AMF_OK;
}
//-------------------------------------------------------------------------------------------------
bool AMF_STD_CALL AMFDataStreamMemoryImpl::IsSeekable()
{
    return true;
}
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
