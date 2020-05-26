// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_SEND_POLICY_H
#define OVR_SEND_POLICY_H

#include "OVR_Platform_Defs.h"

typedef enum ovrSendPolicy_ {
  /// Sends a message using an unreliable data channel (UDP-based). No delivery
  /// or ordering guarantees are provided. Sending will fail unless a connection
  /// to the peer is already established, either via a previous call to
  /// ovr_Net_SendPacket() or an explicit ovr_Net_Connect().
  ///
  /// Ideally, each message should fit into a single packet. Therefore, it is
  /// recommended to keep them under 1200 bytes.
  ovrSend_Unreliable,
  /// Messages are delivered reliably and in order. The networking layer retries
  /// until each message is acknowledged by the peer. Outgoing messages are
  /// buffered until a working connection to the peer is established.
  ovrSend_Reliable,
  ovrSend_Unknown,
} ovrSendPolicy;

/// Converts an ovrSendPolicy enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrSendPolicy_ToString(ovrSendPolicy value);

/// Converts a string representing an ovrSendPolicy enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrSendPolicy) ovrSendPolicy_FromString(const char* str);

#endif
