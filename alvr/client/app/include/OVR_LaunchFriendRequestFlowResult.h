// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_LAUNCHFRIENDREQUESTFLOWRESULT_H
#define OVR_LAUNCHFRIENDREQUESTFLOWRESULT_H

#include "OVR_Platform_Defs.h"
#include <stdbool.h>

typedef struct ovrLaunchFriendRequestFlowResult *ovrLaunchFriendRequestFlowResultHandle;

/// Whether the viewer chose to cancel the friend request flow.
OVRP_PUBLIC_FUNCTION(bool) ovr_LaunchFriendRequestFlowResult_GetDidCancel(const ovrLaunchFriendRequestFlowResultHandle obj);

/// Whether the viewer successfully sent the friend request.
OVRP_PUBLIC_FUNCTION(bool) ovr_LaunchFriendRequestFlowResult_GetDidSendRequest(const ovrLaunchFriendRequestFlowResultHandle obj);


#endif
