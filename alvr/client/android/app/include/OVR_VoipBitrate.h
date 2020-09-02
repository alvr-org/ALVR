// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_VOIP_BITRATE_H
#define OVR_VOIP_BITRATE_H

#include "OVR_Platform_Defs.h"

typedef enum ovrVoipBitrate_ {
  ovrVoipBitrate_Unknown,
  /// Very low audio quality for minimal network usage. This may not give the
  /// full range of Hz for audio, but it will save on network usage.
  ovrVoipBitrate_B16000,
  /// Lower audio quality but also less network usage.
  ovrVoipBitrate_B24000,
  /// This is the default bitrate for voip connections. It should be the best
  /// tradeoff between audio quality and network usage.
  ovrVoipBitrate_B32000,
  /// Higher audio quality at the expense of network usage. Good if there's music
  /// being streamed over the connections
  ovrVoipBitrate_B64000,
  /// Even higher audio quality for music streaming or radio-like quality.
  ovrVoipBitrate_B96000,
  /// At this point the audio quality should be preceptually indistinguishable
  /// from the uncompressed input.
  ovrVoipBitrate_B128000,
} ovrVoipBitrate;

/// Converts an ovrVoipBitrate enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrVoipBitrate_ToString(ovrVoipBitrate value);

/// Converts a string representing an ovrVoipBitrate enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrVoipBitrate) ovrVoipBitrate_FromString(const char* str);

#endif
