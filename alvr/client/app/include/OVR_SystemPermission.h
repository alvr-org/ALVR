// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_SYSTEMPERMISSION_H
#define OVR_SYSTEMPERMISSION_H

#include "OVR_Platform_Defs.h"
#include "OVR_PermissionGrantStatus.h"
#include <stdbool.h>

typedef struct ovrSystemPermission *ovrSystemPermissionHandle;

OVRP_PUBLIC_FUNCTION(bool)                     ovr_SystemPermission_GetHasPermission(const ovrSystemPermissionHandle obj);
OVRP_PUBLIC_FUNCTION(ovrPermissionGrantStatus) ovr_SystemPermission_GetPermissionGrantStatus(const ovrSystemPermissionHandle obj);

#endif
