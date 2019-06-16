#include "NvTextureEncoderD3D11.h"
#include "NvEncoderD3D11.h"

NvTextureEncoderD3D11::NvTextureEncoderD3D11(ID3D11Device* D3D11Device, uint32_t width, uint32_t height, NV_ENC_BUFFER_FORMAT eBufferFormat,
	uint32_t extraOutputDelay)
{
	mEncoder = std::make_shared<NvEncoderD3D11>(D3D11Device, width, height, eBufferFormat, extraOutputDelay);
}


NvTextureEncoderD3D11::~NvTextureEncoderD3D11()
{
}

void NvTextureEncoderD3D11::CreateDefaultEncoderParams(NV_ENC_INITIALIZE_PARAMS * pIntializeParams, GUID codecGuid, GUID presetGuid)
{
	mEncoder->CreateDefaultEncoderParams(pIntializeParams, codecGuid, presetGuid);
}

void NvTextureEncoderD3D11::CreateEncoder(const NV_ENC_INITIALIZE_PARAMS * pEncoderParams)
{
	mEncoder->CreateEncoder(pEncoderParams);
}

void NvTextureEncoderD3D11::EncodeFrame(std::vector<std::vector<uint8_t>>& vPacket, NV_ENC_PIC_PARAMS * pPicParams, ID3D11DeviceContext *d3dContext, ID3D11Texture2D *texture)
{
	const NvEncInputFrame* encoderInputFrame = mEncoder->GetNextInputFrame();
	ID3D11Texture2D *pInputTexture = reinterpret_cast<ID3D11Texture2D*>(encoderInputFrame->inputPtr);

	d3dContext->CopyResource(pInputTexture, texture);
	mEncoder->EncodeFrame(vPacket, pPicParams);
}

bool NvTextureEncoderD3D11::Reconfigure(const NV_ENC_RECONFIGURE_PARAMS * pReconfigureParams)
{
	return mEncoder->Reconfigure(pReconfigureParams);
}

void NvTextureEncoderD3D11::EndEncode(std::vector<std::vector<uint8_t>>& vPacket)
{
	mEncoder->EndEncode(vPacket);
}

void NvTextureEncoderD3D11::DestroyEncoder()
{
	mEncoder->DestroyEncoder();
}