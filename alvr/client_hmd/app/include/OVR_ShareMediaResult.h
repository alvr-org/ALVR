// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_SHAREMEDIARESULT_H
#define OVR_SHAREMEDIARESULT_H

#include "OVR_Platform_Defs.h"
#include "OVR_ShareMediaStatus.h"

typedef struct ovrShareMediaResult *ovrShareMediaResultHandle;

OVRP_PUBLIC_FUNCTION(ovrShareMediaStatus) ovr_ShareMediaResult_GetStatus(const ovrShareMediaResultHandle obj);

#endif
