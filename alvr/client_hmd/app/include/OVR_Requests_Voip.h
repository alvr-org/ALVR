// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_REQUESTS_VOIP_H
#define OVR_REQUESTS_VOIP_H

#include "OVR_Types.h"
#include "OVR_Platform_Defs.h"


/// Sets whether SystemVoip should be suppressed so that this app's Voip can
/// use the mic and play incoming Voip audio.
///
/// A message with type ::ovrMessage_Voip_SetSystemVoipSuppressed will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrSystemVoipStateHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetSystemVoipState().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Voip_SetSystemVoipSuppressed(bool suppressed);

#endif
