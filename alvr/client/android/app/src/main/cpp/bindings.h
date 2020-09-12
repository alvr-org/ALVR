#pragma once

enum class DeviceType {
    OCULUS_QUEST,
    OCULUS_QUEST_NEXT,
};

struct EyeFov {
    float left;
    float right;
    float top;
    float bottom;
};

struct OnCreateResult {
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

extern "C" OnCreateResult onCreate(void *env, void *activity, void *assetManager);

extern "C" void onResume(void *env, void *surface);

extern "C" void onStreamStart(OnStreamStartParams params);

extern "C" void onStreamStop();