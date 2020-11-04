// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_ASSETFILEDELETERESULT_H
#define OVR_ASSETFILEDELETERESULT_H

#include "OVR_Platform_Defs.h"
#include "OVR_Types.h"
#include <stdbool.h>

typedef struct ovrAssetFileDeleteResult *ovrAssetFileDeleteResultHandle;

/// DEPRECATED. Use ovr_AssetFileDeleteResult_GetAssetId().
OVRP_PUBLIC_FUNCTION(ovrID) ovr_AssetFileDeleteResult_GetAssetFileId(const ovrAssetFileDeleteResultHandle obj);

/// ID of the asset file
OVRP_PUBLIC_FUNCTION(ovrID) ovr_AssetFileDeleteResult_GetAssetId(const ovrAssetFileDeleteResultHandle obj);

/// File path of the asset file.
OVRP_PUBLIC_FUNCTION(const char *) ovr_AssetFileDeleteResult_GetFilepath(const ovrAssetFileDeleteResultHandle obj);

/// Whether the asset delete was successful.
OVRP_PUBLIC_FUNCTION(bool) ovr_AssetFileDeleteResult_GetSuccess(const ovrAssetFileDeleteResultHandle obj);


#endif
