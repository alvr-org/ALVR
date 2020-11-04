// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_USERARRAY_H
#define OVR_USERARRAY_H

#include "OVR_Platform_Defs.h"
#include "OVR_User.h"
#include <stdbool.h>
#include <stddef.h>

typedef struct ovrUserArray *ovrUserArrayHandle;

OVRP_PUBLIC_FUNCTION(ovrUserHandle) ovr_UserArray_GetElement(const ovrUserArrayHandle obj, size_t index);
OVRP_PUBLIC_FUNCTION(const char *)  ovr_UserArray_GetNextUrl(const ovrUserArrayHandle obj);
OVRP_PUBLIC_FUNCTION(size_t)        ovr_UserArray_GetSize(const ovrUserArrayHandle obj);
OVRP_PUBLIC_FUNCTION(bool)          ovr_UserArray_HasNextPage(const ovrUserArrayHandle obj);

#endif
