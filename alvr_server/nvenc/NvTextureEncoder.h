#pragma once

#include <d3d11.h>
#include "NvEncoder.h"

class NvTextureEncoder
{
public:
	virtual void CreateDefaultEncoderParams(NV_ENC_INITIALIZE_PARAMS* pIntializeParams, GUID codecGuid, GUID presetGuid);
	virtual void CreateEncoder(const NV_ENC_INITIALIZE_PARAMS* pEncoderParams);
	virtual bool Reconfigure(const NV_ENC_RECONFIGURE_PARAMS *pReconfigureParams);
	virtual void EndEncode(std::vector<std::vector<uint8_t>> &vPacket);
	virtual void DestroyEncoder();

	virtual int GetCapabilityValue(GUID guidCodec, NV_ENC_CAPS capsToQuery);
	virtual void InvalidateRefFrames(uint64_t invalidRefFrameTimeStamp);

	virtual void EncodeFrame(std::vector<std::vector<uint8_t>>& vPacket, NV_ENC_PIC_PARAMS * pPicParams, ID3D11DeviceContext *d3dContext, ID3D11Texture2D *texture) = 0;
protected:
	NvEncoder *mEncoder;
};

