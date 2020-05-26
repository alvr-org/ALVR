// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_ROOMINVITENOTIFICATIONARRAY_H
#define OVR_ROOMINVITENOTIFICATIONARRAY_H

#include "OVR_Platform_Defs.h"
#include "OVR_RoomInviteNotification.h"
#include <stdbool.h>
#include <stddef.h>

typedef struct ovrRoomInviteNotificationArray *ovrRoomInviteNotificationArrayHandle;

OVRP_PUBLIC_FUNCTION(ovrRoomInviteNotificationHandle) ovr_RoomInviteNotificationArray_GetElement(const ovrRoomInviteNotificationArrayHandle obj, size_t index);
OVRP_PUBLIC_FUNCTION(const char *)                    ovr_RoomInviteNotificationArray_GetNextUrl(const ovrRoomInviteNotificationArrayHandle obj);
OVRP_PUBLIC_FUNCTION(size_t)                          ovr_RoomInviteNotificationArray_GetSize(const ovrRoomInviteNotificationArrayHandle obj);
OVRP_PUBLIC_FUNCTION(bool)                            ovr_RoomInviteNotificationArray_HasNextPage(const ovrRoomInviteNotificationArrayHandle obj);

#endif
