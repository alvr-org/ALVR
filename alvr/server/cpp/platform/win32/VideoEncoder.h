#pragma once

#include <memory>
#include "shared/d3drender.h"
#include "alvr_server/ClientConnection.h"
#include "NvEncoderD3D11.h"

class VideoEncoder
{
public:
	virtual void Initialize() = 0;
	virtual void Shutdown() = 0;

	virtual void Transmit(ID3D11Texture2D *pTexture, uint64_t presentationTime, uint64_t frameIndex, uint64_t frameIndex2, bool insertIDR) = 0;
};
