// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_USERANDROOM_H
#define OVR_USERANDROOM_H

#include "OVR_Platform_Defs.h"
#include "OVR_Room.h"
#include "OVR_User.h"

typedef struct ovrUserAndRoom *ovrUserAndRoomHandle;

/// This method may return null. This indicates that the value is not present or that the curent
/// app or user is not permitted to access it.
OVRP_PUBLIC_FUNCTION(ovrRoomHandle) ovr_UserAndRoom_GetRoom(const ovrUserAndRoomHandle obj);

OVRP_PUBLIC_FUNCTION(ovrUserHandle) ovr_UserAndRoom_GetUser(const ovrUserAndRoomHandle obj);

#endif
