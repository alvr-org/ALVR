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
		mConverter = new CudaConverter(D3D11Device, width, height);
	}
	catch (Exception e) {
		throw MakeException(L"Exception:%s", e.what());
	}

	try {
		mEncoder = new NvEncoderCuda(mConverter->GetContext(), width, height, format, extraOutputDelay);
	}
	catch (NVENCException e) {
		throw MakeException(L"NvEnc NvEncoderCuda failed. Code=%d %hs", e.getErrorCode(), e.what());
	}
}


NvTextureEncoderCuda::~NvTextureEncoderCuda()
{
	delete mConverter;
	delete mEncoder;
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
