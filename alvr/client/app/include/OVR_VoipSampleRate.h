// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_VOIP_SAMPLE_RATE_H
#define OVR_VOIP_SAMPLE_RATE_H

#include "OVR_Platform_Defs.h"

typedef enum ovrVoipSampleRate_ {
  ovrVoipSampleRate_Unknown,
  ovrVoipSampleRate_HZ24000,
  ovrVoipSampleRate_HZ44100,
  ovrVoipSampleRate_HZ48000,
} ovrVoipSampleRate;

/// Converts an ovrVoipSampleRate enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrVoipSampleRate_ToString(ovrVoipSampleRate value);

/// Converts a string representing an ovrVoipSampleRate enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrVoipSampleRate) ovrVoipSampleRate_FromString(const char* str);

#endif
