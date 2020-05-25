// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_ACHIEVEMENTDEFINITION_H
#define OVR_ACHIEVEMENTDEFINITION_H

#include "OVR_Platform_Defs.h"
#include "OVR_AchievementType.h"

typedef struct ovrAchievementDefinition *ovrAchievementDefinitionHandle;

/// The name of the achievement.
OVRP_PUBLIC_FUNCTION(const char *) ovr_AchievementDefinition_GetName(const ovrAchievementDefinitionHandle obj);

OVRP_PUBLIC_FUNCTION(unsigned int)       ovr_AchievementDefinition_GetBitfieldLength(const ovrAchievementDefinitionHandle obj);
OVRP_PUBLIC_FUNCTION(unsigned long long) ovr_AchievementDefinition_GetTarget(const ovrAchievementDefinitionHandle obj);
OVRP_PUBLIC_FUNCTION(ovrAchievementType) ovr_AchievementDefinition_GetType(const ovrAchievementDefinitionHandle obj);

#endif
