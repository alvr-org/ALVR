// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_MATCHMAKINGADMINSNAPSHOT_H
#define OVR_MATCHMAKINGADMINSNAPSHOT_H

#include "OVR_Platform_Defs.h"
#include "OVR_MatchmakingAdminSnapshotCandidateArray.h"

typedef struct ovrMatchmakingAdminSnapshot *ovrMatchmakingAdminSnapshotHandle;

OVRP_PUBLIC_FUNCTION(ovrMatchmakingAdminSnapshotCandidateArrayHandle) ovr_MatchmakingAdminSnapshot_GetCandidates(const ovrMatchmakingAdminSnapshotHandle obj);
OVRP_PUBLIC_FUNCTION(double)                                          ovr_MatchmakingAdminSnapshot_GetMyCurrentThreshold(const ovrMatchmakingAdminSnapshotHandle obj);

#endif
