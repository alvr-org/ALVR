// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_ABUSEREPORTRECORDING_H
#define OVR_ABUSEREPORTRECORDING_H

#include "OVR_Platform_Defs.h"

typedef struct ovrAbuseReportRecording *ovrAbuseReportRecordingHandle;

/// A UUID associated with the Abuse Report recording.
OVRP_PUBLIC_FUNCTION(const char *) ovr_AbuseReportRecording_GetRecordingUuid(const ovrAbuseReportRecordingHandle obj);


#endif
