// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_LAUNCHUNBLOCKFLOWRESULT_H
#define OVR_LAUNCHUNBLOCKFLOWRESULT_H

#include "OVR_Platform_Defs.h"
#include <stdbool.h>

typedef struct ovrLaunchUnblockFlowResult *ovrLaunchUnblockFlowResultHandle;

/// Whether the viewer chose to cancel the unblock flow.
OVRP_PUBLIC_FUNCTION(bool) ovr_LaunchUnblockFlowResult_GetDidCancel(const ovrLaunchUnblockFlowResultHandle obj);

/// Whether the viewer successfully unblocked the user.
OVRP_PUBLIC_FUNCTION(bool) ovr_LaunchUnblockFlowResult_GetDidUnblock(const ovrLaunchUnblockFlowResultHandle obj);


#endif
