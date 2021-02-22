#pragma once

//todo:
/*
The following structs must be marked #[cfg_attr(unix, repr(packed))] to work around OpenVR's broken ABI on Linux/OSX:

VRControllerState_t
RenderModel_TextureMap_t
RenderModel_t
VREvent_t
*/

// include modified header
#include "openvr_driver.h"

using namespace vr;

// EXPAND is sometimes necessary for MSVC preprocessor
#define EXPAND(x) x

#define NARGS_SEQ(_1, _2, _3, _4, _5, _6, _7, _8, _9, N, ...) N
#define NARGS_M1(...) EXPAND(NARGS_SEQ(__VA_ARGS__, 8, 7, 6, 5, 4, 3, 2, 1, 0))

#define PRIMITIVE_CAT(x, y) x##y
#define CAT(x, y) PRIMITIVE_CAT(x, y)

#define NAME(name, ...) name
#define TYPES(name, ...) __VA_ARGS__

#define COMMA_0
#define COMMA_1 ,
#define COMMA_2 ,
#define COMMA_3 ,
#define COMMA_4 ,
#define COMMA_5 ,
#define COMMA_6 ,
#define COMMA_7 ,
#define COMMA_8 ,
#define COMMA(...) CAT(COMMA_, NARGS_M1(__VA_ARGS__))

#define PARAMETERS_0(_)
#define PARAMETERS_1(_, t1) t1 _1
#define PARAMETERS_2(_, t1, t2) t1 _1, t2 _2
#define PARAMETERS_3(_, t1, t2, t3) t1 _1, t2 _2, t3 _3
#define PARAMETERS_4(_, t1, t2, t3, t4) t1 _1, t2 _2, t3 _3, t4 _4
#define PARAMETERS_5(_, t1, t2, t3, t4, t5) t1 _1, t2 _2, t3 _3, t4 _4, t5 _5
#define PARAMETERS_6(_, t1, t2, t3, t4, t5, t6) t1 _1, t2 _2, t3 _3, t4 _4, t5 _5, t6 _6
#define PARAMETERS_7(_, t1, t2, t3, t4, t5, t6, t7) t1 _1, t2 _2, t3 _3, t4 _4, t5 _5, t6 _6, t7 _7
#define PARAMETERS_8(_, t1, t2, t3, t4, t5, t6, t7, t8) t1 _1, t2 _2, t3 _3, t4 _4, t5 _5, t6 _6, t7 _7, t8 _8
#define PARAMETERS(...) EXPAND(CAT(PARAMETERS_, NARGS_M1(__VA_ARGS__))(__VA_ARGS__))

#define ARGUMENTS_0
#define ARGUMENTS_1 _1
#define ARGUMENTS_2 _1, _2
#define ARGUMENTS_3 _1, _2, _3
#define ARGUMENTS_4 _1, _2, _3, _4
#define ARGUMENTS_5 _1, _2, _3, _4, _5
#define ARGUMENTS_6 _1, _2, _3, _4, _5, _6
#define ARGUMENTS_7 _1, _2, _3, _4, _5, _6, _7
#define ARGUMENTS_8 _1, _2, _3, _4, _5, _6, _7, _8
#define ARGUMENTS(...) CAT(ARGUMENTS_, NARGS_M1(__VA_ARGS__))

////////////////////////////////////////////////////////////////////////////////////////////

#define CLASS_C_INTERFACE(prefix, ClassName, interface)                     \
    class ClassName final : public prefix##ClassName                        \
    {                                                                       \
    public:                                                                 \
        ClassName(ClassName##Callbacks callbacks) : callbacks(callbacks) {} \
                                                                            \
    private:                                                                \
        ClassName##Callbacks callbacks;                                     \
        interface;                                                          \
    };                                                                      \
                                                                            \
    ClassName *vrCreate##ClassName(ClassName##Callbacks callbacks)          \
    {                                                                       \
        return new ClassName(callbacks);                                    \
    }                                                                       \
                                                                            \
    void vrDestroy##ClassName(ClassName **instance)                         \
    {                                                                       \
        delete *instance;                                                   \
    }

#define METHOD(return_type, ...) EXPAND(                     \
    return_type NAME(__VA_ARGS__)(PARAMETERS(__VA_ARGS__)) { \
        return callbacks.NAME(__VA_ARGS__)(                  \
            callbacks.context                                \
                COMMA(__VA_ARGS__)                           \
                    ARGUMENTS(__VA_ARGS__));                 \
    })

