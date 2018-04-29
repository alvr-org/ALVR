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
#include <iomanip>
#include <chrono>
#include <sys/stat.h>
#include <assert.h>
#include <stdint.h>
#include <string.h>
#include "Logger.h"
#include <thread>

extern simplelogger::Logger *logger;

#ifdef __cuda_cuda_h__
inline bool check(CUresult e, int iLine, const char *szFile) {
    if (e != CUDA_SUCCESS) {
        const char *szErrName = NULL;
        cuGetErrorName(e, &szErrName);
        LOG(FATAL) << "CUDA driver API error " << szErrName << " at line " << iLine << " in file " << szFile;
        return false;
    }
    return true;
}
#endif

#ifdef __CUDA_RUNTIME_H__
inline bool check(cudaError_t e, int iLine, const char *szFile) {
    if (e != cudaSuccess) {
        LOG(FATAL) << "CUDA runtime API error " << cudaGetErrorName(e) << " at line " << iLine << " in file " << szFile;
        return false;
    }
    return true;
}
#endif

#ifdef _NV_ENCODEAPI_H_
inline bool check(NVENCSTATUS e, int iLine, const char *szFile) {
    const char *aszErrName[] = {
        "NV_ENC_SUCCESS",
        "NV_ENC_ERR_NO_ENCODE_DEVICE",
        "NV_ENC_ERR_UNSUPPORTED_DEVICE",
        "NV_ENC_ERR_INVALID_ENCODERDEVICE",
        "NV_ENC_ERR_INVALID_DEVICE",
        "NV_ENC_ERR_DEVICE_NOT_EXIST",
        "NV_ENC_ERR_INVALID_PTR",
        "NV_ENC_ERR_INVALID_EVENT",
        "NV_ENC_ERR_INVALID_PARAM",
        "NV_ENC_ERR_INVALID_CALL",
        "NV_ENC_ERR_OUT_OF_MEMORY",
        "NV_ENC_ERR_ENCODER_NOT_INITIALIZED",
        "NV_ENC_ERR_UNSUPPORTED_PARAM",
        "NV_ENC_ERR_LOCK_BUSY",
        "NV_ENC_ERR_NOT_ENOUGH_BUFFER",
        "NV_ENC_ERR_INVALID_VERSION",
        "NV_ENC_ERR_MAP_FAILED",
        "NV_ENC_ERR_NEED_MORE_INPUT",
        "NV_ENC_ERR_ENCODER_BUSY",
        "NV_ENC_ERR_EVENT_NOT_REGISTERD",
        "NV_ENC_ERR_GENERIC",
        "NV_ENC_ERR_INCOMPATIBLE_CLIENT_KEY",
        "NV_ENC_ERR_UNIMPLEMENTED",
        "NV_ENC_ERR_RESOURCE_REGISTER_FAILED",
        "NV_ENC_ERR_RESOURCE_NOT_REGISTERED",
        "NV_ENC_ERR_RESOURCE_NOT_MAPPED",
    };
    if (e != NV_ENC_SUCCESS) {
        LOG(FATAL) << "NVENC error " << aszErrName[e] << " at line " << iLine << " in file " << szFile;
        return false;
    }
    return true;
}
#endif

#ifdef _WINERROR_
inline bool check(HRESULT e, int iLine, const char *szFile) {
    if (e != S_OK) {
        LOG(FATAL) << "HRESULT error 0x" << (void *)e << " at line " << iLine << " in file " << szFile;
        return false;
    }
    return true;
}
#endif

#if defined(__gl_h_) || defined(__GL_H__)
inline bool check(GLenum e, int iLine, const char *szFile) {
    if (e != 0) {
        LOG(ERROR) << "GLenum error " << e << " at line " << iLine << " in file " << szFile;
        return false;
    }
    return true;
}
#endif

inline bool check(int e, int iLine, const char *szFile) {
    if (e < 0) {
        LOG(ERROR) << "General error " << e << " at line " << iLine << " in file " << szFile;
        return false;
    }
    return true;
}

