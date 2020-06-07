// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_NETSYNCVOIPATTENUATIONVALUE_H
#define OVR_NETSYNCVOIPATTENUATIONVALUE_H

#include "OVR_Platform_Defs.h"

typedef struct ovrNetSyncVoipAttenuationValue *ovrNetSyncVoipAttenuationValueHandle;

/// decibel fall-off value
OVRP_PUBLIC_FUNCTION(float) ovr_NetSyncVoipAttenuationValue_GetDecibels(const ovrNetSyncVoipAttenuationValueHandle obj);

/// The starting distance of this attenuation value
OVRP_PUBLIC_FUNCTION(float) ovr_NetSyncVoipAttenuationValue_GetDistance(const ovrNetSyncVoipAttenuationValueHandle obj);


#endif