struct TrackedDeviceServerDriverCallbacks
{
    void *context;
    EVRInitError (*Activate)(void *, uint32_t);
    void (*Deactivate)(void *);
    void (*EnterStandby)(void *);
    void *(*GetComponent)(void *, const char *);
    void (*DebugRequest)(void *, const char *, char *, uint32_t);
    DriverPose_t (*GetPose)(void *);
};

CLASS_C_INTERFACE(
    I, TrackedDeviceServerDriver,
    METHOD(EVRInitError, Activate, uint32_t);
    METHOD(void, Deactivate);
    METHOD(void, EnterStandby);
    METHOD(void *, GetComponent, const char *);
    METHOD(void, DebugRequest, const char *, char *, uint32_t);
    METHOD(DriverPose_t, GetPose););

struct DisplayComponentCallbacks
{
    void *context;
    void (*GetWindowBounds)(void *, int32_t *, int32_t *, uint32_t *, uint32_t *);
    bool (*IsDisplayOnDesktop)(void *);
    bool (*IsDisplayRealDisplay)(void *);
    void (*GetRecommendedRenderTargetSize)(void *, uint32_t *, uint32_t *);
    void (*GetEyeOutputViewport)(void *, EVREye, uint32_t *, uint32_t *, uint32_t *, uint32_t *);
    void (*GetProjectionRaw)(void *, EVREye, float *, float *, float *, float *);
    DistortionCoordinates_t (*ComputeDistortion)(void *, EVREye, float, float);
};

CLASS_C_INTERFACE(
    IVR, DisplayComponent,
    METHOD(void, GetWindowBounds, int32_t *, int32_t *, uint32_t *, uint32_t *);
    METHOD(bool, IsDisplayOnDesktop);
    METHOD(bool, IsDisplayRealDisplay);
    METHOD(void, GetRecommendedRenderTargetSize, uint32_t *, uint32_t *);
    METHOD(void, GetEyeOutputViewport, EVREye, uint32_t *, uint32_t *, uint32_t *, uint32_t *);
    METHOD(void, GetProjectionRaw, EVREye, float *, float *, float *, float *);
    METHOD(DistortionCoordinates_t, ComputeDistortion, EVREye, float, float););

struct DriverDirectModeComponentCallbacks
{
    void *context;
    void (*CreateSwapTextureSet)(void *,
                                 uint32_t,
                                 const IVRDriverDirectModeComponent::SwapTextureSetDesc_t *,
                                 SharedTextureHandle_t (*)[3]);
    void (*DestroySwapTextureSet)(void *, SharedTextureHandle_t);
    void (*DestroyAllSwapTextureSets)(void *, uint32_t);
    void (*GetNextSwapTextureSetIndex)(void *, const SharedTextureHandle_t (&)[2], uint32_t (*)[2]);
    void (*SubmitLayer)(void *,
                        const IVRDriverDirectModeComponent::SubmitLayerPerEye_t (&)[2],
                        const HmdMatrix34_t *);
    void (*Present)(void *, SharedTextureHandle_t);
    void (*PostPresent)(void *);
    void (*GetFrameTiming)(void *, DriverDirectMode_FrameTiming *);
};

CLASS_C_INTERFACE(
    IVR, DriverDirectModeComponent,

    void CreateSwapTextureSet(uint32_t unPid,
                              const SwapTextureSetDesc_t *pSwapTextureSetDesc,
                              SharedTextureHandle_t (*pSharedTextureHandles)[3]) {
        callbacks.CreateSwapTextureSet(callbacks.context,
                                       unPid,
                                       pSwapTextureSetDesc,
                                       pSharedTextureHandles);
    }

    METHOD(void, DestroySwapTextureSet, SharedTextureHandle_t);
    METHOD(void, DestroyAllSwapTextureSets, uint32_t);

    void GetNextSwapTextureSetIndex(SharedTextureHandle_t sharedTextureHandles[2],
                                    uint32_t (*pIndices)[2]) {
        callbacks.GetNextSwapTextureSetIndex(
            callbacks.context, (const SharedTextureHandle_t(&)[2])sharedTextureHandles, pIndices);
    }

    void SubmitLayer(const SubmitLayerPerEye_t (&perEye)[2], const HmdMatrix34_t *pPose) {
        callbacks.SubmitLayer(callbacks.context, perEye, pPose);
    }

    METHOD(void, Present, SharedTextureHandle_t);
    METHOD(void, PostPresent);
    METHOD(void, GetFrameTiming, DriverDirectMode_FrameTiming *););

