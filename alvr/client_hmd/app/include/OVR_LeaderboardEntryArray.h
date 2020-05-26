// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_LEADERBOARDENTRYARRAY_H
#define OVR_LEADERBOARDENTRYARRAY_H

#include "OVR_Platform_Defs.h"
#include "OVR_LeaderboardEntry.h"
#include <stdbool.h>
#include <stddef.h>

typedef struct ovrLeaderboardEntryArray *ovrLeaderboardEntryArrayHandle;

OVRP_PUBLIC_FUNCTION(ovrLeaderboardEntryHandle) ovr_LeaderboardEntryArray_GetElement(const ovrLeaderboardEntryArrayHandle obj, size_t index);
OVRP_PUBLIC_FUNCTION(const char *)              ovr_LeaderboardEntryArray_GetNextUrl(const ovrLeaderboardEntryArrayHandle obj);
OVRP_PUBLIC_FUNCTION(const char *)              ovr_LeaderboardEntryArray_GetPreviousUrl(const ovrLeaderboardEntryArrayHandle obj);
OVRP_PUBLIC_FUNCTION(size_t)                    ovr_LeaderboardEntryArray_GetSize(const ovrLeaderboardEntryArrayHandle obj);
OVRP_PUBLIC_FUNCTION(unsigned long long)        ovr_LeaderboardEntryArray_GetTotalCount(const ovrLeaderboardEntryArrayHandle obj);
OVRP_PUBLIC_FUNCTION(bool)                      ovr_LeaderboardEntryArray_HasNextPage(const ovrLeaderboardEntryArrayHandle obj);
OVRP_PUBLIC_FUNCTION(bool)                      ovr_LeaderboardEntryArray_HasPreviousPage(const ovrLeaderboardEntryArrayHandle obj);

#endif
