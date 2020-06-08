// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_LEADERBOARD_FILTER_TYPE_H
#define OVR_LEADERBOARD_FILTER_TYPE_H

#include "OVR_Platform_Defs.h"

typedef enum ovrLeaderboardFilterType_ {
  ovrLeaderboard_FilterNone,
  ovrLeaderboard_FilterFriends,
  ovrLeaderboard_FilterUnknown,
  ovrLeaderboard_FilterUserIds,
} ovrLeaderboardFilterType;

/// Converts an ovrLeaderboardFilterType enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrLeaderboardFilterType_ToString(ovrLeaderboardFilterType value);

/// Converts a string representing an ovrLeaderboardFilterType enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrLeaderboardFilterType) ovrLeaderboardFilterType_FromString(const char* str);

#endif