struct CameraVideoSinkCallbackCallbacks
{
    void *context;
    void (*OnCameraVideoSinkCallback)(void *);
};

CLASS_C_INTERFACE(
    I, CameraVideoSinkCallback,
    METHOD(void, OnCameraVideoSinkCallback););

struct CameraComponentCallbacks
{
    void *context;
    bool (*GetCameraFrameDimensions)(void *, ECameraVideoStreamFormat, uint32_t *, uint32_t *);
    bool (*GetCameraFrameBufferingRequirements)(void *, int *, uint32_t *);
    bool (*SetCameraFrameBuffering)(void *, int, void **, uint32_t);
    bool (*SetCameraVideoStreamFormat)(void *, ECameraVideoStreamFormat);
    ECameraVideoStreamFormat (*GetCameraVideoStreamFormat)(void *);
    bool (*StartVideoStream)(void *);
    void (*StopVideoStream)(void *);
    bool (*IsVideoStreamActive)(void *, bool *, float *);
    const CameraVideoStreamFrame_t *(*GetVideoStreamFrame)(void *);
    void (*ReleaseVideoStreamFrame)(void *, const CameraVideoStreamFrame_t *);
    bool (*SetAutoExposure)(void *, bool);
    bool (*PauseVideoStream)(void *);
    bool (*ResumeVideoStream)(void *);
    bool (*GetCameraDistortion)(void *, uint32_t, float, float, float *, float *);
    bool (*GetCameraProjection)(void *,
                                uint32_t,
                                EVRTrackedCameraFrameType,
                                float,
                                float,
                                HmdMatrix44_t *);
    bool (*SetFrameRate)(void *, int, int);
    bool (*SetCameraVideoSinkCallback)(void *, ICameraVideoSinkCallback *);
    bool (*GetCameraCompatibilityMode)(void *, ECameraCompatibilityMode *);
    bool (*SetCameraCompatibilityMode)(void *, ECameraCompatibilityMode);
    bool (*GetCameraFrameBounds)(void *,
                                 EVRTrackedCameraFrameType,
                                 uint32_t *,
                                 uint32_t *,
                                 uint32_t *,
                                 uint32_t *);
    bool (*GetCameraIntrinsics)(void *,
                                uint32_t,
                                EVRTrackedCameraFrameType,
                                HmdVector2_t *,
                                HmdVector2_t *,
                                EVRDistortionFunctionType *,
                                double[k_unMaxDistortionFunctionParameters]);
};

CLASS_C_INTERFACE(
    IVR, CameraComponent,
    METHOD(bool, GetCameraFrameDimensions, ECameraVideoStreamFormat, uint32_t *, uint32_t *);
    METHOD(bool, GetCameraFrameBufferingRequirements, int *, uint32_t *);
    METHOD(bool, SetCameraFrameBuffering, int, void **, uint32_t);
    METHOD(bool, SetCameraVideoStreamFormat, ECameraVideoStreamFormat);
    METHOD(ECameraVideoStreamFormat, GetCameraVideoStreamFormat);
    METHOD(bool, StartVideoStream);
    METHOD(void, StopVideoStream);
    METHOD(bool, IsVideoStreamActive, bool *, float *);
    METHOD(const CameraVideoStreamFrame_t *, GetVideoStreamFrame);
    METHOD(void, ReleaseVideoStreamFrame, const CameraVideoStreamFrame_t *);
    METHOD(bool, SetAutoExposure, bool);
    METHOD(bool, PauseVideoStream);
    METHOD(bool, ResumeVideoStream);
    METHOD(bool, GetCameraDistortion, uint32_t, float, float, float *, float *);
    METHOD(bool, GetCameraProjection,
           uint32_t,
           EVRTrackedCameraFrameType,
           float,
           float,
           HmdMatrix44_t *);
    METHOD(bool, SetFrameRate, int, int);
    METHOD(bool, SetCameraVideoSinkCallback, ICameraVideoSinkCallback *);
    METHOD(bool, GetCameraCompatibilityMode, ECameraCompatibilityMode *);
    METHOD(bool, SetCameraCompatibilityMode, ECameraCompatibilityMode);
    METHOD(bool,
           GetCameraFrameBounds,
           EVRTrackedCameraFrameType,
           uint32_t *,
           uint32_t *,
           uint32_t *,
           uint32_t *);

    bool GetCameraIntrinsics(uint32_t nCameraIndex,
                             EVRTrackedCameraFrameType eFrameType,
                             HmdVector2_t *pFocalLength,
                             HmdVector2_t *pCenter,
                             EVRDistortionFunctionType *peDistortionType,
                             double rCoefficients[k_unMaxDistortionFunctionParameters]) {
        return callbacks.GetCameraIntrinsics(callbacks.context,
                                             nCameraIndex,
                                             eFrameType,
                                             pFocalLength,
                                             pCenter,
                                             peDistortionType,
                                             rCoefficients);
    });

