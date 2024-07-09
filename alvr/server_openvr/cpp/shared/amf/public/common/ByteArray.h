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

#ifndef AMF_ByteArray_h
#define AMF_ByteArray_h


#pragma once
#include "../include/core/Platform.h"
#define    INIT_ARRAY_SIZE 1024
#define    ARRAY_MAX_SIZE (1LL << 60LL) // extremely large maximum size
//------------------------------------------------------------------------
class AMFByteArray
{
protected:
    amf_uint8        *m_pData;
    amf_size         m_iSize;
    amf_size         m_iMaxSize;
public:
    AMFByteArray() : m_pData(0), m_iSize(0), m_iMaxSize(0)
    {
    }
    AMFByteArray(const AMFByteArray &other) : m_pData(0), m_iSize(0), m_iMaxSize(0)
    {
        *this = other;
    }
    AMFByteArray(amf_size num) : m_pData(0), m_iSize(0), m_iMaxSize(0)
    {
        SetSize(num);
    }
    virtual ~AMFByteArray()
    {
        if (m_pData != 0)
        {
            delete[] m_pData;
        }
    }
    void  SetSize(amf_size num)
    {
        if (num == m_iSize)
        {
            return;
        }
        if (num < m_iSize)
        {
            memset(m_pData + num, 0, m_iMaxSize - num);
        }
        else if (num > m_iMaxSize)
        {
            // This is done to prevent the following error from surfacing
            // for the pNewData allocation on some compilers:
            //     -Werror=alloc-size-larger-than=
            amf_size newSize = (num / INIT_ARRAY_SIZE) * INIT_ARRAY_SIZE + INIT_ARRAY_SIZE;
            if (newSize > ARRAY_MAX_SIZE)
            {
                return;
            }
            m_iMaxSize = newSize;

            amf_uint8 *pNewData = new amf_uint8[m_iMaxSize];
            memset(pNewData, 0, m_iMaxSize);
            if (m_pData != NULL)
            {
                memcpy(pNewData, m_pData, m_iSize);
                delete[] m_pData;
            }
            m_pData = pNewData;
        }
        m_iSize = num;
    }
    void Copy(const AMFByteArray &old)
    {
        if (m_iMaxSize < old.m_iSize)
        {
            m_iMaxSize = old.m_iMaxSize;
            if (m_pData != NULL)
            {
                delete[] m_pData;
            }
            m_pData = new amf_uint8[m_iMaxSize];
            memset(m_pData, 0, m_iMaxSize);
        }
        memcpy(m_pData, old.m_pData, old.m_iSize);
        m_iSize = old.m_iSize;
    }
    amf_uint8    operator[] (amf_size iPos) const
    {
        return m_pData[iPos];
    }
    amf_uint8&    operator[] (amf_size iPos)
    {
        return m_pData[iPos];
    }
    AMFByteArray&    operator=(const AMFByteArray &other)
    {
        SetSize(other.GetSize());
        if (GetSize() > 0)
        {
            memcpy(GetData(), other.GetData(), GetSize());
        }
        return *this;
    }
    amf_uint8 *GetData() const { return m_pData; }
    amf_size GetSize() const { return m_iSize; }
};
#endif // AMF_ByteArray_h