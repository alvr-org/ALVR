// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_PINGRESULT_H
#define OVR_PINGRESULT_H

#include "OVR_Platform_Defs.h"
#include "OVR_Types.h"
#include <stdbool.h>

typedef struct ovrPingResult *ovrPingResultHandle;

OVRP_PUBLIC_FUNCTION(ovrID)              ovr_PingResult_GetID(const ovrPingResultHandle obj);
OVRP_PUBLIC_FUNCTION(unsigned long long) ovr_PingResult_GetPingTimeUsec(const ovrPingResultHandle obj);
OVRP_PUBLIC_FUNCTION(bool)               ovr_PingResult_IsTimeout(const ovrPingResultHandle obj);

#endif
