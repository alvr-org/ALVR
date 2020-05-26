// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_DESTINATION_H
#define OVR_DESTINATION_H

#include "OVR_Platform_Defs.h"

typedef struct ovrDestination *ovrDestinationHandle;

/// Pass it into ovr_RichPresenceOptions_SetApiName() when calling
/// ovr_RichPresence_Set() to set this user's rich presence
OVRP_PUBLIC_FUNCTION(const char *) ovr_Destination_GetApiName(const ovrDestinationHandle obj);

/// The information that will be in ovr_LaunchDetails_GetDeeplinkMessage() when
/// a user enters via a deeplink. Alternatively will be in
/// ovr_User_GetPresenceDeeplinkMessage() if the rich presence is set for the
/// user.
OVRP_PUBLIC_FUNCTION(const char *) ovr_Destination_GetDeeplinkMessage(const ovrDestinationHandle obj);

OVRP_PUBLIC_FUNCTION(const char *) ovr_Destination_GetDisplayName(const ovrDestinationHandle obj);

#endif
