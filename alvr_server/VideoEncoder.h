#pragma once

#include <memory>
#include "d3drender.h"
#include "Listener.h"
#include "NvEncoderD3D11.h"
#include "NvEncoderCuda.h"

class VideoEncoder
{
public:
	virtual bool Initialize() = 0;
	virtual void Shutdown() = 0;

	virtual void Transmit(ID3D11Texture2D *pTexture, uint64_t presentationTime, uint64_t frameIndex, uint64_t frameIndex2, uint64_t clientTime) = 0;

	virtual void OnPacketLoss() = 0;

	virtual void OnClientConnected() = 0;

protected:
	void SaveDebugOutput(std::shared_ptr<CD3DRender> m_pD3DRender, std::vector<std::vector<uint8_t>> &vPacket, ID3D11Texture2D *texture, uint64_t frameIndex);
};