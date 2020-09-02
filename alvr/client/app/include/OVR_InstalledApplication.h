// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_INSTALLEDAPPLICATION_H
#define OVR_INSTALLEDAPPLICATION_H

#include "OVR_Platform_Defs.h"

typedef struct ovrInstalledApplication *ovrInstalledApplicationHandle;

OVRP_PUBLIC_FUNCTION(const char *) ovr_InstalledApplication_GetApplicationId(const ovrInstalledApplicationHandle obj);
OVRP_PUBLIC_FUNCTION(const char *) ovr_InstalledApplication_GetPackageName(const ovrInstalledApplicationHandle obj);
OVRP_PUBLIC_FUNCTION(const char *) ovr_InstalledApplication_GetStatus(const ovrInstalledApplicationHandle obj);
OVRP_PUBLIC_FUNCTION(int)          ovr_InstalledApplication_GetVersionCode(const ovrInstalledApplicationHandle obj);
OVRP_PUBLIC_FUNCTION(const char *) ovr_InstalledApplication_GetVersionName(const ovrInstalledApplicationHandle obj);

#endif
