// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_ASSETFILEDOWNLOADRESULT_H
#define OVR_ASSETFILEDOWNLOADRESULT_H

#include "OVR_Platform_Defs.h"
#include "OVR_Types.h"

typedef struct ovrAssetFileDownloadResult *ovrAssetFileDownloadResultHandle;

/// ID of the asset file
OVRP_PUBLIC_FUNCTION(ovrID) ovr_AssetFileDownloadResult_GetAssetId(const ovrAssetFileDownloadResultHandle obj);

/// File path of the asset file.
OVRP_PUBLIC_FUNCTION(const char *) ovr_AssetFileDownloadResult_GetFilepath(const ovrAssetFileDownloadResultHandle obj);


#endif
