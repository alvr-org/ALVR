// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_LAUNCHREPORTFLOWRESULT_H
#define OVR_LAUNCHREPORTFLOWRESULT_H

#include "OVR_Platform_Defs.h"
#include "OVR_Types.h"
#include <stdbool.h>

typedef struct ovrLaunchReportFlowResult *ovrLaunchReportFlowResultHandle;

/// Whether the viewer chose to cancel the report flow.
OVRP_PUBLIC_FUNCTION(bool) ovr_LaunchReportFlowResult_GetDidCancel(const ovrLaunchReportFlowResultHandle obj);

OVRP_PUBLIC_FUNCTION(ovrID) ovr_LaunchReportFlowResult_GetUserReportId(const ovrLaunchReportFlowResultHandle obj);

#endif
