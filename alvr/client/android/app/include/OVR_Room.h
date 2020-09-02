// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_ROOM_H
#define OVR_ROOM_H

#include "OVR_Platform_Defs.h"
#include "OVR_DataStore.h"
#include "OVR_MatchmakingEnqueuedUserArray.h"
#include "OVR_RoomJoinability.h"
#include "OVR_RoomJoinPolicy.h"
#include "OVR_RoomType.h"
#include "OVR_TeamArray.h"
#include "OVR_Types.h"
#include "OVR_User.h"
#include "OVR_UserArray.h"
#include <stdbool.h>

typedef struct ovrRoom *ovrRoomHandle;

/// This method may return null. This indicates that the value is not present or that the curent
/// app or user is not permitted to access it.
OVRP_PUBLIC_FUNCTION(ovrDataStoreHandle) ovr_Room_GetDataStore(const ovrRoomHandle obj);

/// A list of users that have been invited to the room, but have not joined the
/// room yet.
/// This method may return null. This indicates that the value is not present or that the curent
/// app or user is not permitted to access it.
OVRP_PUBLIC_FUNCTION(ovrUserArrayHandle) ovr_Room_GetInvitedUsers(const ovrRoomHandle obj);

/// If this is a matchmaking room, it contains all users matched into the room,
/// including the host as well as users enqueued by someone else. Also includes
/// additional per-user matchmaking metadata.
/// This method may return null. This indicates that the value is not present or that the curent
/// app or user is not permitted to access it.
OVRP_PUBLIC_FUNCTION(ovrMatchmakingEnqueuedUserArrayHandle) ovr_Room_GetMatchedUsers(const ovrRoomHandle obj);

/// This method may return null. This indicates that the value is not present or that the curent
/// app or user is not permitted to access it.
OVRP_PUBLIC_FUNCTION(ovrUserHandle) ovr_Room_GetOwner(const ovrRoomHandle obj);

/// This method may return null. This indicates that the value is not present or that the curent
/// app or user is not permitted to access it.
OVRP_PUBLIC_FUNCTION(ovrTeamArrayHandle) ovr_Room_GetTeams(const ovrRoomHandle obj);

/// This method may return null. This indicates that the value is not present or that the curent
/// app or user is not permitted to access it.
OVRP_PUBLIC_FUNCTION(ovrUserArrayHandle) ovr_Room_GetUsers(const ovrRoomHandle obj);

OVRP_PUBLIC_FUNCTION(ovrID)              ovr_Room_GetApplicationID(const ovrRoomHandle obj);
OVRP_PUBLIC_FUNCTION(const char *)       ovr_Room_GetDescription(const ovrRoomHandle obj);
OVRP_PUBLIC_FUNCTION(ovrID)              ovr_Room_GetID(const ovrRoomHandle obj);
OVRP_PUBLIC_FUNCTION(bool)               ovr_Room_GetIsMembershipLocked(const ovrRoomHandle obj);
OVRP_PUBLIC_FUNCTION(ovrRoomJoinPolicy)  ovr_Room_GetJoinPolicy(const ovrRoomHandle obj);
OVRP_PUBLIC_FUNCTION(ovrRoomJoinability) ovr_Room_GetJoinability(const ovrRoomHandle obj);
OVRP_PUBLIC_FUNCTION(unsigned int)       ovr_Room_GetMaxUsers(const ovrRoomHandle obj);
OVRP_PUBLIC_FUNCTION(const char *)       ovr_Room_GetName(const ovrRoomHandle obj);
OVRP_PUBLIC_FUNCTION(ovrRoomType)        ovr_Room_GetType(const ovrRoomHandle obj);
OVRP_PUBLIC_FUNCTION(unsigned int)       ovr_Room_GetVersion(const ovrRoomHandle obj);

#endif
