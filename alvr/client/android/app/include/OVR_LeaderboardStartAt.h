// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_LEADERBOARD_START_AT_H
#define OVR_LEADERBOARD_START_AT_H

#include "OVR_Platform_Defs.h"

typedef enum ovrLeaderboardStartAt_ {
  ovrLeaderboard_StartAtTop,
  ovrLeaderboard_StartAtCenteredOnViewer,
  ovrLeaderboard_StartAtCenteredOnViewerOrTop,
  ovrLeaderboard_StartAtUnknown,
} ovrLeaderboardStartAt;

/// Converts an ovrLeaderboardStartAt enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrLeaderboardStartAt_ToString(ovrLeaderboardStartAt value);

/// Converts a string representing an ovrLeaderboardStartAt enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrLeaderboardStartAt) ovrLeaderboardStartAt_FromString(const char* str);

#endif
