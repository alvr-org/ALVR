// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_LIVESTREAMING_MICROPHONE_STATUS_H
#define OVR_LIVESTREAMING_MICROPHONE_STATUS_H

#include "OVR_Platform_Defs.h"

typedef enum ovrLivestreamingMicrophoneStatus_ {
  ovrLivestreamingMicrophoneStatus_Unknown,
  ovrLivestreamingMicrophoneStatus_MicrophoneOn,
  ovrLivestreamingMicrophoneStatus_MicrophoneOff,
} ovrLivestreamingMicrophoneStatus;

/// Converts an ovrLivestreamingMicrophoneStatus enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrLivestreamingMicrophoneStatus_ToString(ovrLivestreamingMicrophoneStatus value);

/// Converts a string representing an ovrLivestreamingMicrophoneStatus enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrLivestreamingMicrophoneStatus) ovrLivestreamingMicrophoneStatus_FromString(const char* str);

#endif
