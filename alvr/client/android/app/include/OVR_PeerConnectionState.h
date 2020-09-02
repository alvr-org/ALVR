// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_PEER_CONNECTION_STATE_H
#define OVR_PEER_CONNECTION_STATE_H

#include "OVR_Platform_Defs.h"

typedef enum ovrPeerConnectionState_ {
  ovrPeerState_Unknown,
  /// Connection to the peer is established.
  ovrPeerState_Connected,
  /// A timeout expired while attempting to (re)establish a connection. This can
  /// happen if peer is unreachable or rejected the connection.
  ovrPeerState_Timeout,
  /// Connection to the peer is closed. A connection transitions into this state
  /// when it is explicitly closed by either the local or remote peer calling
  /// ovr_Net_Close(). It also enters this state if the remote peer no longer
  /// responds to our keep-alive probes.
  ovrPeerState_Closed,
} ovrPeerConnectionState;

/// Converts an ovrPeerConnectionState enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrPeerConnectionState_ToString(ovrPeerConnectionState value);

/// Converts a string representing an ovrPeerConnectionState enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrPeerConnectionState) ovrPeerConnectionState_FromString(const char* str);

#endif
