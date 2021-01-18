// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_USER_OPTIONS_H
#define OVR_USER_OPTIONS_H

#include "OVR_Platform_Defs.h"
#include "OVR_Types.h"
#include <stddef.h>
#include <stdbool.h>

#include "OVR_ServiceProvider.h"
#include "OVR_TimeWindow.h"

struct ovrUserOptions;
typedef struct ovrUserOptions* ovrUserOptionsHandle;

OVRP_PUBLIC_FUNCTION(ovrUserOptionsHandle) ovr_UserOptions_Create();
OVRP_PUBLIC_FUNCTION(void) ovr_UserOptions_Destroy(ovrUserOptionsHandle handle);
OVRP_PUBLIC_FUNCTION(void) ovr_UserOptions_SetMaxUsers(ovrUserOptionsHandle handle, unsigned int value);
OVRP_PUBLIC_FUNCTION(void) ovr_UserOptions_AddServiceProvider(ovrUserOptionsHandle handle, ovrServiceProvider value);
OVRP_PUBLIC_FUNCTION(void) ovr_UserOptions_ClearServiceProviders(ovrUserOptionsHandle handle);
OVRP_PUBLIC_FUNCTION(void) ovr_UserOptions_SetTimeWindow(ovrUserOptionsHandle handle, ovrTimeWindow value);

#endif
