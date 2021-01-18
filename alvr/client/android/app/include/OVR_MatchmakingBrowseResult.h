// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_MATCHMAKINGBROWSERESULT_H
#define OVR_MATCHMAKINGBROWSERESULT_H

#include "OVR_Platform_Defs.h"
#include "OVR_MatchmakingEnqueueResult.h"
#include "OVR_MatchmakingRoomArray.h"

typedef struct ovrMatchmakingBrowseResult *ovrMatchmakingBrowseResultHandle;

OVRP_PUBLIC_FUNCTION(ovrMatchmakingEnqueueResultHandle) ovr_MatchmakingBrowseResult_GetEnqueueResult(const ovrMatchmakingBrowseResultHandle obj);
OVRP_PUBLIC_FUNCTION(ovrMatchmakingRoomArrayHandle)     ovr_MatchmakingBrowseResult_GetRooms(const ovrMatchmakingBrowseResultHandle obj);

#endif
