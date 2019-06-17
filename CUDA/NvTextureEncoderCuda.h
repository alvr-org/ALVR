#pragma once
#include "..\alvr_server\nvenc\NvTextureEncoder.h"

class CudaConverter;

class NvTextureEncoderCuda :
	public NvTextureEncoder
{
public:
	NvTextureEncoderCuda(ID3D11Device* pD3D11Device, uint32_t nWidth, uint32_t nHeight, NV_ENC_BUFFER_FORMAT eBufferFormat,
		uint32_t nExtraOutputDelay);
	~NvTextureEncoderCuda();
	
	virtual void EncodeFrame(std::vector<std::vector<uint8_t>>& vPacket, NV_ENC_PIC_PARAMS * pPicParams, ID3D11DeviceContext *d3dContext, ID3D11Texture2D *texture);
private:
	CudaConverter *mConverter;

	bool LoadCudaDLL();
};

