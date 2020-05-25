// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_ACHIEVEMENTDEFINITIONARRAY_H
#define OVR_ACHIEVEMENTDEFINITIONARRAY_H

#include "OVR_Platform_Defs.h"
#include "OVR_AchievementDefinition.h"
#include <stdbool.h>
#include <stddef.h>

typedef struct ovrAchievementDefinitionArray *ovrAchievementDefinitionArrayHandle;

OVRP_PUBLIC_FUNCTION(ovrAchievementDefinitionHandle) ovr_AchievementDefinitionArray_GetElement(const ovrAchievementDefinitionArrayHandle obj, size_t index);
OVRP_PUBLIC_FUNCTION(const char *)                   ovr_AchievementDefinitionArray_GetNextUrl(const ovrAchievementDefinitionArrayHandle obj);
OVRP_PUBLIC_FUNCTION(size_t)                         ovr_AchievementDefinitionArray_GetSize(const ovrAchievementDefinitionArrayHandle obj);
OVRP_PUBLIC_FUNCTION(bool)                           ovr_AchievementDefinitionArray_HasNextPage(const ovrAchievementDefinitionArrayHandle obj);

#endif
