// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_USERREPORTID_H
#define OVR_USERREPORTID_H

#include "OVR_Platform_Defs.h"
#include "OVR_Types.h"
#include <stdbool.h>

typedef struct ovrUserReportID *ovrUserReportIDHandle;

/// Whether the viewer chose to cancel the report flow.
OVRP_PUBLIC_FUNCTION(bool) ovr_UserReportID_GetDidCancel(const ovrUserReportIDHandle obj);

OVRP_PUBLIC_FUNCTION(ovrID) ovr_UserReportID_GetID(const ovrUserReportIDHandle obj);

#endif
