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

#ifndef __AMFSurface_h__
#define __AMFSurface_h__
#pragma once

#include "Data.h"
#include "Plane.h"

#pragma warning( push )
#pragma warning(disable : 4263)
#pragma warning(disable : 4264)

namespace amf
{
    enum AMF_SURFACE_FORMAT
    {
        AMF_SURFACE_UNKNOWN     = 0,
        AMF_SURFACE_NV12,               ///< 1 - planar Y width x height + packed UV width/2 x height/2 - 8 bit per component
        AMF_SURFACE_YV12,               ///< 2 - planar Y width x height + V width/2 x height/2 + U width/2 x height/2 - 8 bit per component
        AMF_SURFACE_BGRA,               ///< 3 - packed - 8 bit per component
        AMF_SURFACE_ARGB,               ///< 4 - packed - 8 bit per component
        AMF_SURFACE_RGBA,               ///< 5 - packed - 8 bit per component
        AMF_SURFACE_GRAY8,              ///< 6 - single component - 8 bit
        AMF_SURFACE_YUV420P,            ///< 7 - planar Y width x height + U width/2 x height/2 + V width/2 x height/2 - 8 bit per component
        AMF_SURFACE_U8V8,               ///< 8 - double component - 8 bit per component
        AMF_SURFACE_YUY2,               ///< 9 - YUY2: Byte 0=8-bit Y'0; Byte 1=8-bit Cb; Byte 2=8-bit Y'1; Byte 3=8-bit Cr
        AMF_SURFACE_P010,               ///< 10- planar Y width x height + packed UV width/2 x height/2 - 10 bit per component (16 allocated, upper 10 bits are used)
        AMF_SURFACE_RGBA_F16,           ///< 11 - packed - 16 bit per component float

        AMF_SURFACE_FIRST = AMF_SURFACE_NV12,
        AMF_SURFACE_LAST = AMF_SURFACE_RGBA_F16
    };

    //----------------------------------------------------------------------------------------------
    // frame type
    //----------------------------------------------------------------------------------------------
    enum AMF_FRAME_TYPE
    {
        // flags
        AMF_FRAME_STEREO_FLAG                           = 0x10000000,
        AMF_FRAME_LEFT_FLAG                             = AMF_FRAME_STEREO_FLAG | 0x20000000,
        AMF_FRAME_RIGHT_FLAG                            = AMF_FRAME_STEREO_FLAG | 0x40000000,
        AMF_FRAME_BOTH_FLAG                             = AMF_FRAME_LEFT_FLAG | AMF_FRAME_RIGHT_FLAG,
        AMF_FRAME_INTERLEAVED_FLAG                      = 0x01000000,
        AMF_FRAME_FIELD_FLAG                            = 0x02000000,
        AMF_FRAME_EVEN_FLAG                             = 0x04000000,
        AMF_FRAME_ODD_FLAG                              = 0x08000000,

        // values
        AMF_FRAME_UNKNOWN                               =-1,
        AMF_FRAME_PROGRESSIVE                           = 0,

        AMF_FRAME_INTERLEAVED_EVEN_FIRST                = AMF_FRAME_INTERLEAVED_FLAG | AMF_FRAME_EVEN_FLAG,
        AMF_FRAME_INTERLEAVED_ODD_FIRST                 = AMF_FRAME_INTERLEAVED_FLAG | AMF_FRAME_ODD_FLAG,
        AMF_FRAME_FIELD_SINGLE_EVEN                     = AMF_FRAME_FIELD_FLAG | AMF_FRAME_EVEN_FLAG,
        AMF_FRAME_FIELD_SINGLE_ODD                      = AMF_FRAME_FIELD_FLAG | AMF_FRAME_ODD_FLAG,

        AMF_FRAME_STEREO_LEFT                           = AMF_FRAME_LEFT_FLAG,
        AMF_FRAME_STEREO_RIGHT                          = AMF_FRAME_RIGHT_FLAG,
        AMF_FRAME_STEREO_BOTH                           = AMF_FRAME_BOTH_FLAG,

