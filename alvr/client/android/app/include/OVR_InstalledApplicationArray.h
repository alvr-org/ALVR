// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_INSTALLEDAPPLICATIONARRAY_H
#define OVR_INSTALLEDAPPLICATIONARRAY_H

#include "OVR_Platform_Defs.h"
#include "OVR_InstalledApplication.h"
#include <stddef.h>

typedef struct ovrInstalledApplicationArray *ovrInstalledApplicationArrayHandle;

OVRP_PUBLIC_FUNCTION(ovrInstalledApplicationHandle) ovr_InstalledApplicationArray_GetElement(const ovrInstalledApplicationArrayHandle obj, size_t index);
OVRP_PUBLIC_FUNCTION(size_t)                        ovr_InstalledApplicationArray_GetSize(const ovrInstalledApplicationArrayHandle obj);

#endif
