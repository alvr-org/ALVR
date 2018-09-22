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

//-------------------------------------------------------------------------------------------------
// DemuxerFFMPEG  interface declaration
//-------------------------------------------------------------------------------------------------
#ifndef __AMFFileDemuxerFFMPEG_h__
#define __AMFFileDemuxerFFMPEG_h__

#pragma once

#define FFMPEG_DEMUXER L"DemuxerFFMPEG"

enum FFMPEG_DEMUXER_STREAM_TYPE_ENUM
{
    DEMUXER_UNKNOWN =   -1,
    DEMUXER_VIDEO =     0,
    DEMUXER_AUDIO =     1,
    DEMUXER_DATA =      2,
};


// component properties
#define FFMPEG_DEMUXER_PATH                     L"Path"                     // string - the file to open
#define FFMPEG_DEMUXER_START_FRAME              L"StartFrame"               // amf_int64 (default = 0)
#define FFMPEG_DEMUXER_FRAME_COUNT              L"FramesNumber"             // amf_int64 (default = 0)
#define FFMPEG_DEMUXER_DURATION                 L"Duration"                 // amf_int64 (default = 0)
#define FFMPEG_DEMUXER_CHECK_MVC                L"CheckMVC"                 // bool (default = true)
//#define FFMPEG_DEMUXER_SYNC_AV                  L"SyncAV"                   // bool (default = false)
#define FFMPEG_DEMUXER_INDIVIDUAL_STREAM_MODE   L"StreamMode"               // bool (default = true)

//common stream properties
#define FFMPEG_DEMUXER_STREAM_TYPE              L"StreamType"               // amf_int64( FFMPEG_DEMUXER_STREAM_TYPE_ENUM )
#define FFMPEG_DEMUXER_STREAM_ENABLED           L"Enabled"                  // bool( default = false )
#define FFMPEG_DEMUXER_CODEC_ID                 L"CodecID"                  // amf_int64 (default = AV_CODEC_ID_NONE) - FFMPEG codec ID
#define FFMPEG_DEMUXER_BIT_RATE                 L"BitRate"                  // amf_int64 (default = codec->bit_rate)
#define FFMPEG_DEMUXER_EXTRA_DATA               L"ExtraData"                // interface to AMFBuffer - as is from FFMPEG

// video stream properties
#define FFMPEG_DEMUXER_VIDEO_DECODER_ID         L"DecoderID"                // string (default - name of the codec ID - see VideoDecoderUVD.h)
#define FFMPEG_DEMUXER_VIDEO_FRAME_RATE         L"FrameRate"                // AMFRate; default - from file 
#define FFMPEG_DEMUXER_VIDEO_FRAMESIZE          L"FrameSize"                // AMFSize; default = 0,0; Frame size
#define FFMPEG_DEMUXER_VIDEO_SURFACE_FORMAT     L"SurfaceFormat"            // amf_int64( AMF_OUTPUT_FORMATS_ENUM )
#define FFMPEG_DEMUXER_VIDEO_PIXEL_ASPECT_RATIO L"PixelAspectRatio"         // double (default = calculated)

// audio stream properties
#define FFMPEG_DEMUXER_AUDIO_SAMPLE_RATE        L"SampleRate"               // amf_int64 (default = codec->sample_rate)
#define FFMPEG_DEMUXER_AUDIO_CHANNELS           L"Channels"                 // amf_int64 (default = codec->channels)
#define FFMPEG_DEMUXER_AUDIO_SAMPLE_FORMAT      L"SampleFormat"             // amf_int64( AMF_AUDIO_FORMAT )
#define FFMPEG_DEMUXER_AUDIO_CHANNEL_LAYOUT     L"ChannelLayout"            // amf_int64 (default = codec->channel_layout)
#define FFMPEG_DEMUXER_AUDIO_BLOCK_ALIGN        L"BlockAlign"               // amf_int64 (default = codec->block_align)
#define FFMPEG_DEMUXER_AUDIO_FRAME_SIZE         L"FrameSize"                // amf_int64 (default = codec->frame_size)

// buffer properties
#define FFMPEG_DEMUXER_BUFFER_TYPE              L"BufferType"               // amf_int64 ( FFMPEG_DEMUXER_STREAM_TYPE_ENUM )
#define FFMPEG_DEMUXER_BUFFER_STREAM_INDEX      L"BufferStreamIndexType"    // amf_int64 ( stream index )
#endif //#ifndef __AMFFileDemuxerFFMPEG_h__
