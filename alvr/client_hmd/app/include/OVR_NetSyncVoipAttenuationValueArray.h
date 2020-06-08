// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_NETSYNCVOIPATTENUATIONVALUEARRAY_H
#define OVR_NETSYNCVOIPATTENUATIONVALUEARRAY_H

#include "OVR_Platform_Defs.h"
#include "OVR_NetSyncVoipAttenuationValue.h"
#include <stddef.h>

typedef struct ovrNetSyncVoipAttenuationValueArray *ovrNetSyncVoipAttenuationValueArrayHandle;

OVRP_PUBLIC_FUNCTION(ovrNetSyncVoipAttenuationValueHandle) ovr_NetSyncVoipAttenuationValueArray_GetElement(const ovrNetSyncVoipAttenuationValueArrayHandle obj, size_t index);
OVRP_PUBLIC_FUNCTION(size_t)                               ovr_NetSyncVoipAttenuationValueArray_GetSize(const ovrNetSyncVoipAttenuationValueArrayHandle obj);

#endif
