// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_MATCHMAKINGENQUEUEDUSER_H
#define OVR_MATCHMAKINGENQUEUEDUSER_H

#include "OVR_Platform_Defs.h"
#include "OVR_DataStore.h"
#include "OVR_Types.h"
#include "OVR_User.h"

typedef struct ovrMatchmakingEnqueuedUser *ovrMatchmakingEnqueuedUserHandle;

/// This method may return null. This indicates that the value is not present or that the curent
/// app or user is not permitted to access it.
OVRP_PUBLIC_FUNCTION(ovrDataStoreHandle) ovr_MatchmakingEnqueuedUser_GetCustomData(const ovrMatchmakingEnqueuedUserHandle obj);

/// This method may return null. This indicates that the value is not present or that the curent
/// app or user is not permitted to access it.
OVRP_PUBLIC_FUNCTION(ovrUserHandle) ovr_MatchmakingEnqueuedUser_GetUser(const ovrMatchmakingEnqueuedUserHandle obj);

OVRP_PUBLIC_FUNCTION(ovrID)        ovr_MatchmakingEnqueuedUser_GetAdditionalUserID(const ovrMatchmakingEnqueuedUserHandle obj, unsigned int index);
OVRP_PUBLIC_FUNCTION(unsigned int) ovr_MatchmakingEnqueuedUser_GetAdditionalUserIDsSize(const ovrMatchmakingEnqueuedUserHandle obj);

#endif
