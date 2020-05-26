// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_PACKET_H
#define OVR_PACKET_H

#include "OVR_Platform_Defs.h"
#include "OVR_SendPolicy.h"
#include "OVR_Types.h"
#include <stddef.h>

typedef struct ovrPacket *ovrPacketHandle;

OVRP_PUBLIC_FUNCTION(void)          ovr_Packet_Free(const ovrPacketHandle obj);
OVRP_PUBLIC_FUNCTION(const void *)  ovr_Packet_GetBytes(const ovrPacketHandle obj);
OVRP_PUBLIC_FUNCTION(ovrSendPolicy) ovr_Packet_GetSendPolicy(const ovrPacketHandle obj);
OVRP_PUBLIC_FUNCTION(ovrID)         ovr_Packet_GetSenderID(const ovrPacketHandle obj);
OVRP_PUBLIC_FUNCTION(size_t)        ovr_Packet_GetSize(const ovrPacketHandle obj);

#endif
