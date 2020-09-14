#pragma once

enum class DeviceType {
    OCULUS_QUEST,
    OCULUS_QUEST_2,
    UNKNOWN,
};

struct EyeFov {
    float left;
    float right;
    float top;
    float bottom;
};

struct OnCreateResult {
    int surfaceTextureHandle;
    int webViewSurfaceHandle;
};

struct OnResumeResult {
    DeviceType deviceType;
    int recommendedEyeWidth;
    int recommendedEyeHeight;
    float refreshRates[16] = {0};
    int refreshRatesCount;
    float defaultRefreshRate;
};

struct OnStreamStartParams {
    int eyeWidth;
    int eyeHeight;
    EyeFov leftEyeFov;
    bool foveationEnabled;
    float foveationStrength;
    float foveationShape;
    float foveationVerticalOffset;
    bool enableMicrophone;
};

// Note: JNI object are obscured behind void* to avoid problems when binding to Rust

extern "C" OnCreateResult onCreate(void *env, void *activity, void *assetManager);

extern "C" OnResumeResult onResume(void *env, void *surface);

extern "C" void onStreamStart(OnStreamStartParams params);

extern "C" void render(bool streaming, long long renderedFrameIndex);

extern "C" void onStreamStop();

extern "C" void onPause();

extern "C" void onDestroy(void *v_env);