struct ServerTrackedDeviceProviderCallbacks
{
    void *context;
    EVRInitError (*Init)(void *, IVRDriverContext *);
    void (*Cleanup)(void *);
    const char *const *(*GetInterfaceVersions)(void *);
    void (*RunFrame)(void *);
    bool (*ShouldBlockStandbyMode)(void *);
    void (*EnterStandby)(void *);
    void (*LeaveStandby)(void *);
};

CLASS_C_INTERFACE(
    I, ServerTrackedDeviceProvider,
    METHOD(EVRInitError, Init, IVRDriverContext *);
    METHOD(void, Cleanup);
    METHOD(const char *const *, GetInterfaceVersions);
    METHOD(void, RunFrame);
    METHOD(bool, ShouldBlockStandbyMode);
    METHOD(void, EnterStandby);
    METHOD(void, LeaveStandby););

struct WatchdogProviderCallbacks
{
    void *context;
    EVRInitError (*Init)(void *, IVRDriverContext *);
    void (*Cleanup)(void *);
};

CLASS_C_INTERFACE(
    IVR, WatchdogProvider,
    METHOD(EVRInitError, Init, IVRDriverContext *);
    METHOD(void, Cleanup););

struct CompositorPluginProviderCallbacks
{
    void *context;
    EVRInitError (*Init)(void *, IVRDriverContext *);
    void (*Cleanup)(void *);
    const char *const *(*GetInterfaceVersions)(void *);
    void *(*GetComponent)(void *, const char *);
};

CLASS_C_INTERFACE(
    IVR, CompositorPluginProvider,
    METHOD(EVRInitError, Init, IVRDriverContext *);
    METHOD(void, Cleanup);
    METHOD(const char *const *, GetInterfaceVersions);
    METHOD(void *, GetComponent, const char *););

struct VirtualDisplayCallbacks
{
    void *context;
    void (*Present)(void *, const PresentInfo_t *, uint32_t);
    void (*WaitForPresent)(void *);
    bool (*GetTimeSinceLastVsync)(void *, float *, uint64_t *);
};

CLASS_C_INTERFACE(
    IVR, VirtualDisplay,
    METHOD(void, Present, const PresentInfo_t *, uint32_t);
    METHOD(void, WaitForPresent);
    METHOD(bool, GetTimeSinceLastVsync, float *, uint64_t *););

////////////////////////////////////////////////////////////////////////////////////////////////

#define FORWARD_FN(callee_class, return_type, fn_prefix, ...) EXPAND(        \
    return_type CAT(fn_prefix, NAME(__VA_ARGS__))(PARAMETERS(__VA_ARGS__)) { \
        return callee_class()->NAME(__VA_ARGS__)(ARGUMENTS(__VA_ARGS__));    \
    })

FORWARD_FN(VRSettings, const char *, vrSettings, GetSettingsErrorNameFromEnum, EVRSettingsError);
FORWARD_FN(VRSettings, void, vrSettings, SetBool,
           const char *, const char *, bool, EVRSettingsError *);
FORWARD_FN(VRSettings, void, vrSettings, SetInt32,
           const char *, const char *, int32_t, EVRSettingsError *);
FORWARD_FN(VRSettings, void, vrSettings, SetFloat,
           const char *, const char *, float, EVRSettingsError *);
FORWARD_FN(VRSettings, void, vrSettings, SetString,
           const char *, const char *, const char *, EVRSettingsError *);
FORWARD_FN(VRSettings, bool, vrSettings, GetBool,
           const char *, const char *, EVRSettingsError *);
FORWARD_FN(VRSettings, int32_t, vrSettings, GetInt32,
           const char *, const char *, EVRSettingsError *);
