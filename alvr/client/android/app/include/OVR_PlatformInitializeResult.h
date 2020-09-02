// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_PLATFORM_INITIALIZE_RESULT_H
#define OVR_PLATFORM_INITIALIZE_RESULT_H

#include "OVR_Platform_Defs.h"

/// Describes the various results possible when attempting to initialize the
/// platform. Anything other than ovrPlatformInitialize_Success should be
/// considered a fatal error with respect to using the platform, as the
/// platform is not guaranteed to be legitimate or work correctly.
typedef enum ovrPlatformInitializeResult_ {
  ovrPlatformInitialize_Success = 0,
  ovrPlatformInitialize_Uninitialized = -1,
  ovrPlatformInitialize_PreLoaded = -2,
  ovrPlatformInitialize_FileInvalid = -3,
  ovrPlatformInitialize_SignatureInvalid = -4,
  ovrPlatformInitialize_UnableToVerify = -5,
  ovrPlatformInitialize_VersionMismatch = -6,
  ovrPlatformInitialize_Unknown = -7,
  ovrPlatformInitialize_InvalidCredentials = -8,
  ovrPlatformInitialize_NotEntitled = -9,
} ovrPlatformInitializeResult;

/// Converts an ovrPlatformInitializeResult enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrPlatformInitializeResult_ToString(ovrPlatformInitializeResult value);

/// Converts a string representing an ovrPlatformInitializeResult enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrPlatformInitializeResult) ovrPlatformInitializeResult_FromString(const char* str);

#endif
