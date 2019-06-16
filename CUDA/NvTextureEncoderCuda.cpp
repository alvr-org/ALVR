#include "NvTextureEncoderCuda.h"
#include <delayimp.h>

#include "CudaConverter.h"
#include "NvEncoderCuda.h"
#include "..\ALVR-common\exception.h"
#include "..\alvr_server\Logger.h"

NvTextureEncoderCuda::NvTextureEncoderCuda(ID3D11Device* D3D11Device, uint32_t width, uint32_t height, NV_ENC_BUFFER_FORMAT format,
	uint32_t extraOutputDelay)
{
	if (!LoadCudaDLL()) {
		throw MakeException(L"Failed to load nvcuda.dll. Please check if NVIDIA graphic driver is installed.");
	}
	try {
		mConverter = std::make_shared<CudaConverter>(D3D11Device, width, height);
	}
	catch (Exception e) {
		throw MakeException(L"Exception:%s", e.what());
	}

	try {
		mEncoder = std::make_shared<NvEncoderCuda>(mConverter->GetContext(), width, height, format, extraOutputDelay);
	}
	catch (NVENCException e) {
		throw MakeException(L"NvEnc NvEncoderCuda failed. Code=%d %hs", e.getErrorCode(), e.what());
	}
}


NvTextureEncoderCuda::~NvTextureEncoderCuda()
{
}

// Delay loading for Cuda driver API to correctly work on non-NVIDIA GPU.
bool NvTextureEncoderCuda::LoadCudaDLL() {
	__try {
		return !FAILED(__HrLoadAllImportsForDll("nvcuda.dll"));
	}
	__except (EXCEPTION_EXECUTE_HANDLER) {
	}
	return false;
}

void NvTextureEncoderCuda::CreateDefaultEncoderParams(NV_ENC_INITIALIZE_PARAMS * pIntializeParams, GUID codecGuid, GUID presetGuid)
{
	mEncoder->CreateDefaultEncoderParams(pIntializeParams, codecGuid, presetGuid);
}

void NvTextureEncoderCuda::CreateEncoder(const NV_ENC_INITIALIZE_PARAMS * pEncoderParams)
{
	mEncoder->CreateEncoder(pEncoderParams);
}

void NvTextureEncoderCuda::EncodeFrame(std::vector<std::vector<uint8_t>>& vPacket, NV_ENC_PIC_PARAMS * pPicParams, ID3D11DeviceContext *d3dContext, ID3D11Texture2D *texture)
{
	const NvEncInputFrame* encoderInputFrame = mEncoder->GetNextInputFrame();
	try {
		mConverter->Convert(texture, encoderInputFrame);
	}
	catch (NVENCException e) {
		throw MakeException(L"Exception:%hs", e.what());
	}

	mEncoder->EncodeFrame(vPacket, pPicParams);
}

bool NvTextureEncoderCuda::Reconfigure(const NV_ENC_RECONFIGURE_PARAMS * pReconfigureParams)
{
	return mEncoder->Reconfigure(pReconfigureParams);
}

void NvTextureEncoderCuda::EndEncode(std::vector<std::vector<uint8_t>>& vPacket)
{
	mEncoder->EndEncode(vPacket);
}

void NvTextureEncoderCuda::DestroyEncoder()
{
	mEncoder->DestroyEncoder();
}