#include "NvTextureEncoderD3D11.h"
#include "NvEncoderD3D11.h"

NvTextureEncoderD3D11::NvTextureEncoderD3D11(ID3D11Device* D3D11Device, uint32_t width, uint32_t height, NV_ENC_BUFFER_FORMAT eBufferFormat,
	uint32_t extraOutputDelay)
{
	mEncoder = new NvEncoderD3D11(D3D11Device, width, height, eBufferFormat, extraOutputDelay);
}

NvTextureEncoderD3D11::~NvTextureEncoderD3D11()
{
	delete mEncoder;
}

void NvTextureEncoderD3D11::EncodeFrame(std::vector<std::vector<uint8_t>>& vPacket, NV_ENC_PIC_PARAMS * pPicParams, ID3D11DeviceContext *d3dContext, ID3D11Texture2D *texture)
{
	const NvEncInputFrame* encoderInputFrame = mEncoder->GetNextInputFrame();
	ID3D11Texture2D *pInputTexture = reinterpret_cast<ID3D11Texture2D*>(encoderInputFrame->inputPtr);

	d3dContext->CopyResource(pInputTexture, texture);
	mEncoder->EncodeFrame(vPacket, pPicParams);
}
