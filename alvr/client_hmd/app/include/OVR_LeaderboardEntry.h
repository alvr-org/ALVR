// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_LEADERBOARDENTRY_H
#define OVR_LEADERBOARDENTRY_H

#include "OVR_Platform_Defs.h"
#include "OVR_User.h"

typedef struct ovrLeaderboardEntry *ovrLeaderboardEntryHandle;

OVRP_PUBLIC_FUNCTION(const char *)       ovr_LeaderboardEntry_GetExtraData(const ovrLeaderboardEntryHandle obj);
OVRP_PUBLIC_FUNCTION(unsigned int)       ovr_LeaderboardEntry_GetExtraDataLength(const ovrLeaderboardEntryHandle obj);
OVRP_PUBLIC_FUNCTION(int)                ovr_LeaderboardEntry_GetRank(const ovrLeaderboardEntryHandle obj);
OVRP_PUBLIC_FUNCTION(long long)          ovr_LeaderboardEntry_GetScore(const ovrLeaderboardEntryHandle obj);
OVRP_PUBLIC_FUNCTION(unsigned long long) ovr_LeaderboardEntry_GetTimestamp(const ovrLeaderboardEntryHandle obj);
OVRP_PUBLIC_FUNCTION(ovrUserHandle)      ovr_LeaderboardEntry_GetUser(const ovrLeaderboardEntryHandle obj);

#endif