#define ck(call) check(call, __LINE__, __FILE__)

class NvThread
{
public:
    NvThread() = default;
    NvThread(const NvThread&) = delete;
    NvThread& operator=(const NvThread& other) = delete;

    NvThread(std::thread&& thread) : t(std::move(thread))
    {

    }

    NvThread(NvThread&& thread) : t(std::move(thread.t))
    {

    }

    NvThread& operator=(NvThread&& other)
    {
        t = std::move(other.t);
        return *this;
    }

    ~NvThread()
    {
        join();
    }

    void join()
    {
        if (t.joinable())
        {
            t.join();
        }
    }
private:
    std::thread t;
};

#ifndef _WIN32
#define _stricmp strcasecmp
#endif

class BufferedFileReader {
public:
    BufferedFileReader(const char *szFileName, bool bPartial = false) {
        struct stat st;

        if (stat(szFileName, &st) != 0) {
            return;
        }
        
        nSize = st.st_size;
        while (nSize) {
            try {
                pBuf = new uint8_t[nSize];
                if (nSize != st.st_size) {
                    LOG(WARNING) << "File is too large - only " << std::setprecision(4) << 100.0 * nSize / (uint32_t)st.st_size << "% is loaded"; 
                }
                break;
            } catch(std::bad_alloc) {
                if (!bPartial) {
                    LOG(ERROR) << "Failed to allocate memory in BufferedReader";
                    return;
                }
                nSize = (uint32_t)(nSize * 0.9);
            }
        }

        std::ifstream fpIn(szFileName, std::ifstream::in | std::ifstream::binary);
        if (!fpIn)
        {
            LOG(ERROR) << "Unable to open input file: " << szFileName;
            return;
        }

        std::streamsize nRead = fpIn.read(reinterpret_cast<char*>(pBuf), nSize).gcount();
        fpIn.close();

        assert(nRead == nSize);
    }
    ~BufferedFileReader() {
        if (pBuf) {
            delete[] pBuf;
        }
    }
    bool GetBuffer(uint8_t **ppBuf, uint32_t *pnSize) {
        if (!pBuf) {
            return false;
        }

        *ppBuf = pBuf;
        *pnSize = nSize;
        return true;
    }

private:
    uint8_t *pBuf = NULL;
    uint32_t nSize = 0;
};

template<typename T>
class YuvConverter {
public:
    YuvConverter(int nWidth, int nHeight) : nWidth(nWidth), nHeight(nHeight) {
        pQuad = new T[nWidth * nHeight / 4];
    }
    ~YuvConverter() {
        delete pQuad;
    }
    void PlanarToUVInterleaved(T *pFrame, int nPitch = 0) {
        if (nPitch == 0) {
            nPitch = nWidth;
        }
        T *puv = pFrame + nPitch * nHeight;
        if (nPitch == nWidth) {
            memcpy(pQuad, puv, nWidth * nHeight / 4 * sizeof(T));
        } else {
            for (int i = 0; i < nHeight / 2; i++) {
                memcpy(pQuad + nWidth / 2 * i, puv + nPitch / 2 * i, nWidth / 2 * sizeof(T));
            }
        }
        T *pv = puv + (nPitch / 2) * (nHeight / 2);
        for (int y = 0; y < nHeight / 2; y++) {
            for (int x = 0; x < nWidth / 2; x++) {
                puv[y * nPitch + x * 2] = pQuad[y * nWidth / 2 + x];
                puv[y * nPitch + x * 2 + 1] = pv[y * nPitch / 2 + x];
            }
        }
    }
    void UVInterleavedToPlanar(T *pFrame, int nPitch = 0) {
        if (nPitch == 0) {
            nPitch = nWidth;
        }
        T *puv = pFrame + nPitch * nHeight, 
            *pu = puv, 
            *pv = puv + nPitch * nHeight / 4;
        for (int y = 0; y < nHeight / 2; y++) {
            for (int x = 0; x < nWidth / 2; x++) {
                pu[y * nPitch / 2 + x] = puv[y * nPitch + x * 2];
                pQuad[y * nWidth / 2 + x] = puv[y * nPitch + x * 2 + 1];
            }
        }
        if (nPitch == nWidth) {
            memcpy(pv, pQuad, nWidth * nHeight / 4 * sizeof(T));
        } else {
            for (int i = 0; i < nHeight / 2; i++) {
                memcpy(pv + nPitch / 2 * i, pQuad + nWidth / 2 * i, nWidth / 2 * sizeof(T));
            }
        }
    }

private:
    T *pQuad;
    int nWidth, nHeight;
};

