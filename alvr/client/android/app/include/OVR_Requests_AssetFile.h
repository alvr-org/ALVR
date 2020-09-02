// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_REQUESTS_ASSETFILE_H
#define OVR_REQUESTS_ASSETFILE_H

#include "OVR_Types.h"
#include "OVR_Platform_Defs.h"


/// \file
/// # Asset Files
///
/// An asset file is an extra dynamic contents which can be downloaded post-installation at runtime. This allows reducing initial installation size, and download the contents as users progress in your app. As an example might be extra game levels which are downloaded after finishing first set of levels.
///
/// Asset files are also used as a storage layer for DLCs (downloadable content), which can be proposed to a user separately from the main app.
///
/// If your apps supports asset files, users are able to download them from within the Oculus app on PDP page of the main app (including in Home), and you also can manage the asset files from your app itself via SDK calls.

/// DEPRECATED. Use ovr_AssetFile_DeleteById()
///
/// A message with type ::ovrMessage_AssetFile_Delete will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrAssetFileDeleteResultHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetAssetFileDeleteResult().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_AssetFile_Delete(ovrID assetFileID);

/// Removes an previously installed asset file from the device by its ID.
/// Returns an object containing the asset ID and file name, and a success
/// flag.
/// \param assetFileID The asset file ID
///
/// A message with type ::ovrMessage_AssetFile_DeleteById will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrAssetFileDeleteResultHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetAssetFileDeleteResult().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_AssetFile_DeleteById(ovrID assetFileID);

/// Removes an previously installed asset file from the device by its name.
/// Returns an object containing the asset ID and file name, and a success
/// flag.
/// \param assetFileName The asset file name
///
/// A message with type ::ovrMessage_AssetFile_DeleteByName will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrAssetFileDeleteResultHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetAssetFileDeleteResult().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_AssetFile_DeleteByName(const char *assetFileName);

/// DEPRECATED. Use ovr_AssetFile_DownloadById()
///
/// A message with type ::ovrMessage_AssetFile_Download will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrAssetFileDownloadResultHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetAssetFileDownloadResult().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_AssetFile_Download(ovrID assetFileID);

/// Downloads an asset file by its ID on demand. Returns an object containing
/// the asset ID and filepath. Sends periodic
/// ovrNotification_AssetFile_DownloadUpdate to track the downloads.
/// \param assetFileID The asset file ID
///
/// A message with type ::ovrMessage_AssetFile_DownloadById will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrAssetFileDownloadResultHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetAssetFileDownloadResult().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_AssetFile_DownloadById(ovrID assetFileID);

/// Downloads an asset file by its name on demand. Returns an object containing
/// the asset ID and filepath. Sends periodic
/// {notifications.asset_file.download_update}} to track the downloads.
/// \param assetFileName The asset file name
///
/// A message with type ::ovrMessage_AssetFile_DownloadByName will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrAssetFileDownloadResultHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetAssetFileDownloadResult().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_AssetFile_DownloadByName(const char *assetFileName);

/// DEPRECATED. Use ovr_AssetFile_DownloadCancelById()
///
/// A message with type ::ovrMessage_AssetFile_DownloadCancel will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrAssetFileDownloadCancelResultHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetAssetFileDownloadCancelResult().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_AssetFile_DownloadCancel(ovrID assetFileID);

/// Cancels a previously spawned download request for an asset file by its ID.
/// Returns an object containing the asset ID and file path, and a success
/// flag.
/// \param assetFileID The asset file ID
///
/// A message with type ::ovrMessage_AssetFile_DownloadCancelById will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrAssetFileDownloadCancelResultHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetAssetFileDownloadCancelResult().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_AssetFile_DownloadCancelById(ovrID assetFileID);

/// Cancels a previously spawned download request for an asset file by its
/// name. Returns an object containing the asset ID and file path, and a
/// success flag.
/// \param assetFileName The asset file name
///
/// A message with type ::ovrMessage_AssetFile_DownloadCancelByName will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrAssetFileDownloadCancelResultHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetAssetFileDownloadCancelResult().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_AssetFile_DownloadCancelByName(const char *assetFileName);

/// Returns an array of objects with asset file names and their associated IDs,
/// and and whether it's currently installed.
///
/// A message with type ::ovrMessage_AssetFile_GetList will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrAssetDetailsArrayHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetAssetDetailsArray().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_AssetFile_GetList();

/// DEPRECATED. Use ovr_AssetFile_StatusById()
///
/// A message with type ::ovrMessage_AssetFile_Status will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrAssetDetailsHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetAssetDetails().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_AssetFile_Status(ovrID assetFileID);

/// Returns the details on a single asset: ID, file name, and whether it's
/// currently installed
/// \param assetFileID The asset file ID
///
/// A message with type ::ovrMessage_AssetFile_StatusById will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrAssetDetailsHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetAssetDetails().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_AssetFile_StatusById(ovrID assetFileID);

/// Returns the details on a single asset: ID, file name, and whether it's
/// currently installed
/// \param assetFileName The asset file name
///
/// A message with type ::ovrMessage_AssetFile_StatusByName will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrAssetDetailsHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetAssetDetails().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_AssetFile_StatusByName(const char *assetFileName);

#endif
