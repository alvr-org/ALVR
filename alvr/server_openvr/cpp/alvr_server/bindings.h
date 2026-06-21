#pragma once

struct FfiFov {
    float left;
    float right;
    float up;
    float down;
};

struct FfiQuat {
    float x;
    float y;
    float z;
    float w;
};

struct FfiPose {
    FfiQuat orientation;
    float position[3];
};

struct FfiDeviceMotion {
    unsigned long long deviceID;
    FfiPose pose;
    float linearVelocity[3];
    float angularVelocity[3];
};

struct FfiViewParams {
    FfiPose pose;
    FfiFov fov;
};

struct FfiHandSkeleton {
    float jointPositions[31][3];
    FfiQuat jointRotations[31];
};

struct FfiHandData {
    const FfiDeviceMotion* controllerMotion;
    const FfiHandSkeleton* handSkeleton;
    bool isHandTracker;
    bool predictHandSkeleton;
};

enum FfiOpenvrPropertyType {
    Bool,
    Float,
    Int32,
    Uint64,
    Vector3,
    Double,
    String,
};

union FfiOpenvrPropertyValue {
    unsigned int bool_;
    float float_;
    int int32;
    unsigned long long uint64;
    float vector3[3];
    double double_;
    char string[256];
};

struct FfiOpenvrProperty {
    unsigned int key;
    FfiOpenvrPropertyType type;
    FfiOpenvrPropertyValue value;
};

enum FfiButtonType {
    BUTTON_TYPE_BINARY,
    BUTTON_TYPE_SCALAR,
};

struct FfiButtonValue {
    FfiButtonType type;
    union {
        unsigned int binary;
        float scalar;
    };
};

struct FfiDynamicEncoderParams {
    unsigned int updated;
    unsigned long long bitrate_bps;
    float framerate;
};

struct Settings {
    int m_refreshRate;
    unsigned int m_renderWidth;
    unsigned int m_renderHeight;
    int m_recommendedTargetWidth;
    int m_recommendedTargetHeight;
    int m_nAdapterIndex;
    char m_captureFrameDir[1024];

    bool m_enableFoveatedEncoding;
    float m_foveationCenterSizeX;
    float m_foveationCenterSizeY;
    float m_foveationCenterShiftX;
    float m_foveationCenterShiftY;
    float m_foveationEdgeRatioX;
    float m_foveationEdgeRatioY;

    bool m_enableColorCorrection;
    float m_brightness;
    float m_contrast;
    float m_saturation;
    float m_gamma;
    float m_sharpening;

    int m_codec;
    int m_h264Profile;
    bool m_use10bitEncoder;
    double m_encodingGamma;
    bool m_enableHdr;
    bool m_forceHdrSrgbCorrection;
    bool m_clampHdrExtendedRange;
    bool m_enableAmfPreAnalysis;
    bool m_enableVbaq;
    bool m_enableAmfHmqb;
    bool m_useAmfPreproc;
    unsigned int m_amfPreProcSigma;
    unsigned int m_amfPreProcTor;
    unsigned int m_encoderQualityPreset;
    bool m_amdBitrateCorruptionFix;
    unsigned int m_nvencQualityPreset;
    unsigned int m_rateControlMode;
    bool m_fillerData;
    unsigned int m_entropyCoding;
    bool m_forceSwEncoding;
    unsigned int m_swThreadCount;

    unsigned int m_nvencTuningPreset;
    unsigned int m_nvencMultiPass;
    unsigned int m_nvencAdaptiveQuantizationMode;
    long long m_nvencLowDelayKeyFrameScale;
    long long m_nvencRefreshRate;
    bool m_nvencEnableIntraRefresh;
    long long m_nvencIntraRefreshPeriod;
    long long m_nvencIntraRefreshCount;
    long long m_nvencMaxNumRefFrames;
    long long m_nvencGopLength;
    long long m_nvencPFrameStrategy;
    long long m_nvencRateControlMode;
    long long m_nvencRcBufferSize;
    long long m_nvencRcInitialDelay;
    long long m_nvencRcMaxBitrate;
    long long m_nvencRcAverageBitrate;
    bool m_nvencEnableWeightedPrediction;

    unsigned long long m_minimumIdrIntervalMs;

    bool m_enableViveTrackerProxy = false;
    bool m_trackingRefOnly = false;
    bool m_enableLinuxVulkanAsyncCompute;
    bool m_enableLinuxAsyncReprojection;

    bool m_enableControllers;
    bool m_controllerIsTracker = false;
    bool m_enableBodyTrackingFakeVive = false;
    bool m_bodyTrackingHasLegs = false;
    bool m_useSeparateHandTrackers = false;
};

