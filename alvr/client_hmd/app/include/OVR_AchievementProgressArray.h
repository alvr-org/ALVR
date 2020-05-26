// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_ACHIEVEMENTPROGRESSARRAY_H
#define OVR_ACHIEVEMENTPROGRESSARRAY_H

#include "OVR_Platform_Defs.h"
#include "OVR_AchievementProgress.h"
#include <stdbool.h>
#include <stddef.h>

typedef struct ovrAchievementProgressArray *ovrAchievementProgressArrayHandle;

OVRP_PUBLIC_FUNCTION(ovrAchievementProgressHandle) ovr_AchievementProgressArray_GetElement(const ovrAchievementProgressArrayHandle obj, size_t index);
OVRP_PUBLIC_FUNCTION(const char *)                 ovr_AchievementProgressArray_GetNextUrl(const ovrAchievementProgressArrayHandle obj);
OVRP_PUBLIC_FUNCTION(size_t)                       ovr_AchievementProgressArray_GetSize(const ovrAchievementProgressArrayHandle obj);
OVRP_PUBLIC_FUNCTION(bool)                         ovr_AchievementProgressArray_HasNextPage(const ovrAchievementProgressArrayHandle obj);

#endif
