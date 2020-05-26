// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_ASSETDETAILSARRAY_H
#define OVR_ASSETDETAILSARRAY_H

#include "OVR_Platform_Defs.h"
#include "OVR_AssetDetails.h"
#include <stddef.h>

typedef struct ovrAssetDetailsArray *ovrAssetDetailsArrayHandle;

OVRP_PUBLIC_FUNCTION(ovrAssetDetailsHandle) ovr_AssetDetailsArray_GetElement(const ovrAssetDetailsArrayHandle obj, size_t index);
OVRP_PUBLIC_FUNCTION(size_t)                ovr_AssetDetailsArray_GetSize(const ovrAssetDetailsArrayHandle obj);

#endif
