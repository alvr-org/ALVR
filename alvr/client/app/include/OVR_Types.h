#ifndef OVR_TYPES_H
#define OVR_TYPES_H

#include "OVR_KeyValuePairType.h"
#include "OVR_MatchmakingCriterionImportance.h"
#include "OVR_VoipSampleRate.h"
#include "OVR_MediaContentType.h"

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/// Represents a single state change in the platform such as the
/// response to a request, or some new information from the backend.
typedef uint64_t ovrRequest;

// Represents an RequestID that can used as a default.
// We guarantee that no valid Request ID will equal invalidRequestID
const uint64_t invalidRequestID = 0;

// Represents an invalid MessageID that can used as a default.
// We guarantee that no valid Message ID will equal invalidMessageID
const uint64_t invalidMessageID = 0;

typedef struct {
  const char* key;
  ovrKeyValuePairType valueType;

  const char* stringValue;
  int intValue;
  double doubleValue;

} ovrKeyValuePair;

/// Helper function for making an int ovrKeyValuePair.
///
/// For example, ovrKeyValuePair_makeInt("key", 1);
ovrKeyValuePair ovrKeyValuePair_makeInt(const char* key, int value);

/// Helper function for making a double ovrKeyValuePair.
///
/// For example, ovrKeyValuePair_makeDouble("key", 1.1);
ovrKeyValuePair ovrKeyValuePair_makeDouble(const char* key, double value);

/// Helper function for making a string ovrKeyValuePair.
///
/// For example, ovrKeyValuePair_makeString("key", "value");
ovrKeyValuePair ovrKeyValuePair_makeString(const char* key, const char* value);

typedef struct {
  const char* key;
  ovrMatchmakingCriterionImportance importance;

  ovrKeyValuePair* parameterArray;
  unsigned int parameterArrayCount;

} ovrMatchmakingCriterion;

typedef struct {
  ovrKeyValuePair* customQueryDataArray;
  unsigned int customQueryDataArrayCount;

  ovrMatchmakingCriterion* customQueryCriterionArray;
  unsigned int customQueryCriterionArrayCount;
} ovrMatchmakingCustomQueryData;

/// A unique identifier for some entity in the system (user, room, etc).
///
typedef uint64_t ovrID;

/// Convert a string into an ovrID.  Returns false if the input is
/// malformed (either out of range, or not an integer).
bool ovrID_FromString(ovrID* outId, const char* inId);

/// Convert an ID back into a string.  This function round trips with
/// ovrID_FromString().  Note: the id format may change in the future.
/// Developers should not rely on the string representation being an
/// integer.
///
/// Length of outParam should be > 20.
bool ovrID_ToString(char* outParam, size_t bufferLength, ovrID id);

typedef void (*LogFunctionPtr)(const char*, const char*);
extern LogFunctionPtr DoLogging;

/// Callback used by the Voip subsystem for audio filtering
///
typedef void (
    *VoipFilterCallback)(int16_t pcmData[], size_t pcmDataLength, int frequency, int numChannels);

/// Callback used by the ovrMicrophone class to signal that data is available
///
typedef void (*MicrophoneDataAvailableCallback)(void*);

typedef struct {
  float x;
  float y;
  float z;
} ovrNetSyncVec3;

#ifdef __cplusplus
}
#endif

#endif
