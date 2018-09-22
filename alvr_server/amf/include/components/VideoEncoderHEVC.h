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

#ifndef __AMFVideoEncoderHW_HEVC_h__
#define __AMFVideoEncoderHW_HEVC_h__
#pragma once

#include "Component.h"

#define AMFVideoEncoder_HEVC L"AMFVideoEncoderHW_HEVC"

enum AMF_VIDEO_ENCODER_HEVC_USAGE_ENUM
{
    AMF_VIDEO_ENCODER_HEVC_USAGE_TRANSCONDING = 0,
    AMF_VIDEO_ENCODER_HEVC_USAGE_ULTRA_LOW_LATENCY,
    AMF_VIDEO_ENCODER_HEVC_USAGE_LOW_LATENCY,
    AMF_VIDEO_ENCODER_HEVC_USAGE_WEBCAM
};

enum AMF_VIDEO_ENCODER_HEVC_PROFILE_ENUM
{
    AMF_VIDEO_ENCODER_HEVC_PROFILE_MAIN = 1
};

enum AMF_VIDEO_ENCODER_HEVC_TIER_ENUM
{
    AMF_VIDEO_ENCODER_HEVC_TIER_MAIN    = 0,
    AMF_VIDEO_ENCODER_HEVC_TIER_HIGH    = 1
};

enum AMF_VIDEO_ENCODER_LEVEL_ENUM
{
    AMF_LEVEL_1     = 30,
    AMF_LEVEL_2     = 60,
    AMF_LEVEL_2_1   = 63,
    AMF_LEVEL_3     = 90,
    AMF_LEVEL_3_1   = 93,
    AMF_LEVEL_4     = 120,
    AMF_LEVEL_4_1   = 123,
    AMF_LEVEL_5     = 150,
    AMF_LEVEL_5_1   = 153,
    AMF_LEVEL_5_2   = 156,
    AMF_LEVEL_6     = 180,
    AMF_LEVEL_6_1   = 183,
    AMF_LEVEL_6_2   = 186
};

enum AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD_ENUM
{
    AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD_CONSTANT_QP = 0,
    AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD_LATENCY_CONSTRAINED_VBR,
    AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD_PEAK_CONSTRAINED_VBR,
    AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD_CBR
};

enum AMF_VIDEO_ENCODER_HEVC_PICTURE_TYPE_ENUM
{
    AMF_VIDEO_ENCODER_HEVC_PICTURE_TYPE_NONE = 0,
    AMF_VIDEO_ENCODER_HEVC_PICTURE_TYPE_SKIP,
    AMF_VIDEO_ENCODER_HEVC_PICTURE_TYPE_IDR,
    AMF_VIDEO_ENCODER_HEVC_PICTURE_TYPE_I,
    AMF_VIDEO_ENCODER_HEVC_PICTURE_TYPE_P
};

enum AMF_VIDEO_ENCODER_HEVC_OUTPUT_DATA_TYPE_ENUM
{
    AMF_VIDEO_ENCODER_HEVC_OUTPUT_DATA_TYPE_I,
    AMF_VIDEO_ENCODER_HEVC_OUTPUT_DATA_TYPE_P
};

enum AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET_ENUM
{
    AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET_QUALITY    = 0,    
    AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET_BALANCED   = 5,
    AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET_SPEED      = 10
};

enum AMF_VIDEO_ENCODER_HEVC_HEADER_INSERTION_MODE_ENUM
{
    AMF_VIDEO_ENCODER_HEVC_HEADER_INSERTION_MODE_NONE = 0,
    AMF_VIDEO_ENCODER_HEVC_HEADER_INSERTION_MODE_GOP_ALIGNED,
    AMF_VIDEO_ENCODER_HEVC_HEADER_INSERTION_MODE_IDR_ALIGNED
};

enum AMF_VIDEO_ENCODER_HEVC_VBAQ_MODE_ENUM
{
    AMF_VIDEO_ENCODER_HEVC_VBAQ_MODE_NONE           = 0, 
    AMF_VIDEO_ENCODER_HEVC_VBAQ_MODE_AUTO
};



// Static properties - can be set before Init()
#define AMF_VIDEO_ENCODER_HEVC_FRAMESIZE                            L"HevcFrameSize"                // AMFSize; default = 0,0; Frame size

