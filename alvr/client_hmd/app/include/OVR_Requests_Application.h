// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_REQUESTS_APPLICATION_H
#define OVR_REQUESTS_APPLICATION_H

#include "OVR_Types.h"
#include "OVR_Platform_Defs.h"

#include "OVR_ApplicationOptions.h"
#include "OVR_ApplicationVersion.h"

/// \file
/// *** Application Overview:
/// An application is what you're writing! These requests/methods will allow you to
/// pull basic metadata about your application.

/// Requests version information, including the currently installed and latest
/// available version name and version code.
///
/// A message with type ::ovrMessage_Application_GetVersion will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrApplicationVersionHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetApplicationVersion().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Application_GetVersion();

/// Launches a different application in the user's library. If the user does
/// not have that application installed, they will be taken to that app's page
/// in the Oculus Store
/// \param appID The ID of the app to launch
/// \param deeplink_options Additional configuration for this requests. Optional.
///
/// A message with type ::ovrMessage_Application_LaunchOtherApp will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type const char *.
/// Extract the payload from the message handle with ::ovr_Message_GetString().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Application_LaunchOtherApp(ovrID appID, ovrApplicationOptionsHandle deeplink_options);

#endif
