#pragma once

extern "C" void decoderInput(long long frameIndex);
extern "C" void decoderOutput(long long frameIndex);

extern "C" void
initializeNative(void *env, void *instance, void *activity, void *assetManager, void *vrThread,
                 unsigned char ARMode, int initialRefreshRate);
extern "C" void destroyNative(void *env);
extern "C" int getLoadingTextureNative();
extern "C" int getSurfaceTextureIDNative();
extern "C" int getWebViewSurfaceTextureNative();
extern "C" void renderNative(long long renderedFrameIndex);
extern "C" void renderLoadingNative();
extern "C" void sendTrackingInfoNative(void *env, void *udpReceiverThread);
extern "C" void sendGuardianInfoNative(void *env, void *udpReceiverThread);
extern "C" void sendMicDataNative(void *env, void *udpReceiverThread);
extern "C" void onChangeSettingsNative(int suspend);
extern "C" void onSurfaceCreatedNative(void *surface);
extern "C" void onSurfaceDestroyedNative();
extern "C" void onSurfaceChangedNative(void *surface);
extern "C" void onResumeNative();
extern "C" void onPauseNative();
extern "C" unsigned char isVrModeNative();
extern "C" void getDeviceDescriptorNative(void *env, void *deviceDescriptor);
extern "C" void setFrameGeometryNative(int width, int height);
extern "C" void setRefreshRateNative(int refreshRate);
extern "C" void setStreamMicNative(unsigned char streamMic);
extern "C" void setFFRParamsNative(int foveationMode, float foveationStrength,
                                   float foveationShape, float foveationVerticalOffset);
extern "C" void
onHapticsFeedbackNative(long long startTime, float amplitude, float duration,
                        float frequency, unsigned char hand);
extern "C" void onGuardianSyncAckNative(long long timestamp);
extern "C" void onGuardianSegmentAckNative(long long timestamp, int segmentIndex);

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