FORWARD_FN(VRSettings, float, vrSettings, GetFloat,
           const char *, const char *, EVRSettingsError *);
FORWARD_FN(VRSettings, void, vrSettings, GetString,
           const char *, const char *, char *, uint32_t, EVRSettingsError *);
FORWARD_FN(VRSettings, void, vrSettings, RemoveSection,
           const char *, EVRSettingsError *);
FORWARD_FN(VRSettings, void, vrSettings, RemoveKeyInSection,
           const char *, const char *, EVRSettingsError *);

FORWARD_FN(VRPropertiesRaw, const char *, vr, GetPropErrorNameFromEnum, ETrackedPropertyError);
FORWARD_FN(VRPropertiesRaw, PropertyContainerHandle_t, vr, TrackedDeviceToPropertyContainer,
           TrackedDeviceIndex_t);

FORWARD_FN(VRProperties, bool, vr, GetBoolProperty,
           PropertyContainerHandle_t, ETrackedDeviceProperty, ETrackedPropertyError *);
FORWARD_FN(VRProperties, float, vr, GetFloatProperty,
           PropertyContainerHandle_t, ETrackedDeviceProperty, ETrackedPropertyError *);
FORWARD_FN(VRProperties, int32_t, vr, GetInt32Property,
           PropertyContainerHandle_t, ETrackedDeviceProperty, ETrackedPropertyError *);
FORWARD_FN(VRProperties, uint64_t, vr, GetUint64Property,
           PropertyContainerHandle_t, ETrackedDeviceProperty, ETrackedPropertyError *);
FORWARD_FN(VRProperties, HmdVector2_t, vr, GetVec2Property,
           PropertyContainerHandle_t, ETrackedDeviceProperty, ETrackedPropertyError *);
FORWARD_FN(VRProperties, HmdVector3_t, vr, GetVec3Property,
           PropertyContainerHandle_t, ETrackedDeviceProperty, ETrackedPropertyError *);
FORWARD_FN(VRProperties, HmdVector4_t, vr, GetVec4Property,
           PropertyContainerHandle_t, ETrackedDeviceProperty, ETrackedPropertyError *);
FORWARD_FN(VRProperties, double, vr, GetDoubleProperty,
           PropertyContainerHandle_t, ETrackedDeviceProperty, ETrackedPropertyError *);
FORWARD_FN(VRProperties, uint32_t, vr, GetProperty,
           PropertyContainerHandle_t,
           ETrackedDeviceProperty,
           void *,
           uint32_t,
           PropertyTypeTag_t *,
           ETrackedPropertyError *);
FORWARD_FN(VRProperties, uint32_t, vr, GetStringProperty,
           PropertyContainerHandle_t,
           ETrackedDeviceProperty,
           char *,
           uint32_t,
           ETrackedPropertyError *);
FORWARD_FN(VRProperties, ETrackedPropertyError, vr, SetBoolProperty,
           PropertyContainerHandle_t, ETrackedDeviceProperty, bool);
FORWARD_FN(VRProperties, ETrackedPropertyError, vr, SetFloatProperty,
           PropertyContainerHandle_t, ETrackedDeviceProperty, float);
FORWARD_FN(VRProperties, ETrackedPropertyError, vr, SetInt32Property,
           PropertyContainerHandle_t, ETrackedDeviceProperty, int32_t);
FORWARD_FN(VRProperties, ETrackedPropertyError, vr, SetUint64Property,
           PropertyContainerHandle_t, ETrackedDeviceProperty, uint64_t);
FORWARD_FN(VRProperties, ETrackedPropertyError, vr, SetVec2Property,
           PropertyContainerHandle_t, ETrackedDeviceProperty, const HmdVector2_t &);
FORWARD_FN(VRProperties, ETrackedPropertyError, vr, SetVec3Property,
           PropertyContainerHandle_t, ETrackedDeviceProperty, const HmdVector3_t &);
FORWARD_FN(VRProperties, ETrackedPropertyError, vr, SetVec4Property,
           PropertyContainerHandle_t, ETrackedDeviceProperty, const HmdVector4_t &);
FORWARD_FN(VRProperties, ETrackedPropertyError, vr, SetDoubleProperty,
           PropertyContainerHandle_t, ETrackedDeviceProperty, double);