class StopWatch {
public:
    void Start() {
        t0 = std::chrono::high_resolution_clock::now();
    }
    double Stop() {
        return std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::high_resolution_clock::now().time_since_epoch() - t0.time_since_epoch()).count() / 1.0e9;
    }

private:
    std::chrono::high_resolution_clock::time_point t0;
};

inline void CheckInputFile(const char *szInFilePath) {
    std::ifstream fpIn(szInFilePath, std::ios::in | std::ios::binary);
    if (fpIn.fail()) {
        std::ostringstream err;
        err << "Unable to open input file: " << szInFilePath << std::endl;
        throw std::invalid_argument(err.str());
    }
}

void Nv12ToBgra32(uint8_t *dpNv12, int nNv12Pitch, uint8_t *dpBgra, int nBgraPitch, int nWidth, int nHeight, int iMatrix = 0);
void Nv12ToBgra64(uint8_t *dpNv12, int nNv12Pitch, uint8_t *dpBgra, int nBgraPitch, int nWidth, int nHeight, int iMatrix = 0);

void P016ToBgra32(uint8_t *dpP016, int nP016Pitch, uint8_t *dpBgra, int nBgraPitch, int nWidth, int nHeight, int iMatrix = 4);
void P016ToBgra64(uint8_t *dpP016, int nP016Pitch, uint8_t *dpBgra, int nBgraPitch, int nWidth, int nHeight, int iMatrix = 4);

void Nv12ToBgrPlanar(uint8_t *dpNv12, int nNv12Pitch, uint8_t *dpBgrp, int nBgrpPitch, int nWidth, int nHeight, int iMatrix = 0);
void P016ToBgrPlanar(uint8_t *dpP016, int nP016Pitch, uint8_t *dpBgrp, int nBgrpPitch, int nWidth, int nHeight, int iMatrix = 4);

void Bgra64ToP016(uint8_t *dpBgra, int nBgraPitch, uint8_t *dpP016, int nP016Pitch, int nWidth, int nHeight, int iMatrix = 4);

void ConvertUInt8ToUInt16(uint8_t *dpUInt8, uint16_t *dpUInt16, int nSrcPitch, int nDestPitch, int nWidth, int nHeight);
void ConvertUInt16ToUInt8(uint16_t *dpUInt16, uint8_t *dpUInt8, int nSrcPitch, int nDestPitch, int nWidth, int nHeight);

void ResizeNv12(unsigned char *dpDstNv12, int nDstPitch, int nDstWidth, int nDstHeight, unsigned char *dpSrcNv12, int nSrcPitch, int nSrcWidth, int nSrcHeight, unsigned char *dpDstNv12UV = nullptr);
void ResizeP016(unsigned char *dpDstP016, int nDstPitch, int nDstWidth, int nDstHeight, unsigned char *dpSrcP016, int nSrcPitch, int nSrcWidth, int nSrcHeight, unsigned char *dpDstP016UV = nullptr);

void ScaleYUV420(unsigned char *dpDstY, unsigned char* dpDstU, unsigned char* dpDstV, int nDstPitch, int nDstChromaPitch, int nDstWidth, int nDstHeight,
    unsigned char *dpSrcY, unsigned char* dpSrcU, unsigned char* dpSrcV, int nSrcPitch, int nSrcChromaPitch, int nSrcWidth, int nSrcHeight, bool bSemiplanar);
