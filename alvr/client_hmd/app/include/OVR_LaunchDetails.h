// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_LAUNCHDETAILS_H
#define OVR_LAUNCHDETAILS_H

#include "OVR_Platform_Defs.h"
#include "OVR_LaunchType.h"
#include "OVR_Types.h"
#include "OVR_UserArray.h"

typedef struct ovrLaunchDetails *ovrLaunchDetailsHandle;

/// An opaque string provided by the developer to help them deeplink to content
/// on app startup.
OVRP_PUBLIC_FUNCTION(const char *) ovr_LaunchDetails_GetDeeplinkMessage(const ovrLaunchDetailsHandle obj);

/// If provided, the intended destination the user would like to go to
OVRP_PUBLIC_FUNCTION(const char *) ovr_LaunchDetails_GetDestinationApiName(const ovrLaunchDetailsHandle obj);

/// A string typically used to distinguish where the deeplink came from. For
/// instance, a DEEPLINK launch type could be coming from events or rich
/// presence.
OVRP_PUBLIC_FUNCTION(const char *) ovr_LaunchDetails_GetLaunchSource(const ovrLaunchDetailsHandle obj);

/// If provided, the intended room the user would like to be in
OVRP_PUBLIC_FUNCTION(ovrID) ovr_LaunchDetails_GetRoomID(const ovrLaunchDetailsHandle obj);

/// If provided, the intended users the user would like to be with
/// This method may return null. This indicates that the value is not present or that the curent
/// app or user is not permitted to access it.
OVRP_PUBLIC_FUNCTION(ovrUserArrayHandle) ovr_LaunchDetails_GetUsers(const ovrLaunchDetailsHandle obj);

OVRP_PUBLIC_FUNCTION(ovrLaunchType) ovr_LaunchDetails_GetLaunchType(const ovrLaunchDetailsHandle obj);

#endif