FORWARD_FN(VRProperties, ETrackedPropertyError, vr, SetStringProperty,
           PropertyContainerHandle_t, ETrackedDeviceProperty, const char *);
FORWARD_FN(VRProperties, ETrackedPropertyError, vr, SetProperty,
           PropertyContainerHandle_t, ETrackedDeviceProperty, void *, uint32_t, PropertyTypeTag_t);
FORWARD_FN(VRProperties, ETrackedPropertyError, vr, SetPropertyError,
           PropertyContainerHandle_t, ETrackedDeviceProperty, ETrackedPropertyError);
FORWARD_FN(VRProperties, ETrackedPropertyError, vr, EraseProperty,
           PropertyContainerHandle_t, ETrackedDeviceProperty);
FORWARD_FN(VRProperties, bool, vr, IsPropertySet,
           PropertyContainerHandle_t, ETrackedDeviceProperty, ETrackedPropertyError *);

FORWARD_FN(VRHiddenArea, ETrackedPropertyError, vr, SetHiddenArea,
           EVREye, EHiddenAreaMeshType, HmdVector2_t *, uint32_t);
FORWARD_FN(VRHiddenArea, uint32_t, vr, GetHiddenArea,
           EVREye, EHiddenAreaMeshType, HmdVector2_t *, uint32_t, ETrackedPropertyError *);

FORWARD_FN(VRDriverLog, void, vrDriver, Log, const char *);

bool vrServerDriverHostTrackedDeviceAdded(
    const char *pchDeviceSerialNumber,
    ETrackedDeviceClass eDeviceClass,
    TrackedDeviceServerDriver *pDriver)
{
    return VRServerDriverHost()->TrackedDeviceAdded(
        pchDeviceSerialNumber,
        eDeviceClass,
        (ITrackedDeviceServerDriver *)pDriver);
}
FORWARD_FN(VRServerDriverHost, void, vrServerDriverHost, TrackedDevicePoseUpdated,
           uint32_t, const DriverPose_t &, uint32_t);
FORWARD_FN(VRServerDriverHost, void, vrServerDriverHost, VsyncEvent, double);
FORWARD_FN(VRServerDriverHost, void, vrServerDriverHost, VendorSpecificEvent,
           uint32_t, EVREventType, const VREvent_Data_t &, double);
FORWARD_FN(VRServerDriverHost, bool, vrServerDriverHost, IsExiting);
FORWARD_FN(VRServerDriverHost, bool, vrServerDriverHost, PollNextEvent,
           VREvent_t *, uint32_t);
FORWARD_FN(VRServerDriverHost, void, vrServerDriverHost, GetRawTrackedDevicePoses,
           float, TrackedDevicePose_t *, uint32_t);
FORWARD_FN(VRServerDriverHost, void, vrServerDriverHost, RequestRestart,
           const char *, const char *, const char *, const char *);
FORWARD_FN(VRServerDriverHost, uint32_t, vrServerDriverHost, GetFrameTimings,
           Compositor_FrameTiming *, uint32_t);
FORWARD_FN(VRServerDriverHost, void, vrServerDriverHost, SetDisplayEyeToHead,
           uint32_t, const HmdMatrix34_t &, const HmdMatrix34_t &);
FORWARD_FN(VRServerDriverHost, void, vrServerDriverHost, SetDisplayProjectionRaw,
           uint32_t, const HmdRect2_t &, const HmdRect2_t &);
FORWARD_FN(VRServerDriverHost, void, vrServerDriverHost, SetRecommendedRenderTargetSize,
           uint32_t, uint32_t, uint32_t);

FORWARD_FN(VRWatchdogHost, void, vr, WatchdogWakeUp, ETrackedDeviceClass);

FORWARD_FN(VRCompositorDriverHost, bool, vrCompositorDriverHost, PollNextEvent,
           VREvent_t *, uint32_t);

DriverHandle_t vrDriverHandle()
{
    return VRDriverHandle();
}

FORWARD_FN(VRDriverManager, uint32_t, vrDriverManager, GetDriverCount);
FORWARD_FN(VRDriverManager, uint32_t, vrDriverManager, GetDriverName,
           DriverId_t, char *, uint32_t);
FORWARD_FN(VRDriverManager, DriverHandle_t, vrDriverManager, GetDriverHandle, const char *);
FORWARD_FN(VRDriverManager, bool, vrDriverManager, IsEnabled, DriverId_t);

