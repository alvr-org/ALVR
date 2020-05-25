// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_REQUESTS_CLOUDSTORAGE2_H
#define OVR_REQUESTS_CLOUDSTORAGE2_H

#include "OVR_Types.h"
#include "OVR_Platform_Defs.h"


/// Get the directory path for the current user/app pair that will be used
/// during cloud storage synchronization
///
/// A message with type ::ovrMessage_CloudStorage2_GetUserDirectoryPath will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type const char *.
/// Extract the payload from the message handle with ::ovr_Message_GetString().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_CloudStorage2_GetUserDirectoryPath();

#endif
