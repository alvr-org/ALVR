// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_USER_H
#define OVR_USER_H

#include "OVR_Platform_Defs.h"
#include "OVR_Types.h"
#include "OVR_UserPresenceStatus.h"

typedef struct ovrUser *ovrUserHandle;

/// Human readable string of what the user is currently doing. Not intended to
/// be parsed as it might change at anytime or be translated
OVRP_PUBLIC_FUNCTION(const char *) ovr_User_GetPresence(const ovrUserHandle obj);

/// Intended to be parsed and used to deeplink to parts of the app
OVRP_PUBLIC_FUNCTION(const char *) ovr_User_GetPresenceDeeplinkMessage(const ovrUserHandle obj);

/// If provided, the destination this user is currently at in the app
OVRP_PUBLIC_FUNCTION(const char *) ovr_User_GetPresenceDestinationApiName(const ovrUserHandle obj);

/// Enum value of what the user is currently doing.
OVRP_PUBLIC_FUNCTION(ovrUserPresenceStatus) ovr_User_GetPresenceStatus(const ovrUserHandle obj);

OVRP_PUBLIC_FUNCTION(ovrID)        ovr_User_GetID(const ovrUserHandle obj);
OVRP_PUBLIC_FUNCTION(const char *) ovr_User_GetImageUrl(const ovrUserHandle obj);
OVRP_PUBLIC_FUNCTION(const char *) ovr_User_GetInviteToken(const ovrUserHandle obj);
OVRP_PUBLIC_FUNCTION(const char *) ovr_User_GetOculusID(const ovrUserHandle obj);
OVRP_PUBLIC_FUNCTION(const char *) ovr_User_GetSmallImageUrl(const ovrUserHandle obj);

#endif
