// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_NETWORKINGPEER_H
#define OVR_NETWORKINGPEER_H

#include "OVR_Platform_Defs.h"
#include "OVR_PeerConnectionState.h"
#include "OVR_Types.h"

typedef struct ovrNetworkingPeer *ovrNetworkingPeerHandle;

OVRP_PUBLIC_FUNCTION(ovrID)                  ovr_NetworkingPeer_GetID(const ovrNetworkingPeerHandle obj);
OVRP_PUBLIC_FUNCTION(ovrPeerConnectionState) ovr_NetworkingPeer_GetState(const ovrNetworkingPeerHandle obj);

#endif