extern "C" const unsigned char* FRAME_RENDER_VS_CSO_PTR;
extern "C" unsigned int FRAME_RENDER_VS_CSO_LEN;
extern "C" const unsigned char* FRAME_RENDER_PS_CSO_PTR;
extern "C" unsigned int FRAME_RENDER_PS_CSO_LEN;
extern "C" const unsigned char* QUAD_SHADER_CSO_PTR;
extern "C" unsigned int QUAD_SHADER_CSO_LEN;
extern "C" const unsigned char* COMPRESS_AXIS_ALIGNED_CSO_PTR;
extern "C" unsigned int COMPRESS_AXIS_ALIGNED_CSO_LEN;
extern "C" const unsigned char* COLOR_CORRECTION_CSO_PTR;
extern "C" unsigned int COLOR_CORRECTION_CSO_LEN;
extern "C" const unsigned char* RGBTOYUV420_CSO_PTR;
extern "C" unsigned int RGBTOYUV420_CSO_LEN;

extern "C" const unsigned char* QUAD_SHADER_COMP_SPV_PTR;
extern "C" unsigned int QUAD_SHADER_COMP_SPV_LEN;
extern "C" const unsigned char* COLOR_SHADER_COMP_SPV_PTR;
extern "C" unsigned int COLOR_SHADER_COMP_SPV_LEN;
extern "C" const unsigned char* FFR_SHADER_COMP_SPV_PTR;
extern "C" unsigned int FFR_SHADER_COMP_SPV_LEN;
extern "C" const unsigned char* RGBTOYUV420_SHADER_COMP_SPV_PTR;
extern "C" unsigned int RGBTOYUV420_SHADER_COMP_SPV_LEN;

extern "C" const char* g_sessionPath;
extern "C" const char* g_driverRootDir;

extern "C" void (*LogError)(const char* stringPtr);
extern "C" void (*LogWarn)(const char* stringPtr);
extern "C" void (*LogInfo)(const char* stringPtr);
extern "C" void (*LogDebug)(const char* stringPtr);
extern "C" void (*LogEncoder)(const char* stringPtr);
extern "C" void (*LogPeriodically)(const char* tag, const char* stringPtr);
extern "C" void (*DriverReadyIdle)(bool setDefaultChaprone);
extern "C" void (*SetVideoConfigNals)(const unsigned char* configBuffer, int len, int codec);
extern "C" void (*VideoSend)(
    unsigned long long targetTimestampNs, unsigned char* buf, int len, bool isIdr
);
extern "C" void (*HapticsSend)(
    unsigned long long path, float duration_s, float frequency, float amplitude
);
extern "C" void (*ShutdownRuntime)();
extern "C" unsigned long long (*PathStringToHash)(const char* path);
extern "C" void (*ReportPresent)(unsigned long long timestamp_ns, unsigned long long offset_ns);
extern "C" void (*ReportComposed)(unsigned long long timestamp_ns, unsigned long long offset_ns);
extern "C" FfiDynamicEncoderParams (*GetDynamicEncoderParams)();
extern "C" unsigned long long (*GetSerialNumber)(unsigned long long deviceID, char* outString);
extern "C" void (*SetOpenvrProps)(void* instancePtr, unsigned long long deviceID);
extern "C" void (*RegisterButtons)(void* instancePtr, unsigned long long deviceID);
extern "C" void (*WaitForVSync)();

extern "C" void CppInit(bool earlyHmdInitialization, Settings settings);
extern "C" void* CppOpenvrEntryPoint(const char* pInterfaceName, int* pReturnCode);
extern "C" bool InitializeStreaming(Settings settings);
extern "C" void DeinitializeStreaming();
extern "C" void SendVSync();
extern "C" void RequestIDR();
extern "C" void SetTracking(
    unsigned long long targetTimestampNs,
    float controllerPoseTimeOffsetS,
    FfiDeviceMotion headMotion,
    FfiHandData leftHandData,
    FfiHandData rightHandData,
    const FfiDeviceMotion* bodyTrackerMotions,
    int bodyTrackerMotionCount
);
extern "C" void RequestDriverResync();
extern "C" void ShutdownSteamvr();

extern "C" void SetOpenvrProperty(void* instancePtr, FfiOpenvrProperty prop);
extern "C" void SetOpenvrPropByDeviceID(unsigned long long deviceID, FfiOpenvrProperty prop);
extern "C" void RegisterButton(void* instancePtr, unsigned long long buttonID);
extern "C" void SetLocalViewParams(const FfiViewParams params[2]);
extern "C" void SetBattery(unsigned long long deviceID, float gauge_value, bool is_plugged);
extern "C" void SetButton(unsigned long long buttonID, FfiButtonValue value);
extern "C" void SetProximityState(bool headset_is_worn);

extern "C" void InitOpenvrClient();
extern "C" void ShutdownOpenvrClient();
extern "C" void SetChaperoneArea(float areaWidth, float areaHeight);

extern "C" void CaptureFrame();

// NalParsing.cpp
void ParseFrameNals(
    int codec, unsigned char* buf, int len, unsigned long long targetTimestampNs, bool isIdr
);

// CrashHandler.cpp
void HookCrashHandler();

// alvr_server.cpp
const Settings* Settings_Instance();