#define AMF_VIDEO_ENCODER_HEVC_USAGE                                L"HevcUsage"                    // amf_int64(AMF_VIDEO_ENCODER_HEVC_USAGE_ENUM); default = N/A; Encoder usage type. fully configures parameter set. 
#define AMF_VIDEO_ENCODER_HEVC_PROFILE                              L"HevcProfile"                  // amf_int64(AMF_VIDEO_ENCODER_HEVC_PROFILE_ENUM) ; default = AMF_VIDEO_ENCODER_HEVC_PROFILE_MAIN;
#define AMF_VIDEO_ENCODER_HEVC_TIER                                 L"HevcTier"                     // amf_int64(AMF_VIDEO_ENCODER_HEVC_TIER_ENUM) ; default = AMF_VIDEO_ENCODER_HEVC_TIER_MAIN;
#define AMF_VIDEO_ENCODER_HEVC_PROFILE_LEVEL                        L"HevcProfileLevel"             // amf_int64 (AMF_VIDEO_ENCODER_LEVEL_ENUM, default depends on HW capabilities); 
#define AMF_VIDEO_ENCODER_HEVC_MAX_LTR_FRAMES                       L"HevcMaxOfLTRFrames"           // amf_int64; default = 0; Max number of LTR frames
#define AMF_VIDEO_ENCODER_HEVC_MAX_NUM_REFRAMES                     L"HevcMaxNumRefFrames"          // amf_int64; default = 1; Maximum number of reference frames
#define AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET                       L"HevcQualityPreset"            // amf_int64(AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET_ENUM); default = depends on USAGE; Quality Preset 
#define AMF_VIDEO_ENCODER_HEVC_EXTRADATA                            L"HevcExtraData"                // AMFInterface* - > AMFBuffer*; SPS/PPS buffer - read-only
#define AMF_VIDEO_ENCODER_HEVC_ASPECT_RATIO                         L"HevcAspectRatio"              // AMFRatio; default = 1, 1

// Picture control properties
#define AMF_VIDEO_ENCODER_HEVC_NUM_GOPS_PER_IDR                     L"HevcGOPSPerIDR"               // amf_int64; default = 60; The frequency to insert IDR as start of a GOP. 0 means no IDR will be inserted.
#define AMF_VIDEO_ENCODER_HEVC_GOP_SIZE                             L"HevcGOPSize"                  // amf_int64; default = 60; GOP Size, in frames
#define AMF_VIDEO_ENCODER_HEVC_DE_BLOCKING_FILTER_DISABLE           L"HevcDeBlockingFilter"         // bool; default = depends on USAGE; De-blocking Filter
#define AMF_VIDEO_ENCODER_HEVC_SLICES_PER_FRAME                     L"HevcSlicesPerFrame"           // amf_int64; default = 1; Number of slices Per Frame 
#define AMF_VIDEO_ENCODER_HEVC_HEADER_INSERTION_MODE                L"HevcHeaderInsertionMode"      // amf_int64(AMF_VIDEO_ENCODER_HEVC_HEADER_INSERTION_MODE_ENUM); default = NONE

// Rate control properties
#define AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD                  L"HevcRateControlMethod"        // amf_int64(AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_MODE_ENUM); default = depends on USAGE; Rate Control Method 
#define AMF_VIDEO_ENCODER_HEVC_FRAMERATE                            L"HevcFrameRate"                // AMFRate; default = depends on usage; Frame Rate 
#define AMF_VIDEO_ENCODER_HEVC_VBV_BUFFER_SIZE                      L"HevcVBVBufferSize"            // amf_int64; default = depends on USAGE; VBV Buffer Size in bits
#define AMF_VIDEO_ENCODER_HEVC_INITIAL_VBV_BUFFER_FULLNESS          L"HevcInitialVBVBufferFullness" // amf_int64; default =  64; Initial VBV Buffer Fullness 0=0% 64=100%
#define AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_PREANALYSIS_ENABLE      L"HevcRateControlPreAnalysisEnable"  // bool; default =  depends on USAGE; enable Pre-analysis assisted rate control 
#define AMF_VIDEO_ENCODER_HEVC_ENABLE_VBAQ                          L"HevcEnableVBAQ"               // amf_int64(AMF_VIDEO_ENCODER_HEVC_VBAQ_MODE_ENUM) default  = AMF_VIDEO_ENCODER_HEVC_VBAQ_MODE_NONE; Enable VBAQ


// Dynamic properties - can be set at any time

// Rate control properties
#define AMF_VIDEO_ENCODER_HEVC_ENFORCE_HRD                          L"HevcEnforceHRD"               // bool; default = depends on USAGE; Enforce HRD
#define AMF_VIDEO_ENCODER_HEVC_FILLER_DATA_ENABLE                   L"HevcFillerDataEnable"         // bool; default = depends on USAGE; Enforce HRD
#define AMF_VIDEO_ENCODER_HEVC_TARGET_BITRATE                       L"HevcTargetBitrate"            // amf_int64; default = depends on USAGE; Target bit rate in bits
#define AMF_VIDEO_ENCODER_HEVC_PEAK_BITRATE                         L"HevcPeakBitrate"              // amf_int64; default = depends on USAGE; Peak bit rate in bits

#define AMF_VIDEO_ENCODER_HEVC_MAX_AU_SIZE                          L"HevcMaxAUSize"                // amf_int64; default = 60; Max AU Size in bits

