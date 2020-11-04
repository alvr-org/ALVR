// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_MATCHMAKINGENQUEUEDUSERARRAY_H
#define OVR_MATCHMAKINGENQUEUEDUSERARRAY_H

#include "OVR_Platform_Defs.h"
#include "OVR_MatchmakingEnqueuedUser.h"
#include <stddef.h>

typedef struct ovrMatchmakingEnqueuedUserArray *ovrMatchmakingEnqueuedUserArrayHandle;

OVRP_PUBLIC_FUNCTION(ovrMatchmakingEnqueuedUserHandle) ovr_MatchmakingEnqueuedUserArray_GetElement(const ovrMatchmakingEnqueuedUserArrayHandle obj, size_t index);
OVRP_PUBLIC_FUNCTION(size_t)                           ovr_MatchmakingEnqueuedUserArray_GetSize(const ovrMatchmakingEnqueuedUserArrayHandle obj);

#endif
