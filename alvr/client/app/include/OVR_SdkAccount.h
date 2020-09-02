// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_SDKACCOUNT_H
#define OVR_SDKACCOUNT_H

#include "OVR_Platform_Defs.h"
#include "OVR_SdkAccountType.h"
#include "OVR_Types.h"

typedef struct ovrSdkAccount *ovrSdkAccountHandle;

OVRP_PUBLIC_FUNCTION(ovrSdkAccountType) ovr_SdkAccount_GetAccountType(const ovrSdkAccountHandle obj);
OVRP_PUBLIC_FUNCTION(ovrID)             ovr_SdkAccount_GetUserId(const ovrSdkAccountHandle obj);

#endif
