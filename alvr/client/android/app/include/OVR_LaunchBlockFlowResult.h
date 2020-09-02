// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_LAUNCHBLOCKFLOWRESULT_H
#define OVR_LAUNCHBLOCKFLOWRESULT_H

#include "OVR_Platform_Defs.h"
#include <stdbool.h>

typedef struct ovrLaunchBlockFlowResult *ovrLaunchBlockFlowResultHandle;

/// Whether the viewer successfully blocked the user.
OVRP_PUBLIC_FUNCTION(bool) ovr_LaunchBlockFlowResult_GetDidBlock(const ovrLaunchBlockFlowResultHandle obj);

/// Whether the viewer chose to cancel the block flow.
OVRP_PUBLIC_FUNCTION(bool) ovr_LaunchBlockFlowResult_GetDidCancel(const ovrLaunchBlockFlowResultHandle obj);


#endif