#define AMF_VIDEO_ENCODER_HEVC_MIN_QP_I                             L"HevcMinQP_I"                  // amf_int64; default = depends on USAGE; Min QP; range = 
#define AMF_VIDEO_ENCODER_HEVC_MAX_QP_I                             L"HevcMaxQP_I"                  // amf_int64; default = depends on USAGE; Max QP; range = 
#define AMF_VIDEO_ENCODER_HEVC_MIN_QP_P                             L"HevcMinQP_P"                  // amf_int64; default = depends on USAGE; Min QP; range = 
#define AMF_VIDEO_ENCODER_HEVC_MAX_QP_P                             L"HevcMaxQP_P"                  // amf_int64; default = depends on USAGE; Max QP; range = 

#define AMF_VIDEO_ENCODER_HEVC_QP_I                                 L"HevcQP_I"                     // amf_int64; default = 26; P-frame QP; range = 0-51
#define AMF_VIDEO_ENCODER_HEVC_QP_P                                 L"HevcQP_P"                     // amf_int64; default = 26; P-frame QP; range = 0-51

#define AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_SKIP_FRAME_ENABLE       L"HevcRateControlSkipFrameEnable" // bool; default =  depends on USAGE; Rate Control Based Frame Skip 


// Motion estimation
#define AMF_VIDEO_ENCODER_HEVC_MOTION_HALF_PIXEL                    L"HevcHalfPixel"                // bool; default= true; Half Pixel 
#define AMF_VIDEO_ENCODER_HEVC_MOTION_QUARTERPIXEL                  L"HevcQuarterPixel"             // bool; default= true; Quarter Pixel

// Per-submittion properties - can be set on input surface interface
#define AMF_VIDEO_ENCODER_HEVC_END_OF_SEQUENCE                      L"HevcEndOfSequence"            // bool; default = false; generate end of sequence
#define AMF_VIDEO_ENCODER_HEVC_FORCE_PICTURE_TYPE                   L"HevcForcePictureType"         // amf_int64(AMF_VIDEO_ENCODER_HEVC_PICTURE_TYPE_ENUM); default = AMF_VIDEO_ENCODER_HEVC_PICTURE_TYPE_NONE; generate particular picture type
#define AMF_VIDEO_ENCODER_HEVC_INSERT_AUD                           L"HevcInsertAUD"                // bool; default = false; insert AUD
#define AMF_VIDEO_ENCODER_HEVC_INSERT_HEADER                        L"HevcInsertHeader"             // bool; default = false; insert header(SPS, PPS, VPS)

#define AMF_VIDEO_ENCODER_HEVC_MARK_CURRENT_WITH_LTR_INDEX          L"HevcMarkCurrentWithLTRIndex"  // amf_int64; default = N/A; Mark current frame with LTR index
#define AMF_VIDEO_ENCODER_HEVC_FORCE_LTR_REFERENCE_BITFIELD         L"HevcForceLTRReferenceBitfield"// amf_int64; default = 0; force LTR bit-field 

// Properties set by encoder on output buffer interface
#define AMF_VIDEO_ENCODER_HEVC_OUTPUT_DATA_TYPE                     L"HevcOutputDataType"           // amf_int64(AMF_VIDEO_ENCODER_HEVC_OUTPUT_DATA_TYPE_ENUM); default = N/A
#define AMF_VIDEO_ENCODER_HEVC_OUTPUT_MARKED_LTR_INDEX              L"HevcMarkedLTRIndex"           // amf_int64; default = -1; Marked LTR index
#define AMF_VIDEO_ENCODER_HEVC_OUTPUT_REFERENCED_LTR_INDEX_BITFIELD L"HevcReferencedLTRIndexBitfield"// amf_int64; default = 0; referenced LTR bit-field 

// HEVC Encoder capabilities - exposed in AMFCaps interface
#define AMF_VIDEO_ENCODER_HEVC_CAP_MAX_BITRATE                      L"HevcMaxBitrate"               // amf_int64; Maximum bit rate in bits
#define AMF_VIDEO_ENCODER_HEVC_CAP_NUM_OF_STREAMS                   L"HevcNumOfStreams"             // amf_int64; maximum number of encode streams supported 
#define AMF_VIDEO_ENCODER_HEVC_CAP_MAX_PROFILE                      L"HevcMaxProfile"               // amf_int64(AMF_VIDEO_ENCODER_HEVC_PROFILE_ENUM)
#define AMF_VIDEO_ENCODER_HEVC_CAP_MAX_TIER                         L"HevcMaxTier"                  // amf_int64(AMF_VIDEO_ENCODER_HEVC_TIER_ENUM) maximum profile tier 
#define AMF_VIDEO_ENCODER_HEVC_CAP_MAX_LEVEL                        L"HevcMaxLevel"                 // amf_int64 maximum profile level
#define AMF_VIDEO_ENCODER_HEVC_CAP_MIN_REFERENCE_FRAMES             L"HevcMinReferenceFrames"       // amf_int64 minimum number of reference frames
#define AMF_VIDEO_ENCODER_HEVC_CAP_MAX_REFERENCE_FRAMES             L"HevcMaxReferenceFrames"       // amf_int64 maximum number of reference frames


#endif //#ifndef __AMFVideoEncoderHW_HEVC_h__
