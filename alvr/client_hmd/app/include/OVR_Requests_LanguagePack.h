// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_REQUESTS_LANGUAGEPACK_H
#define OVR_REQUESTS_LANGUAGEPACK_H

#include "OVR_Types.h"
#include "OVR_Platform_Defs.h"


/// Returns currently installed and selected language pack for an app in the
/// view of the `asset_details`. Use `language` field to extract neeeded
/// language info. A particular language can be download and installed by a
/// user from the Oculus app on the application page.
///
/// A message with type ::ovrMessage_LanguagePack_GetCurrent will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrAssetDetailsHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetAssetDetails().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_LanguagePack_GetCurrent();

/// Sets the current language to specified. The parameter is the BCP47 language
/// tag. If a language pack is not downloaded yet, spawns automatically the
/// ovr_AssetFile_DownloadByName() request, and sends periodic
/// ovrNotification_AssetFile_DownloadUpdate to track the downloads. Once the
/// language asset file is downloaded, call ovr_LanguagePack_GetCurrent() to
/// retrive the data, and use the language at runtime.
/// \param tag BCP47 language tag
///
/// A message with type ::ovrMessage_LanguagePack_SetCurrent will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrAssetFileDownloadResultHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetAssetFileDownloadResult().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_LanguagePack_SetCurrent(const char *tag);

#endif
