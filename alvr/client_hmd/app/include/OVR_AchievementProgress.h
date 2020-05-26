// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_ACHIEVEMENTPROGRESS_H
#define OVR_ACHIEVEMENTPROGRESS_H

#include "OVR_Platform_Defs.h"
#include <stdbool.h>

typedef struct ovrAchievementProgress *ovrAchievementProgressHandle;

OVRP_PUBLIC_FUNCTION(const char *)       ovr_AchievementProgress_GetBitfield(const ovrAchievementProgressHandle obj);
OVRP_PUBLIC_FUNCTION(unsigned long long) ovr_AchievementProgress_GetCount(const ovrAchievementProgressHandle obj);
OVRP_PUBLIC_FUNCTION(bool)               ovr_AchievementProgress_GetIsUnlocked(const ovrAchievementProgressHandle obj);
OVRP_PUBLIC_FUNCTION(const char *)       ovr_AchievementProgress_GetName(const ovrAchievementProgressHandle obj);
OVRP_PUBLIC_FUNCTION(unsigned long long) ovr_AchievementProgress_GetUnlockTime(const ovrAchievementProgressHandle obj);

#endif
