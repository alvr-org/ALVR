/*
* Copyright 2017-2018 NVIDIA Corporation.  All rights reserved.
*
* Please refer to the NVIDIA end user license agreement (EULA) associated
* with this source code for terms and conditions that govern your use of
* this software. Any use, reproduction, disclosure, or distribution of
* this software and related documentation outside the terms of the EULA
* is strictly prohibited.
*
*/

#pragma once

#include <vector>
#include <stdint.h>
#include <mutex>
#include <cuda.h>
#include "NvEncoder.h"


/**
*  @brief Encoder for CUDA device memory.
*/
class NvEncoderCuda : public NvEncoder
{
public:
    NvEncoderCuda(CUcontext cuContext, uint32_t nWidth, uint32_t nHeight, NV_ENC_BUFFER_FORMAT eBufferFormat,
        uint32_t nExtraOutputDelay = 3, bool bMotionEstimationOnly = false);
    virtual ~NvEncoderCuda();

    /**
    *  @brief This is a static function to copy input data from host memory to device memory.
    *  This function assumes YUV plane is a single contiguous memory segment.
    */
    static void CopyToDeviceFrame(CUcontext device,
        void* pSrcFrame,
        uint32_t nSrcPitch,
        CUdeviceptr pDstFrame,
        uint32_t dstPitch,
        int width,
        int height,
        CUmemorytype srcMemoryType,
        NV_ENC_BUFFER_FORMAT pixelFormat,
        const uint32_t dstChromaOffsets[],
        uint32_t numChromaPlanes,
        bool bUnAlignedDeviceCopy = false);


    /**
    *  @brief This is a static function to copy input data from host memory to device memory.
    *  Application must pass a seperate device pointer for each YUV plane.
    */
    static void CopyToDeviceFrame(CUcontext device,
        void* pSrcFrame,
        uint32_t nSrcPitch,
        CUdeviceptr pDstFrame,
        uint32_t dstPitch,
        int width,
        int height,
        CUmemorytype srcMemoryType,
        NV_ENC_BUFFER_FORMAT pixelFormat,
        CUdeviceptr dstChromaPtr[],
        uint32_t dstChromaPitch,
        uint32_t numChromaPlanes,
        bool bUnAlignedDeviceCopy = false);
private:
    /**
    *  @brief This function is used to allocate input buffers for encoding.
    *  This function is an override of virtual function NvEncoder::AllocateInputBuffers().
    */
    virtual void AllocateInputBuffers(int32_t numInputBuffers) override;

    /**
    *  @brief This function is used to release the input buffers allocated for encoding.
    *  This function is an override of virtual function NvEncoder::ReleaseInputBuffers().
    */
    virtual void ReleaseInputBuffers() override;
private:
    /**
    *  @brief This is a private function to release CUDA device memory used for encoding.
    */
    void ReleaseCudaResources();
private:
    size_t m_cudaPitch = 0;
    CUcontext m_cuContext;
};
