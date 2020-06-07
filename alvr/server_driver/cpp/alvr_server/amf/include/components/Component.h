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

/**
 ***************************************************************************************************
 * @file  Component.h
 * @brief AMFComponent interface declaration
 ***************************************************************************************************
 */
#ifndef __AMFComponent_h__
#define __AMFComponent_h__
#pragma once

#include "../core/Data.h"
#include "../core/PropertyStorageEx.h"
#include "../core/Surface.h"
#include "../core/Context.h"
#include "ComponentCaps.h"

namespace amf
{
    //----------------------------------------------------------------------------------------------
    // AMFDataAllocatorCB interface
    //----------------------------------------------------------------------------------------------
    class AMF_NO_VTABLE AMFDataAllocatorCB : public AMFInterface
    {
    public:
        AMF_DECLARE_IID(0x4bf46198, 0x8b7b, 0x49d0, 0xaa, 0x72, 0x48, 0xd4, 0x7, 0xce, 0x24, 0xc5 )

        virtual AMF_RESULT AMF_STD_CALL AllocBuffer(AMF_MEMORY_TYPE type, amf_size size, AMFBuffer** ppBuffer) = 0;
        virtual AMF_RESULT AMF_STD_CALL AllocSurface(AMF_MEMORY_TYPE type, AMF_SURFACE_FORMAT format,
            amf_int32 width, amf_int32 height, amf_int32 hPitch, amf_int32 vPitch, AMFSurface** ppSurface) = 0;
    };
    //----------------------------------------------------------------------------------------------
    // smart pointer
    //----------------------------------------------------------------------------------------------
    typedef AMFInterfacePtr_T<AMFDataAllocatorCB> AMFDataAllocatorCBPtr;
    //----------------------------------------------------------------------------------------------
    class AMF_NO_VTABLE AMFComponentOptimizationCallback
    {
    public:
        virtual AMF_RESULT AMF_STD_CALL OnComponentOptimizationProgress(amf_uint percent) = 0;
    };

    //----------------------------------------------------------------------------------------------
    // AMFComponent interface
    //----------------------------------------------------------------------------------------------
    class AMF_NO_VTABLE AMFComponent : public AMFPropertyStorageEx
    {
    public:
        AMF_DECLARE_IID(0x8b51e5e4, 0x455d, 0x4034, 0xa7, 0x46, 0xde, 0x1b, 0xed, 0xc3, 0xc4, 0x6)

        virtual AMF_RESULT  AMF_STD_CALL Init(AMF_SURFACE_FORMAT format,amf_int32 width,amf_int32 height) = 0;
        virtual AMF_RESULT  AMF_STD_CALL ReInit(amf_int32 width,amf_int32 height) = 0;
        virtual AMF_RESULT  AMF_STD_CALL Terminate() = 0;
        virtual AMF_RESULT  AMF_STD_CALL Drain() = 0;
        virtual AMF_RESULT  AMF_STD_CALL Flush() = 0;

        virtual AMF_RESULT  AMF_STD_CALL SubmitInput(AMFData* pData) = 0;
        virtual AMF_RESULT  AMF_STD_CALL QueryOutput(AMFData** ppData) = 0;
        virtual AMFContext* AMF_STD_CALL GetContext() = 0;
        virtual AMF_RESULT  AMF_STD_CALL SetOutputDataAllocatorCB(AMFDataAllocatorCB* callback) = 0;

        virtual AMF_RESULT AMF_STD_CALL GetCaps(AMFCaps** ppCaps) = 0;
        virtual AMF_RESULT  AMF_STD_CALL Optimize(AMFComponentOptimizationCallback* pCallback) = 0;
    };
    //----------------------------------------------------------------------------------------------
    // smart pointer
    //----------------------------------------------------------------------------------------------
    typedef AMFInterfacePtr_T<AMFComponent> AMFComponentPtr;
    //----------------------------------------------------------------------------------------------
    // AMFInput interface
    //----------------------------------------------------------------------------------------------
    class AMF_NO_VTABLE AMFInput : public AMFPropertyStorageEx
    {
    public:
        AMF_DECLARE_IID(0x1181eee7, 0x95f2, 0x434a, 0x9b, 0x96, 0xea, 0x55, 0xa, 0xa7, 0x84, 0x89)

        virtual AMF_RESULT  AMF_STD_CALL SubmitInput(AMFData* pData) = 0;

    };
    //----------------------------------------------------------------------------------------------
    // smart pointer
    //----------------------------------------------------------------------------------------------
    typedef AMFInterfacePtr_T<AMFInput> AMFInputPtr;

    //----------------------------------------------------------------------------------------------
    // AMFOutput interface
    //----------------------------------------------------------------------------------------------
    class AMF_NO_VTABLE AMFOutput : public AMFPropertyStorageEx
    {
    public:
        AMF_DECLARE_IID(0x86a8a037, 0x912c, 0x4698, 0xb0, 0x46, 0x7, 0x5a, 0x1f, 0xac, 0x6b, 0x97);

        virtual AMF_RESULT  AMF_STD_CALL QueryOutput(AMFData** ppData) = 0;
    };
    //----------------------------------------------------------------------------------------------
    // smart pointer
    //----------------------------------------------------------------------------------------------
    typedef AMFInterfacePtr_T<AMFOutput> AMFOutputPtr;

    //----------------------------------------------------------------------------------------------
    // AMFComponent interface
    //----------------------------------------------------------------------------------------------
    class AMF_NO_VTABLE AMFComponentEx : public AMFComponent
    {
    public:
        AMF_DECLARE_IID(0xfda792af, 0x8712, 0x44df, 0x8e, 0xa0, 0xdf, 0xfa, 0xad, 0x2c, 0x80, 0x93)

        virtual amf_int32   AMF_STD_CALL GetInputCount() = 0;
        virtual amf_int32   AMF_STD_CALL GetOutputCount() = 0;

        virtual AMF_RESULT  AMF_STD_CALL GetInput(amf_int32 index, AMFInput** ppInput) = 0;
        virtual AMF_RESULT  AMF_STD_CALL GetOutput(amf_int32 index, AMFOutput** ppOutput) = 0;

    };
    //----------------------------------------------------------------------------------------------
    // smart pointer
    //----------------------------------------------------------------------------------------------
    typedef AMFInterfacePtr_T<AMFComponentEx> AMFComponentExPtr;





} // namespace

#endif //#ifndef __AMFComponent_h__
