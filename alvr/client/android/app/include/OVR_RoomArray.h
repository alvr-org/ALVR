// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_ROOMARRAY_H
#define OVR_ROOMARRAY_H

#include "OVR_Platform_Defs.h"
#include "OVR_Room.h"
#include <stdbool.h>
#include <stddef.h>

typedef struct ovrRoomArray *ovrRoomArrayHandle;

OVRP_PUBLIC_FUNCTION(ovrRoomHandle) ovr_RoomArray_GetElement(const ovrRoomArrayHandle obj, size_t index);
OVRP_PUBLIC_FUNCTION(const char *)  ovr_RoomArray_GetNextUrl(const ovrRoomArrayHandle obj);
OVRP_PUBLIC_FUNCTION(size_t)        ovr_RoomArray_GetSize(const ovrRoomArrayHandle obj);
OVRP_PUBLIC_FUNCTION(bool)          ovr_RoomArray_HasNextPage(const ovrRoomArrayHandle obj);

#endif
