// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_NETWORKING_H
#define OVR_NETWORKING_H

#include "OVR_Packet.h"
#include "OVR_PeerConnectionState.h"
#include "OVR_Platform_Defs.h"
#include "OVR_SendPolicy.h"
#include "OVR_Types.h"
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

/// Allow `peerID` to establish a peer-to-peer connection to this host. Call
/// this after receiving ovrNotification_Networking_PeerConnectRequest. This
/// function is a no-op if there are no pending connection attempts from
/// peerID.
///
/// This function can be safely called from any thread.
OVRP_PUBLIC_FUNCTION(void) ovr_Net_Accept(ovrID peerID);

/// Automatically accept all current and future connection attempts from
/// members of the current room. Note that the room has to be created or joined
/// by calling one of the existing room/matchmaking functions. Returns false if
/// the user currently isn't in a room.
///
/// This function can be safely called from any thread.
OVRP_PUBLIC_FUNCTION(bool) ovr_Net_AcceptForCurrentRoom();

/// Destroy the connection to peerID, if one exists. Note that in most cases
/// this is not needed, as the library manages the pool of connections and
/// discards unused ones.
///
/// This function can be safely called from any thread.
OVRP_PUBLIC_FUNCTION(void) ovr_Net_Close(ovrID peerID);

/// Close the connection to everyone in the current room. This is typically
/// called before leaving the room. Can be called from any thread.
OVRP_PUBLIC_FUNCTION(void) ovr_Net_CloseForCurrentRoom();

/// Connects to the peer with the specified user ID. This function returns
/// immediately. Once the connection is established, a
/// ovrNotification_Networking_ConnectionStateChange message will be enqueued.
///
/// Can be called from any thread.
///
/// Note that ovr_Net_SendPacket() implicitly connects. However, it does not
/// buffer messages in unreliable mode. ovr_Net_Connect() allows the
/// application to delay sending messages until an actual connection is
/// established.
OVRP_PUBLIC_FUNCTION(void) ovr_Net_Connect(ovrID peerID);

/// Returns true only when there is an open connection to peerID. Can be called
/// from any thread.
OVRP_PUBLIC_FUNCTION(bool) ovr_Net_IsConnected(ovrID peerID);

/// Ping the user with the given ID. Once the request completes, a
/// ovrNotification_Networking_PingResult message is enqueued. Can be called
/// from any thread.
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Net_Ping(ovrID peerID);

/// Read the next incoming packet. Returns null when no more packets are
/// available. Returned handle points to an object representing data read from
/// the network. Ownership of that object is transferred to the application;
/// use ovr_Packet_Free() to release memory.
///
/// For example:
///
///   ovrPacketHandle packet;
///   while (packet = ovr_Net_ReadPacket()) {
///     ovrID sender = ovr_Packet_GetSender(packet);
///     // dispatch packet
///     ovr_Packet_Free(packet);
///   }
///
/// This function can be safely called from any thread.
OVRP_PUBLIC_FUNCTION(ovrPacketHandle) ovr_Net_ReadPacket();

/// Send a sequence of bytes to another user. The length must be less than or
/// equal to the allocated length of bytes. A new connection to userID will be
/// established (asynchronously) unless one already exists.
///
/// Depending on the policy, this message might be buffered until a valid
/// connection to the peer exists. The function returns false if the packet
/// can't be enqueued for sending (e.g., there's not enough memory) or the
/// policy prohibits buffering. See ovrSendPolicy and ovr_Net_Connect().
///
/// This function can be safely called from any thread.
OVRP_PUBLIC_FUNCTION(bool) ovr_Net_SendPacket(ovrID userID, size_t length, const void *bytes, ovrSendPolicy policy);

/// Sends a packet to all members of the room, excluding the currently logged
/// in user. Note that the room has to be created or joined by calling one of
/// the existing room/matchmaking functions, with subscribe_to_updates enabled.
/// See ovr_Net_SendPacket() for a description of parameters. This function
/// returns false if the user currently isn't in a room.
///
/// This function can be safely called from any thread.
OVRP_PUBLIC_FUNCTION(bool) ovr_Net_SendPacketToCurrentRoom(size_t length, const void *bytes, ovrSendPolicy policy);


#endif
