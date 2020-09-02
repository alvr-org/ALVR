// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_MATCHMAKING_STAT_APPROACH_H
#define OVR_MATCHMAKING_STAT_APPROACH_H

#include "OVR_Platform_Defs.h"

typedef enum ovrMatchmakingStatApproach_ {
  ovrMatchmakingStatApproach_Unknown,
  ovrMatchmakingStatApproach_Trailing,
  ovrMatchmakingStatApproach_Swingy,
} ovrMatchmakingStatApproach;

/// Converts an ovrMatchmakingStatApproach enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrMatchmakingStatApproach_ToString(ovrMatchmakingStatApproach value);

/// Converts a string representing an ovrMatchmakingStatApproach enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrMatchmakingStatApproach) ovrMatchmakingStatApproach_FromString(const char* str);

#endif
