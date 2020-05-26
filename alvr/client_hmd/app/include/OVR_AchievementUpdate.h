// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_ACHIEVEMENTUPDATE_H
#define OVR_ACHIEVEMENTUPDATE_H

#include "OVR_Platform_Defs.h"
#include <stdbool.h>

typedef struct ovrAchievementUpdate *ovrAchievementUpdateHandle;

OVRP_PUBLIC_FUNCTION(bool)         ovr_AchievementUpdate_GetJustUnlocked(const ovrAchievementUpdateHandle obj);
OVRP_PUBLIC_FUNCTION(const char *) ovr_AchievementUpdate_GetName(const ovrAchievementUpdateHandle obj);

#endif
