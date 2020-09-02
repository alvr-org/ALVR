// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_MATCHMAKINGADMINSNAPSHOTCANDIDATE_H
#define OVR_MATCHMAKINGADMINSNAPSHOTCANDIDATE_H

#include "OVR_Platform_Defs.h"
#include <stdbool.h>

typedef struct ovrMatchmakingAdminSnapshotCandidate *ovrMatchmakingAdminSnapshotCandidateHandle;

OVRP_PUBLIC_FUNCTION(bool)         ovr_MatchmakingAdminSnapshotCandidate_GetCanMatch(const ovrMatchmakingAdminSnapshotCandidateHandle obj);
OVRP_PUBLIC_FUNCTION(double)       ovr_MatchmakingAdminSnapshotCandidate_GetMyTotalScore(const ovrMatchmakingAdminSnapshotCandidateHandle obj);
OVRP_PUBLIC_FUNCTION(double)       ovr_MatchmakingAdminSnapshotCandidate_GetTheirCurrentThreshold(const ovrMatchmakingAdminSnapshotCandidateHandle obj);
OVRP_PUBLIC_FUNCTION(double)       ovr_MatchmakingAdminSnapshotCandidate_GetTheirTotalScore(const ovrMatchmakingAdminSnapshotCandidateHandle obj);
OVRP_PUBLIC_FUNCTION(const char *) ovr_MatchmakingAdminSnapshotCandidate_GetTraceId(const ovrMatchmakingAdminSnapshotCandidateHandle obj);

#endif
