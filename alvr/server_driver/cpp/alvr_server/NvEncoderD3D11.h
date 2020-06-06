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
#include <unordered_map>
#include <d3d11.h>
#include "NvEncoder.h"

class NvEncoderD3D11 : public NvEncoder
{
public:
    NvEncoderD3D11(ID3D11Device* pD3D11Device, uint32_t nWidth, uint32_t nHeight, NV_ENC_BUFFER_FORMAT eBufferFormat, 
        uint32_t nExtraOutputDelay = 3, bool bMotionEstimationOnly = false);
    virtual ~NvEncoderD3D11();
private:
    /**
    *  @brief This function is used to allocate input buffers for encoding.
    *  This function is an override of virtual function NvEncoder::AllocateInputBuffers().
    *  This function creates ID3D11Texture2D textures which is used to accept input data.
    *  To obtain handle to input buffers application must call NvEncoder::GetNextInputFrame()
    */
    virtual void AllocateInputBuffers(int32_t numInputBuffers) override;

    /**
    *  @brief This function is used to release the input buffers allocated for encoding.
    *  This function is an override of virtual function NvEncoder::ReleaseInputBuffers().
    */
    virtual void ReleaseInputBuffers() override;
private:
    /**
    *  @brief This is a private function to release ID3D11Texture2D textures used for encoding.
    */
    void ReleaseD3D11Resources();
private:
    ID3D11Device *m_pD3D11Device = nullptr;
    ID3D11DeviceContext* m_pD3D11DeviceContext = nullptr;
};
