// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_SERVICE_PROVIDER_H
#define OVR_SERVICE_PROVIDER_H

#include "OVR_Platform_Defs.h"

typedef enum ovrServiceProvider_ {
  ovrServiceProvider_Unknown,
  ovrServiceProvider_Dropbox,
  ovrServiceProvider_Facebook,
  ovrServiceProvider_Google,
  ovrServiceProvider_Instagram,
  ovrServiceProvider_RemoteMedia,
} ovrServiceProvider;

/// Converts an ovrServiceProvider enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrServiceProvider_ToString(ovrServiceProvider value);

/// Converts a string representing an ovrServiceProvider enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrServiceProvider) ovrServiceProvider_FromString(const char* str);

#endif
