// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_ACHIEVEMENT_TYPE_H
#define OVR_ACHIEVEMENT_TYPE_H

#include "OVR_Platform_Defs.h"

typedef enum ovrAchievementType_ {
  ovrAchievement_TypeUnknown,
  ovrAchievement_TypeSimple,
  ovrAchievement_TypeBitfield,
  ovrAchievement_TypeCount,
} ovrAchievementType;

/// Converts an ovrAchievementType enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrAchievementType_ToString(ovrAchievementType value);

/// Converts a string representing an ovrAchievementType enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrAchievementType) ovrAchievementType_FromString(const char* str);

#endif
