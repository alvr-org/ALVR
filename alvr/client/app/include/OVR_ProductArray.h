// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_PRODUCTARRAY_H
#define OVR_PRODUCTARRAY_H

#include "OVR_Platform_Defs.h"
#include "OVR_Product.h"
#include <stdbool.h>
#include <stddef.h>

typedef struct ovrProductArray *ovrProductArrayHandle;

OVRP_PUBLIC_FUNCTION(ovrProductHandle) ovr_ProductArray_GetElement(const ovrProductArrayHandle obj, size_t index);
OVRP_PUBLIC_FUNCTION(const char *)     ovr_ProductArray_GetNextUrl(const ovrProductArrayHandle obj);
OVRP_PUBLIC_FUNCTION(size_t)           ovr_ProductArray_GetSize(const ovrProductArrayHandle obj);
OVRP_PUBLIC_FUNCTION(bool)             ovr_ProductArray_HasNextPage(const ovrProductArrayHandle obj);

#endif
