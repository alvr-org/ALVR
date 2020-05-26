// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_MICROPHONE_H
#define OVR_MICROPHONE_H

#include "OVR_Platform_Defs.h"
#include "OVR_Types.h"
#include <stddef.h>
#include <stdint.h>

typedef struct ovrMicrophone *ovrMicrophoneHandle;

/// Returns the minimum number of samples available to be read. This function
/// is inherently racy, it is possible that more samples may be returned by the
/// next call to getPCM. This function is only implemented on Android. Windows
/// will always return 0.
OVRP_PUBLIC_FUNCTION(size_t) ovr_Microphone_GetNumSamplesAvailable(const ovrMicrophoneHandle obj);

/// Returns the size of the internal ringbuffer used by the microhone in
/// elements. This size is the maximum number of elements that can ever be
/// returned by ovr_Microphone_GetPCM()*.
///
/// This function can be safely called from any thread.
OVRP_PUBLIC_FUNCTION(size_t) ovr_Microphone_GetOutputBufferMaxSize(const ovrMicrophoneHandle obj);

/// Gets all available samples of microphone data and copies it into
/// outputBuffer. The microphone will generate data at roughly the rate of 480
/// samples per 10ms. The data format is 16 bit fixed point 48khz mono.
///
/// This function can be safely called from any thread.
OVRP_PUBLIC_FUNCTION(size_t) ovr_Microphone_GetPCM(const ovrMicrophoneHandle obj, int16_t *outputBuffer, size_t outputBufferNumElements);

/// Gets all available samples of microphone data and copies it into
/// outputBuffer. The microphone will generate data at roughly the rate of 480
/// samples per 10ms. The data format is 32 bit floating point 48khz mono.
///
/// This function can be safely called from any thread.
OVRP_PUBLIC_FUNCTION(size_t) ovr_Microphone_GetPCMFloat(const ovrMicrophoneHandle obj, float *outputBuffer, size_t outputBufferNumElements);

/// DEPRECATED: Use ovr_Microphone_GetPCMFloat() instead.
///
/// Gets all available samples of microphone data and copies it into
/// outputBuffer. The microphone will generate data at roughly the rate of 480
/// samples per 10ms. The data format is 32 bit floating point 48khz mono.
///
/// This function can be safely called from any thread.
OVRP_PUBLIC_FUNCTION(size_t) ovr_Microphone_ReadData(const ovrMicrophoneHandle obj, float *outputBuffer, size_t outputBufferSize);

/// Indicates that the caller is fine with a certain delay in the delivery of
/// recorded audio frames. Setting this to a low value will reduce the latency
/// at the cost of efficiency. Note that this is only a hint; the actual
/// implementation may ignore it.
OVRP_PUBLIC_FUNCTION(void) ovr_Microphone_SetAcceptableRecordingDelayHint(const ovrMicrophoneHandle obj, size_t delayMs);

/// Register a callback that will be called whenever audio data is available
/// for the microphone.
OVRP_PUBLIC_FUNCTION(void) ovr_Microphone_SetAudioDataAvailableCallback(const ovrMicrophoneHandle obj, MicrophoneDataAvailableCallback cb, void *userData);

/// Starts microphone recording. After this is called pcm data can be extracted
/// using ovr_Microphone_GetPCM().
///
/// This function can be safely called from any thread.
OVRP_PUBLIC_FUNCTION(void) ovr_Microphone_Start(const ovrMicrophoneHandle obj);

/// Stops microphone recording.
///
/// This function can be safely called from any thread.
OVRP_PUBLIC_FUNCTION(void) ovr_Microphone_Stop(const ovrMicrophoneHandle obj);


#endif
