// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_ASSETDETAILS_H
#define OVR_ASSETDETAILS_H

#include "OVR_Platform_Defs.h"
#include "OVR_LanguagePackInfo.h"
#include "OVR_Types.h"

typedef struct ovrAssetDetails *ovrAssetDetailsHandle;

/// ID of the asset file
OVRP_PUBLIC_FUNCTION(ovrID) ovr_AssetDetails_GetAssetId(const ovrAssetDetailsHandle obj);

/// One of 'default', 'store', or 'language_pack'. The 'default' type denotes
/// this Asset File is used purely as an implementation detail (to download
/// extra content post-installation). The 'store' type shows, that the Asset
/// File should be shown in Store. The 'language_pack' is a special type used
/// to manage different languages and translation data, which can be downloaded
/// post-installation.
OVRP_PUBLIC_FUNCTION(const char *) ovr_AssetDetails_GetAssetType(const ovrAssetDetailsHandle obj);

/// One of 'installed', 'available', or 'in-progress'
OVRP_PUBLIC_FUNCTION(const char *) ovr_AssetDetails_GetDownloadStatus(const ovrAssetDetailsHandle obj);

/// File path of the asset file
OVRP_PUBLIC_FUNCTION(const char *) ovr_AssetDetails_GetFilepath(const ovrAssetDetailsHandle obj);

/// One of 'free', 'entitled', or 'not-entitled'
OVRP_PUBLIC_FUNCTION(const char *) ovr_AssetDetails_GetIapStatus(const ovrAssetDetailsHandle obj);

/// For 'language_pack' assets type, contains language info.
/// This method may return null. This indicates that the value is not present or that the curent
/// app or user is not permitted to access it.
OVRP_PUBLIC_FUNCTION(ovrLanguagePackInfoHandle) ovr_AssetDetails_GetLanguage(const ovrAssetDetailsHandle obj);

/// Extra metadata associated with this asset file
OVRP_PUBLIC_FUNCTION(const char *) ovr_AssetDetails_GetMetadata(const ovrAssetDetailsHandle obj);


#endif
