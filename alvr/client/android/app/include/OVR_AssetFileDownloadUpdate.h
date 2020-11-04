// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_ASSETFILEDOWNLOADUPDATE_H
#define OVR_ASSETFILEDOWNLOADUPDATE_H

#include "OVR_Platform_Defs.h"
#include "OVR_Types.h"
#include <stdbool.h>

typedef struct ovrAssetFileDownloadUpdate *ovrAssetFileDownloadUpdateHandle;

/// DEPRECATED. Use ovr_AssetFileDownloadUpdate_GetAssetId().
OVRP_PUBLIC_FUNCTION(ovrID) ovr_AssetFileDownloadUpdate_GetAssetFileId(const ovrAssetFileDownloadUpdateHandle obj);

/// ID of the asset file
OVRP_PUBLIC_FUNCTION(ovrID) ovr_AssetFileDownloadUpdate_GetAssetId(const ovrAssetFileDownloadUpdateHandle obj);

/// Total number of bytes.
OVRP_PUBLIC_FUNCTION(unsigned int) ovr_AssetFileDownloadUpdate_GetBytesTotal(const ovrAssetFileDownloadUpdateHandle obj);

/// Number of bytes have been downloaded. -1 If the download hasn't started
/// yet.
OVRP_PUBLIC_FUNCTION(int) ovr_AssetFileDownloadUpdate_GetBytesTransferred(const ovrAssetFileDownloadUpdateHandle obj);

/// Flag indicating a download is completed.
OVRP_PUBLIC_FUNCTION(bool) ovr_AssetFileDownloadUpdate_GetCompleted(const ovrAssetFileDownloadUpdateHandle obj);


#endif
