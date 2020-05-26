// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_APPLICATION_OPTIONS_H
#define OVR_APPLICATION_OPTIONS_H

#include "OVR_Platform_Defs.h"
#include "OVR_Types.h"
#include <stddef.h>
#include <stdbool.h>


struct ovrApplicationOptions;
typedef struct ovrApplicationOptions* ovrApplicationOptionsHandle;

OVRP_PUBLIC_FUNCTION(ovrApplicationOptionsHandle) ovr_ApplicationOptions_Create();
OVRP_PUBLIC_FUNCTION(void) ovr_ApplicationOptions_Destroy(ovrApplicationOptionsHandle handle);
OVRP_PUBLIC_FUNCTION(void) ovr_ApplicationOptions_SetDeeplinkMessage(ovrApplicationOptionsHandle handle, const char * value);

#endif
