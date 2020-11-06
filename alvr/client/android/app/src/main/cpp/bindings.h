#pragma once

extern "C" void decoderInput(long long frameIndex);
extern "C" void decoderOutput(long long frameIndex);

extern "C" long long
initializeNative(void *env, void *instance, void *activity, void *assetManager, void *vrThread,
                 unsigned char ARMode, int initialRefreshRate);
extern "C" void destroyNative(void *env, long long handle);
extern "C" int getLoadingTextureNative(long long handle);
extern "C" int getSurfaceTextureIDNative(long long handle);
extern "C" int getWebViewSurfaceTextureNative(long long handle);
extern "C" void renderNative(long long handle, long long renderedFrameIndex);
extern "C" void renderLoadingNative(long long handle);
extern "C" void sendTrackingInfoNative(void *env, long long handle, void *udpReceiverThread);
extern "C" void sendGuardianInfoNative(void *env, long long handle, void *udpReceiverThread);
extern "C" void sendMicDataNative(void *env, long long handle, void *udpReceiverThread);
extern "C" void onChangeSettingsNative(long long handle, int suspend);
extern "C" void onSurfaceCreatedNative(long long handle, void *surface);
extern "C" void onSurfaceDestroyedNative(long long handle);
extern "C" void onSurfaceChangedNative(long long handle, void *surface);
extern "C" void onResumeNative(long long handle);
extern "C" void onPauseNative(long long handle);
extern "C" unsigned char isVrModeNative(long long handle);
extern "C" void getDeviceDescriptorNative(void *env, long long handle, void *deviceDescriptor);
extern "C" void setFrameGeometryNative(long long handle, int width, int height);
extern "C" void setRefreshRateNative(long long handle, int refreshRate);
extern "C" void setStreamMicNative(long long handle, unsigned char streamMic);
extern "C" void setFFRParamsNative(long long handle, int foveationMode, float foveationStrength,
                                   float foveationShape, float foveationVerticalOffset);
extern "C" void
onHapticsFeedbackNative(long long handle, long long startTime, float amplitude, float duration,
                        float frequency, unsigned char hand);
extern "C" unsigned char getButtonDownNative(long long handle);
extern "C" void onGuardianSyncAckNative(long long handle, long long timestamp);
extern "C" void onGuardianSegmentAckNative(long long handle, long long timestamp, int segmentIndex);

extern "C" long long
initializeSocket(void *env, void *instance, int helloPort, int port, void *deviceName,
                 void *broadcastAddrList, void *refreshRates, int renderWidth, int renderHeight,
                 void *fov, int deviceType, int deviceSubType, int deviceCapabilityFlags,
                 int controllerCapabilityFlags, float ipd);
extern "C" void closeSocket(long long nativeHandle);
extern "C" void
runLoop(void *env, void *instance, long long nativeHandle, void *serverAddress, int serverPort);
extern "C" void interruptNative(long long nativeHandle);
extern "C" unsigned char isConnectedNative(long long nativeHandle);
extern "C" void *getServerAddress(void *env, long long nativeHandle);
extern "C" int getServerPort(long long nativeHandle);
extern "C" void sendNative(long long nativeHandle, long long nativeBuffer, int bufferLength);
extern "C" void setSinkPreparedNative(long long nativeHandle, unsigned char prepared);

extern "C" void setFrameLogEnabled(long long debugFlags);