FORWARD_FN(VRResources, uint32_t, vr, LoadSharedResource, const char *, char *, uint32_t);
FORWARD_FN(VRResources, uint32_t, vr, GetResourceFullPath,
           const char *, const char *, char *, uint32_t);

FORWARD_FN(VRDriverInput, EVRInputError, vrDriverInput, CreateBooleanComponent,
           PropertyContainerHandle_t, const char *, VRInputComponentHandle_t *);
FORWARD_FN(VRDriverInput, EVRInputError, vrDriverInput, UpdateBooleanComponent,
           VRInputComponentHandle_t, bool, double);
FORWARD_FN(VRDriverInput, EVRInputError, vrDriverInput, CreateScalarComponent,
           PropertyContainerHandle_t,
           const char *,
           VRInputComponentHandle_t *,
           EVRScalarType,
           EVRScalarUnits);
FORWARD_FN(VRDriverInput, EVRInputError, vrDriverInput, UpdateScalarComponent,
           VRInputComponentHandle_t, float, double);
FORWARD_FN(VRDriverInput, EVRInputError, vrDriverInput, CreateHapticComponent,
           PropertyContainerHandle_t, const char *, VRInputComponentHandle_t *);
FORWARD_FN(VRDriverInput, EVRInputError, vrDriverInput, CreateSkeletonComponent,
           PropertyContainerHandle_t,
           const char *,
           const char *,
           const char *,
           EVRSkeletalTrackingLevel,
           const VRBoneTransform_t *,
           uint32_t,
           VRInputComponentHandle_t *);
FORWARD_FN(VRDriverInput, EVRInputError, vrDriverInput, UpdateSkeletonComponent,
           VRInputComponentHandle_t, EVRSkeletalMotionRange, const VRBoneTransform_t *, uint32_t);

FORWARD_FN(VRIOBuffer, EIOBufferError, vrIOBuffer, Open,
           const char *, EIOBufferMode, uint32_t, uint32_t, IOBufferHandle_t *);
FORWARD_FN(VRIOBuffer, EIOBufferError, vrIOBuffer, Close, IOBufferHandle_t);
FORWARD_FN(VRIOBuffer, EIOBufferError, vrIOBuffer, Read,
           IOBufferHandle_t, void *, uint32_t, uint32_t *);
FORWARD_FN(VRIOBuffer, EIOBufferError, vrIOBuffer, Write, IOBufferHandle_t, void *, uint32_t);
FORWARD_FN(VRIOBuffer, PropertyContainerHandle_t, vrIOBuffer, PropertyContainer, IOBufferHandle_t);
FORWARD_FN(VRIOBuffer, bool, vrIOBuffer, HasReaders, IOBufferHandle_t);

FORWARD_FN(VRDriverSpatialAnchors,
           EVRSpatialAnchorError, vrDriverSpatialAnchors, UpdateSpatialAnchorPose,
           SpatialAnchorHandle_t, const SpatialAnchorDriverPose_t *);
FORWARD_FN(VRDriverSpatialAnchors,
           EVRSpatialAnchorError, vrDriverSpatialAnchors, SetSpatialAnchorPoseError,
           SpatialAnchorHandle_t, EVRSpatialAnchorError, double);
FORWARD_FN(VRDriverSpatialAnchors,
           EVRSpatialAnchorError, vrDriverSpatialAnchors, UpdateSpatialAnchorDescriptor,
           SpatialAnchorHandle_t, const char *);
FORWARD_FN(VRDriverSpatialAnchors,
           EVRSpatialAnchorError, vrDriverSpatialAnchors, GetSpatialAnchorPose,
           SpatialAnchorHandle_t, SpatialAnchorDriverPose_t *);
FORWARD_FN(VRDriverSpatialAnchors,
           EVRSpatialAnchorError, vrDriverSpatialAnchors, GetSpatialAnchorDescriptor,
           SpatialAnchorHandle_t, char *, uint32_t *, bool);

#define INIT_DRIVER_CONTEXT(name)                                        \
    EVRInitError vrInit##name##DriverContext(IVRDriverContext *pContext) \
    {                                                                    \
        return Init##name##DriverContext(pContext);                      \
    }

INIT_DRIVER_CONTEXT(Server);
INIT_DRIVER_CONTEXT(Watchdog);
INIT_DRIVER_CONTEXT(Compositor);

void vrCleanupDriverContext()
{
    CleanupDriverContext();
}