        AMF_FRAME_INTERLEAVED_EVEN_FIRST_STEREO_LEFT    = AMF_FRAME_INTERLEAVED_FLAG | AMF_FRAME_EVEN_FLAG | AMF_FRAME_LEFT_FLAG,
        AMF_FRAME_INTERLEAVED_EVEN_FIRST_STEREO_RIGHT   = AMF_FRAME_INTERLEAVED_FLAG | AMF_FRAME_EVEN_FLAG | AMF_FRAME_RIGHT_FLAG,
        AMF_FRAME_INTERLEAVED_EVEN_FIRST_STEREO_BOTH    = AMF_FRAME_INTERLEAVED_FLAG | AMF_FRAME_EVEN_FLAG | AMF_FRAME_BOTH_FLAG,

        AMF_FRAME_INTERLEAVED_ODD_FIRST_STEREO_LEFT     = AMF_FRAME_INTERLEAVED_FLAG | AMF_FRAME_ODD_FLAG | AMF_FRAME_LEFT_FLAG,
        AMF_FRAME_INTERLEAVED_ODD_FIRST_STEREO_RIGHT    = AMF_FRAME_INTERLEAVED_FLAG | AMF_FRAME_ODD_FLAG | AMF_FRAME_RIGHT_FLAG,
        AMF_FRAME_INTERLEAVED_ODD_FIRST_STEREO_BOTH     = AMF_FRAME_INTERLEAVED_FLAG | AMF_FRAME_ODD_FLAG | AMF_FRAME_BOTH_FLAG,
    };

    //----------------------------------------------------------------------------------------------
    // AMFSurfaceObserver interface - callback; is called before internal release resources.
    //----------------------------------------------------------------------------------------------
    class AMFSurface;
    class AMF_NO_VTABLE AMFSurfaceObserver
    {
    public:
        virtual void AMF_STD_CALL OnSurfaceDataRelease(AMFSurface* pSurface) = 0;
    };
    //----------------------------------------------------------------------------------------------
    // AMFSurface interface
    //----------------------------------------------------------------------------------------------
    class AMF_NO_VTABLE AMFSurface : public AMFData
    {
    public:
        AMF_DECLARE_IID(0x3075dbe3, 0x8718, 0x4cfa, 0x86, 0xfb, 0x21, 0x14, 0xc0, 0xa5, 0xa4, 0x51)

        virtual AMF_SURFACE_FORMAT  AMF_STD_CALL GetFormat() = 0;

        // do not store planes outside. should be used together with Surface
        virtual amf_size            AMF_STD_CALL GetPlanesCount() = 0;
        virtual AMFPlane*           AMF_STD_CALL GetPlaneAt(amf_size index) = 0;
        virtual AMFPlane*           AMF_STD_CALL GetPlane(AMF_PLANE_TYPE type) = 0;

        virtual AMF_FRAME_TYPE      AMF_STD_CALL GetFrameType() = 0;
        virtual void                AMF_STD_CALL SetFrameType(AMF_FRAME_TYPE type) = 0;

        virtual AMF_RESULT          AMF_STD_CALL SetCrop(amf_int32 x,amf_int32 y, amf_int32 width, amf_int32 height) = 0;
        virtual AMF_RESULT          AMF_STD_CALL CopySurfaceRegion(AMFSurface* pDest, amf_int32 dstX, amf_int32 dstY, amf_int32 srcX, amf_int32 srcY, amf_int32 width, amf_int32 height) = 0;

        // Observer management
        virtual void                AMF_STD_CALL AddObserver(AMFSurfaceObserver* pObserver) = 0;
        virtual void                AMF_STD_CALL RemoveObserver(AMFSurfaceObserver* pObserver) = 0;
    };
    //----------------------------------------------------------------------------------------------
    // smart pointer
    //----------------------------------------------------------------------------------------------
    typedef AMFInterfacePtr_T<AMFSurface> AMFSurfacePtr;
    //----------------------------------------------------------------------------------------------
}
#pragma warning( pop )

#endif //#ifndef __AMFSurface_h__
