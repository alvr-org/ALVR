// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_ROOM_OPTIONS_H
#define OVR_ROOM_OPTIONS_H

#include "OVR_Platform_Defs.h"
#include "OVR_Types.h"
#include <stddef.h>
#include <stdbool.h>

#include "OVR_TimeWindow.h"
#include "OVR_UserOrdering.h"

struct ovrRoomOptions;
typedef struct ovrRoomOptions* ovrRoomOptionsHandle;

OVRP_PUBLIC_FUNCTION(ovrRoomOptionsHandle) ovr_RoomOptions_Create();
OVRP_PUBLIC_FUNCTION(void) ovr_RoomOptions_Destroy(ovrRoomOptionsHandle handle);
OVRP_PUBLIC_FUNCTION(void) ovr_RoomOptions_SetDataStoreString(ovrRoomOptionsHandle handle, const char* key, const char* value);
OVRP_PUBLIC_FUNCTION(void) ovr_RoomOptions_ClearDataStore(ovrRoomOptionsHandle handle);
OVRP_PUBLIC_FUNCTION(void) ovr_RoomOptions_SetExcludeRecentlyMet(ovrRoomOptionsHandle handle, bool value);
OVRP_PUBLIC_FUNCTION(void) ovr_RoomOptions_SetMaxUserResults(ovrRoomOptionsHandle handle, unsigned int value);
OVRP_PUBLIC_FUNCTION(void) ovr_RoomOptions_SetOrdering(ovrRoomOptionsHandle handle, ovrUserOrdering value);
OVRP_PUBLIC_FUNCTION(void) ovr_RoomOptions_SetRecentlyMetTimeWindow(ovrRoomOptionsHandle handle, ovrTimeWindow value);
OVRP_PUBLIC_FUNCTION(void) ovr_RoomOptions_SetRoomId(ovrRoomOptionsHandle handle, ovrID value);
OVRP_PUBLIC_FUNCTION(void) ovr_RoomOptions_SetTurnOffUpdates(ovrRoomOptionsHandle handle, bool value);

#endif
