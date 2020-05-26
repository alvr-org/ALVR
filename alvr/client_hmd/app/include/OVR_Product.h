// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_PRODUCT_H
#define OVR_PRODUCT_H

#include "OVR_Platform_Defs.h"

typedef struct ovrProduct *ovrProductHandle;

OVRP_PUBLIC_FUNCTION(const char *) ovr_Product_GetDescription(const ovrProductHandle obj);
OVRP_PUBLIC_FUNCTION(const char *) ovr_Product_GetFormattedPrice(const ovrProductHandle obj);
OVRP_PUBLIC_FUNCTION(const char *) ovr_Product_GetName(const ovrProductHandle obj);
OVRP_PUBLIC_FUNCTION(const char *) ovr_Product_GetSKU(const ovrProductHandle obj);

#endif
