// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_VOIPDECODER_H
#define OVR_VOIPDECODER_H

#include "OVR_Platform_Defs.h"
#include <stddef.h>

typedef struct ovrVoipDecoder *ovrVoipDecoderHandle;

OVRP_PUBLIC_FUNCTION(void)   ovr_VoipDecoder_Decode(const ovrVoipDecoderHandle obj, const unsigned char *compressedData, size_t compressedSize);
OVRP_PUBLIC_FUNCTION(size_t) ovr_VoipDecoder_GetDecodedPCM(const ovrVoipDecoderHandle obj, float *outputBuffer, size_t outputBufferSize);

#endif
