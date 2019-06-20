#pragma once

#include <memory>
#include "openvr-utils\d3drender.h"
#include "Listener.h"
#include "Bitrate.h"

class VideoEncoder
{
public:
	virtual void Initialize() = 0;
	virtual void Shutdown() = 0;

	virtual void Transmit(ID3D11Texture2D *pTexture, uint64_t presentationTime, uint64_t videoFrameIndex, uint64_t trackingFrameIndex, uint64_t clientTime, bool insertIDR) = 0;
	virtual void Reconfigure(int refreshRate, int renderWidth, int renderHeight, Bitrate bitrate) = 0;

	virtual bool SupportsReferenceFrameInvalidation() = 0;
	virtual void InvalidateReferenceFrame(uint64_t videoFrameIndex) = 0;
protected:
	void SaveDebugOutput(std::shared_ptr<CD3DRender> mD3DRender, std::vector<std::vector<uint8_t>> &vPacket, ID3D11Texture2D *texture, uint64_t frameIndex);
};