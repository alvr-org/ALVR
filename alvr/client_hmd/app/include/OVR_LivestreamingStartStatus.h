// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_LIVESTREAMING_START_STATUS_H
#define OVR_LIVESTREAMING_START_STATUS_H

#include "OVR_Platform_Defs.h"

typedef enum ovrLivestreamingStartStatus_ {
  ovrLivestreamingStartStatus_Success = 1,
  ovrLivestreamingStartStatus_Unknown = 0,
  ovrLivestreamingStartStatus_NoPackageSet = -1,
  ovrLivestreamingStartStatus_NoFbConnect = -2,
  ovrLivestreamingStartStatus_NoSessionId = -3,
  ovrLivestreamingStartStatus_MissingParameters = -4,
} ovrLivestreamingStartStatus;

/// Converts an ovrLivestreamingStartStatus enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrLivestreamingStartStatus_ToString(ovrLivestreamingStartStatus value);

/// Converts a string representing an ovrLivestreamingStartStatus enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrLivestreamingStartStatus) ovrLivestreamingStartStatus_FromString(const char* str);

#endif
