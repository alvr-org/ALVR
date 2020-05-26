// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_ROOMINVITENOTIFICATION_H
#define OVR_ROOMINVITENOTIFICATION_H

#include "OVR_Platform_Defs.h"
#include "OVR_Types.h"

typedef struct ovrRoomInviteNotification *ovrRoomInviteNotificationHandle;

OVRP_PUBLIC_FUNCTION(ovrID)              ovr_RoomInviteNotification_GetID(const ovrRoomInviteNotificationHandle obj);
OVRP_PUBLIC_FUNCTION(ovrID)              ovr_RoomInviteNotification_GetRoomID(const ovrRoomInviteNotificationHandle obj);
OVRP_PUBLIC_FUNCTION(ovrID)              ovr_RoomInviteNotification_GetSenderID(const ovrRoomInviteNotificationHandle obj);
OVRP_PUBLIC_FUNCTION(unsigned long long) ovr_RoomInviteNotification_GetSentTime(const ovrRoomInviteNotificationHandle obj);

#endif
