// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_NET_SYNC_OPTIONS_H
#define OVR_NET_SYNC_OPTIONS_H

#include "OVR_Platform_Defs.h"
#include "OVR_Types.h"
#include <stddef.h>
#include <stdbool.h>

#include "OVR_NetSyncVoipStreamMode.h"

struct ovrNetSyncOptions;
typedef struct ovrNetSyncOptions* ovrNetSyncOptionsHandle;

OVRP_PUBLIC_FUNCTION(ovrNetSyncOptionsHandle) ovr_NetSyncOptions_Create();
OVRP_PUBLIC_FUNCTION(void) ovr_NetSyncOptions_Destroy(ovrNetSyncOptionsHandle handle);
OVRP_PUBLIC_FUNCTION(void) ovr_NetSyncOptions_SetVoipGroup(ovrNetSyncOptionsHandle handle, const char * value);
OVRP_PUBLIC_FUNCTION(void) ovr_NetSyncOptions_SetVoipStreamDefault(ovrNetSyncOptionsHandle handle, ovrNetSyncVoipStreamMode value);
OVRP_PUBLIC_FUNCTION(void) ovr_NetSyncOptions_SetZoneId(ovrNetSyncOptionsHandle handle, const char * value);

#endif
