#pragma once

#include <d3d11.h>
#include "NvEncoder.h"

class INvTextureEncoder
{
public:
	virtual void CreateDefaultEncoderParams(NV_ENC_INITIALIZE_PARAMS* pIntializeParams, GUID codecGuid, GUID presetGuid) = 0;
	virtual void CreateEncoder(const NV_ENC_INITIALIZE_PARAMS* pEncoderParams) = 0;
	virtual void EncodeFrame(std::vector<std::vector<uint8_t>>& vPacket, NV_ENC_PIC_PARAMS * pPicParams, ID3D11DeviceContext *d3dContext, ID3D11Texture2D *texture) = 0;
	virtual bool Reconfigure(const NV_ENC_RECONFIGURE_PARAMS *pReconfigureParams) = 0;
	virtual void EndEncode(std::vector<std::vector<uint8_t>> &vPacket) = 0;
	virtual void DestroyEncoder() = 0;
};

