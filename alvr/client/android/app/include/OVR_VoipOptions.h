// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_VOIP_OPTIONS_H
#define OVR_VOIP_OPTIONS_H

#include "OVR_Platform_Defs.h"
#include "OVR_Types.h"
#include <stddef.h>
#include <stdbool.h>

#include "OVR_VoipBitrate.h"
#include "OVR_VoipDtxState.h"

struct ovrVoipOptions;
typedef struct ovrVoipOptions* ovrVoipOptionsHandle;

OVRP_PUBLIC_FUNCTION(ovrVoipOptionsHandle) ovr_VoipOptions_Create();
OVRP_PUBLIC_FUNCTION(void) ovr_VoipOptions_Destroy(ovrVoipOptionsHandle handle);
OVRP_PUBLIC_FUNCTION(void) ovr_VoipOptions_SetBitrateForNewConnections(ovrVoipOptionsHandle handle, ovrVoipBitrate value);
OVRP_PUBLIC_FUNCTION(void) ovr_VoipOptions_SetCreateNewConnectionUseDtx(ovrVoipOptionsHandle handle, ovrVoipDtxState value);

#endif
