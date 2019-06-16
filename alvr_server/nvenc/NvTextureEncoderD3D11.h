#pragma once
#include "INvTextureEncoder.h"

class NvEncoderD3D11;

class NvTextureEncoderD3D11 :
	public INvTextureEncoder
{
public:
	NvTextureEncoderD3D11(ID3D11Device* pD3D11Device, uint32_t nWidth, uint32_t nHeight, NV_ENC_BUFFER_FORMAT eBufferFormat,
		uint32_t nExtraOutputDelay);
	~NvTextureEncoderD3D11();
	
	virtual void CreateDefaultEncoderParams(NV_ENC_INITIALIZE_PARAMS* pIntializeParams, GUID codecGuid, GUID presetGuid);
	virtual void CreateEncoder(const NV_ENC_INITIALIZE_PARAMS* pEncoderParams);
	virtual void EncodeFrame(std::vector<std::vector<uint8_t>>& vPacket, NV_ENC_PIC_PARAMS * pPicParams, ID3D11DeviceContext *d3dContext, ID3D11Texture2D *texture);
	virtual bool Reconfigure(const NV_ENC_RECONFIGURE_PARAMS *pReconfigureParams);
	virtual void EndEncode(std::vector<std::vector<uint8_t>> &vPacket);
	virtual void DestroyEncoder();

private:
	std::shared_ptr<NvEncoderD3D11> mEncoder;
};

