#include "NvTextureEncoder.h"

void NvTextureEncoder::CreateDefaultEncoderParams(NV_ENC_INITIALIZE_PARAMS * pIntializeParams, GUID codecGuid, GUID presetGuid)
{
	mEncoder->CreateDefaultEncoderParams(pIntializeParams, codecGuid, presetGuid);
}

void NvTextureEncoder::CreateEncoder(const NV_ENC_INITIALIZE_PARAMS * pEncoderParams)
{
	mEncoder->CreateEncoder(pEncoderParams);
}

bool NvTextureEncoder::Reconfigure(const NV_ENC_RECONFIGURE_PARAMS * pReconfigureParams)
{
	return mEncoder->Reconfigure(pReconfigureParams);
}

void NvTextureEncoder::EndEncode(std::vector<std::vector<uint8_t>>& vPacket)
{
	mEncoder->EndEncode(vPacket);
}

void NvTextureEncoder::DestroyEncoder()
{
	mEncoder->DestroyEncoder();
}

int NvTextureEncoder::GetCapabilityValue(GUID guidCodec, NV_ENC_CAPS capsToQuery)
{
	return mEncoder->GetCapabilityValue(guidCodec, capsToQuery);
}

void NvTextureEncoder::InvalidateRefFrames(uint64_t invalidRefFrameTimeStamp)
{
	mEncoder->InvalidateRefFrames(invalidRefFrameTimeStamp);
}
