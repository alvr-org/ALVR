// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_MATCHMAKING_CRITERION_IMPORTANCE_H
#define OVR_MATCHMAKING_CRITERION_IMPORTANCE_H

#include "OVR_Platform_Defs.h"

typedef enum ovrMatchmakingCriterionImportance_ {
  ovrMatchmaking_ImportanceRequired,
  ovrMatchmaking_ImportanceHigh,
  ovrMatchmaking_ImportanceMedium,
  ovrMatchmaking_ImportanceLow,
  ovrMatchmaking_ImportanceUnknown,
} ovrMatchmakingCriterionImportance;

/// Converts an ovrMatchmakingCriterionImportance enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrMatchmakingCriterionImportance_ToString(ovrMatchmakingCriterionImportance value);

/// Converts a string representing an ovrMatchmakingCriterionImportance enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrMatchmakingCriterionImportance) ovrMatchmakingCriterionImportance_FromString(const char* str);

#endif
