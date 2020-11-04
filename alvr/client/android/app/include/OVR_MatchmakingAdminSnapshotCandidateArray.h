// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_MATCHMAKINGADMINSNAPSHOTCANDIDATEARRAY_H
#define OVR_MATCHMAKINGADMINSNAPSHOTCANDIDATEARRAY_H

#include "OVR_Platform_Defs.h"
#include "OVR_MatchmakingAdminSnapshotCandidate.h"
#include <stddef.h>

typedef struct ovrMatchmakingAdminSnapshotCandidateArray *ovrMatchmakingAdminSnapshotCandidateArrayHandle;

OVRP_PUBLIC_FUNCTION(ovrMatchmakingAdminSnapshotCandidateHandle) ovr_MatchmakingAdminSnapshotCandidateArray_GetElement(const ovrMatchmakingAdminSnapshotCandidateArrayHandle obj, size_t index);
OVRP_PUBLIC_FUNCTION(size_t)                                     ovr_MatchmakingAdminSnapshotCandidateArray_GetSize(const ovrMatchmakingAdminSnapshotCandidateArrayHandle obj);

#endif
