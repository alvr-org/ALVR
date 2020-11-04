// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_SDKACCOUNTARRAY_H
#define OVR_SDKACCOUNTARRAY_H

#include "OVR_Platform_Defs.h"
#include "OVR_SdkAccount.h"
#include <stddef.h>

typedef struct ovrSdkAccountArray *ovrSdkAccountArrayHandle;

OVRP_PUBLIC_FUNCTION(ovrSdkAccountHandle) ovr_SdkAccountArray_GetElement(const ovrSdkAccountArrayHandle obj, size_t index);
OVRP_PUBLIC_FUNCTION(size_t)              ovr_SdkAccountArray_GetSize(const ovrSdkAccountArrayHandle obj);

#endif
