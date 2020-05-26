// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_VOIP_DTX_STATE_H
#define OVR_VOIP_DTX_STATE_H

#include "OVR_Platform_Defs.h"

typedef enum ovrVoipDtxState_ {
  ovrVoipDtxState_Unknown,
  ovrVoipDtxState_Enabled,
  ovrVoipDtxState_Disabled,
} ovrVoipDtxState;

/// Converts an ovrVoipDtxState enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrVoipDtxState_ToString(ovrVoipDtxState value);

/// Converts a string representing an ovrVoipDtxState enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrVoipDtxState) ovrVoipDtxState_FromString(const char* str);

#endif
