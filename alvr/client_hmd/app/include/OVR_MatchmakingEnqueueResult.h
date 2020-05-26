// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_MATCHMAKINGENQUEUERESULT_H
#define OVR_MATCHMAKINGENQUEUERESULT_H

#include "OVR_Platform_Defs.h"
#include "OVR_MatchmakingAdminSnapshot.h"

typedef struct ovrMatchmakingEnqueueResult *ovrMatchmakingEnqueueResultHandle;

/// If 'IsDebug' is set in ovrMatchmakingOptionsHandle, this will return with
/// the enqueue results.
/// This method may return null. This indicates that the value is not present or that the curent
/// app or user is not permitted to access it.
OVRP_PUBLIC_FUNCTION(ovrMatchmakingAdminSnapshotHandle) ovr_MatchmakingEnqueueResult_GetAdminSnapshot(const ovrMatchmakingEnqueueResultHandle obj);

/// The average amount of time (mean average) that users in this queue have
/// waited during the last hour or more. The wait times, whether the users
/// canceled or found a match, are used to generate this value. Use this to
/// give users an indication of how long they can expect to wait.
OVRP_PUBLIC_FUNCTION(unsigned int) ovr_MatchmakingEnqueueResult_GetAverageWait(const ovrMatchmakingEnqueueResultHandle obj);

/// The number of matches made from the pool the user is participating in. You
/// can use this to give users an indication of whether they should bother to
/// wait.
OVRP_PUBLIC_FUNCTION(unsigned int) ovr_MatchmakingEnqueueResult_GetMatchesInLastHourCount(const ovrMatchmakingEnqueueResultHandle obj);

/// The amount of time the 95th percentile waited during the last hour or more.
/// The wait times, whether the users canceled or found a match, are used to
/// generate this value. Use this to give users an indication of the maximum
/// amount of time they can expect to wait.
OVRP_PUBLIC_FUNCTION(unsigned int) ovr_MatchmakingEnqueueResult_GetMaxExpectedWait(const ovrMatchmakingEnqueueResultHandle obj);

/// Percentage of people in the same queue as the user who got matched, from 0
/// to 100 percent. Stats are taken from the last hour or more. You can use
/// this to give users an indication of whether they should wait.
OVRP_PUBLIC_FUNCTION(unsigned int) ovr_MatchmakingEnqueueResult_GetRecentMatchPercentage(const ovrMatchmakingEnqueueResultHandle obj);

OVRP_PUBLIC_FUNCTION(const char *) ovr_MatchmakingEnqueueResult_GetPool(const ovrMatchmakingEnqueueResultHandle obj);
OVRP_PUBLIC_FUNCTION(const char *) ovr_MatchmakingEnqueueResult_GetRequestHash(const ovrMatchmakingEnqueueResultHandle obj);

#endif
