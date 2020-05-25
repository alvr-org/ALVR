// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_SYSTEMVOIPSTATE_H
#define OVR_SYSTEMVOIPSTATE_H

#include "OVR_Platform_Defs.h"
#include "OVR_SystemVoipStatus.h"
#include "OVR_VoipMuteState.h"

typedef struct ovrSystemVoipState *ovrSystemVoipStateHandle;

OVRP_PUBLIC_FUNCTION(ovrVoipMuteState)    ovr_SystemVoipState_GetMicrophoneMuted(const ovrSystemVoipStateHandle obj);
OVRP_PUBLIC_FUNCTION(ovrSystemVoipStatus) ovr_SystemVoipState_GetStatus(const ovrSystemVoipStateHandle obj);

#endif
