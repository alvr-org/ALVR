// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_ABUSE_REPORT_TYPE_H
#define OVR_ABUSE_REPORT_TYPE_H

#include "OVR_Platform_Defs.h"

typedef enum ovrAbuseReportType_ {
  ovrAbuseReportType_Unknown,
  ovrAbuseReportType_Object,
  ovrAbuseReportType_User,
} ovrAbuseReportType;

/// Converts an ovrAbuseReportType enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrAbuseReportType_ToString(ovrAbuseReportType value);

/// Converts a string representing an ovrAbuseReportType enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrAbuseReportType) ovrAbuseReportType_FromString(const char* str);

#endif
