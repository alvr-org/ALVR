#pragma once

#include <memory>
#include "d3drender.h"
#include "ClientConnection.h"
#include "NvEncoderD3D11.h"

class VideoEncoder
{
public:
	virtual void Initialize() = 0;
	virtual void Shutdown() = 0;

	virtual void Transmit(ID3D11Texture2D *pTexture, uint64_t presentationTime, uint64_t frameIndex, uint64_t frameIndex2, uint64_t clientTime, bool insertIDR) = 0;

	virtual void Reconfigure(int refreshRate, int renderWidth, int renderHeight, int bitrateInMBits) = 0;
};