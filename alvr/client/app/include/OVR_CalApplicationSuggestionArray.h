// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_CALAPPLICATIONSUGGESTIONARRAY_H
#define OVR_CALAPPLICATIONSUGGESTIONARRAY_H

#include "OVR_Platform_Defs.h"
#include "OVR_CalApplicationSuggestion.h"
#include <stddef.h>

typedef struct ovrCalApplicationSuggestionArray *ovrCalApplicationSuggestionArrayHandle;

OVRP_PUBLIC_FUNCTION(ovrCalApplicationSuggestionHandle) ovr_CalApplicationSuggestionArray_GetElement(const ovrCalApplicationSuggestionArrayHandle obj, size_t index);
OVRP_PUBLIC_FUNCTION(size_t)                            ovr_CalApplicationSuggestionArray_GetSize(const ovrCalApplicationSuggestionArrayHandle obj);

#endif
