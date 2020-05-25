#ifndef OVR_VOIP_LOWLEVEL_H
#define OVR_VOIP_LOWLEVEL_H

#include "OVR_Platform_Defs.h"
#include "OVR_Types.h"

#include "OVR_VoipEncoder.h"
#include "OVR_VoipDecoder.h"
#include "OVR_Microphone.h"

#ifdef __cplusplus
extern "C" {
#endif

OVRP_PUBLIC_FUNCTION(ovrVoipEncoderHandle) ovr_Voip_CreateEncoder();
OVRP_PUBLIC_FUNCTION(void) ovr_Voip_DestroyEncoder(ovrVoipEncoderHandle encoder);

OVRP_PUBLIC_FUNCTION(ovrVoipDecoderHandle) ovr_Voip_CreateDecoder();
OVRP_PUBLIC_FUNCTION(void) ovr_Voip_DestroyDecoder(ovrVoipDecoderHandle decoder);

OVRP_PUBLIC_FUNCTION(ovrMicrophoneHandle) ovr_Microphone_Create();
OVRP_PUBLIC_FUNCTION(void) ovr_Microphone_Destroy(ovrMicrophoneHandle obj);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // OVR_VOIP_LOWLEVEL_H
