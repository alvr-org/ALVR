// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_VOIPENCODER_H
#define OVR_VOIPENCODER_H

#include "OVR_Platform_Defs.h"
#include <stddef.h>

typedef struct ovrVoipEncoder *ovrVoipEncoderHandle;

OVRP_PUBLIC_FUNCTION(void)   ovr_VoipEncoder_AddPCM(const ovrVoipEncoderHandle obj, const float *inputData, unsigned int inputSize);
OVRP_PUBLIC_FUNCTION(size_t) ovr_VoipEncoder_GetCompressedData(const ovrVoipEncoderHandle obj, unsigned char *outputBuffer, size_t intputSize);
OVRP_PUBLIC_FUNCTION(size_t) ovr_VoipEncoder_GetCompressedDataSize(const ovrVoipEncoderHandle obj);

#endif
