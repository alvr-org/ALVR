#include "bindings.h"

#include <unistd.h>
#include <jni.h>
#include <VrApi.h>
#include <VrApi_Types.h>
#include <VrApi_Helpers.h>
#include <VrApi_SystemUtils.h>
#include <VrApi_Input.h>
#include <memory>
#include <chrono>
#include <android/native_window.h>
#include <android/native_window_jni.h>
#include <android/input.h>
#include "ffr.h"
#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <GLES3/gl3.h>
#include <GLES2/gl2ext.h>
#include <string>
#include <map>
#include <vector>
#include "utils.h"
#include "render.h"
#include "packet_types.h"
#include "asset.h"
#include <inttypes.h>
#include <glm/gtx/euler_angles.hpp>
#include <mutex>

using namespace std;
using namespace gl_render_utils;

void (*inputSend)(TrackingInfo data);
void (*reportSubmit)(unsigned long long targetTimestampNs, unsigned long long vsyncQueueNs);
unsigned long long (*getPredictionOffsetNs)();
void (*videoErrorReportSend)();
void (*viewsConfigSend)(EyeFov fov[2], float ipd_m);
void (*batterySend)(unsigned long long device_path, float gauge_value, bool is_plugged);
unsigned long long (*pathStringToHash)(const char *path);

uint64_t HEAD_PATH;
uint64_t LEFT_HAND_PATH;
uint64_t RIGHT_HAND_PATH;
uint64_t LEFT_CONTROLLER_HAPTICS_PATH;
uint64_t RIGHT_CONTROLLER_HAPTICS_PATH;

const chrono::duration<float> MENU_BUTTON_LONG_PRESS_DURATION = 5s;
const uint32_t ovrButton_Unknown1 = 0x01000000;
const int MAXIMUM_TRACKING_FRAMES = 360;

const int LOADING_TEXTURE_WIDTH = 1280;
const int LOADING_TEXTURE_HEIGHT = 720;

struct Rect {
    unsigned int x, y, width, height;
};

class OvrContext {
public:
    ANativeWindow *window = nullptr;
    ovrMobile *Ovr{};
    ovrJava java{};
    JNIEnv *env{};

    unique_ptr<Texture> streamTexture;
    unique_ptr<Texture> loadingTexture;
    std::vector<uint8_t> loadingTextureBitmap;
    std::mutex loadingTextureMutex;

    StreamConfig streamConfig{};

    vector<float> refreshRatesBuffer;

    uint64_t ovrFrameIndex = 0;

    int m_LastHMDRecenterCount = -1;

    std::map<uint64_t, ovrTracking2> trackingFrameMap;
    std::mutex trackingFrameMutex;

    bool darkMode;
    ovrRenderer Renderer;

    uint8_t lastLeftControllerBattery = 0;
    uint8_t lastRightControllerBattery = 0;

    float lastIpd;
    EyeFov lastFov;

    ovrHandPose lastHandPose[2];
    ovrTracking lastTrackingRot[2];
    ovrTracking lastTrackingPos[2];

    struct HapticsState {
        uint64_t startUs;
        uint64_t endUs;
        float amplitude;
        float frequency;
        bool fresh;
        bool buffered;
    };
    // mHapticsState[0]: right hand state
    // mHapticsState[1]: left hand state
    HapticsState mHapticsState[2]{};


    std::chrono::system_clock::time_point mMenuNotPressedLastInstant;
    bool mMenuLongPressActivated = false;
};

namespace {
    OvrContext g_ctx;
}

OnCreateResult onCreate(void *v_env, void *v_activity, void *v_assetManager) {
    auto *env = (JNIEnv *) v_env;
    auto activity = (jobject) v_activity;
    auto assetManager = (jobject) v_assetManager;

    HEAD_PATH = pathStringToHash("/user/head");
    LEFT_HAND_PATH = pathStringToHash("/user/hand/left");
    RIGHT_HAND_PATH = pathStringToHash("/user/hand/right");
    LEFT_CONTROLLER_HAPTICS_PATH = pathStringToHash("/user/hand/left/output/haptic");
    RIGHT_CONTROLLER_HAPTICS_PATH = pathStringToHash("/user/hand/right/output/haptic");

    LOG("Initializing EGL.");

    setAssetManager(env, assetManager);

    g_ctx.env = env;
    g_ctx.java.Env = env;
    env->GetJavaVM(&g_ctx.java.Vm);
    g_ctx.java.ActivityObject = env->NewGlobalRef(activity);

    eglInit();

    const ovrInitParms initParms = vrapi_DefaultInitParms(&g_ctx.java);
    int32_t initResult = vrapi_Initialize(&initParms);
    if (initResult != VRAPI_INITIALIZE_SUCCESS) {
        // If initialization failed, vrapi_* function calls will not be available.
        LOGE("vrapi_Initialize failed");
    }

    //
    // Generate texture for SurfaceTexture which is output of MediaCodec.
    //

    g_ctx.streamTexture = make_unique<Texture>(true);

    g_ctx.loadingTexture = make_unique<Texture>(
            false, 1280, 720, GL_RGBA, std::vector<uint8_t>(1280 * 720 * 4, 0));

    memset(g_ctx.mHapticsState, 0, sizeof(g_ctx.mHapticsState));

    //ovrPlatformInitializeResult res = ovr_PlatformInitializeAndroid("", activity, env);
    //LOGI("ovrPlatformInitializeResult %s", ovrPlatformInitializeResult_ToString(res));
    //ovrRequest req;
    //req = ovr_User_GetLoggedInUser();
    //LOGI("Logged in user is %" PRIu64 "\n", req);

    return {(int) g_ctx.streamTexture.get()->GetGLTexture(),
            (int) g_ctx.loadingTexture.get()->GetGLTexture()};
}

void destroyNative(void *v_env) {
    auto *env = (JNIEnv *) v_env;

    LOG("Destroying EGL.");

    g_ctx.streamTexture.reset();
    g_ctx.loadingTexture.reset();

    eglDestroy();

    vrapi_Shutdown();

    env->DeleteGlobalRef(g_ctx.java.ActivityObject);
}

uint64_t mapButtons(ovrInputTrackedRemoteCapabilities *remoteCapabilities,
                    ovrInputStateTrackedRemote *remoteInputState) {
    uint64_t buttons = 0;
    if (remoteCapabilities->ControllerCapabilities & ovrControllerCaps_ModelOculusTouch) {
        // Oculus Quest Touch Cotroller
        if (remoteInputState->Buttons & ovrButton_A) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_A_CLICK);
        }
        if (remoteInputState->Buttons & ovrButton_B) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_B_CLICK);
        }
        if (remoteInputState->Buttons & ovrButton_RThumb) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_JOYSTICK_CLICK);
        }
        if (remoteInputState->Buttons & ovrButton_X) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_X_CLICK);
        }
        if (remoteInputState->Buttons & ovrButton_Y) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_Y_CLICK);
        }
        if (remoteInputState->Buttons & ovrButton_LThumb) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_JOYSTICK_CLICK);
        }
        if (remoteInputState->Buttons & ovrButton_Enter) {
            // Menu button on left hand
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_SYSTEM_CLICK);
        }
        if (remoteInputState->Buttons & ovrButton_GripTrigger) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_GRIP_CLICK);
        }
        if (remoteInputState->Buttons & ovrButton_Trigger) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_CLICK);
        }
        if (remoteInputState->Buttons & ovrButton_Joystick) {
            if (remoteCapabilities->ControllerCapabilities & ovrControllerCaps_LeftHand) {
                buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_JOYSTICK_LEFT_CLICK);
            } else {
                buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_JOYSTICK_RIGHT_CLICK);
            }
        }
        if (remoteInputState->Buttons & ovrButton_Unknown1) {
            // Only on right controller. What's button???
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_BACK_CLICK);
        }
        if (remoteInputState->Touches & ovrTouch_A) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_A_TOUCH);
        }
        if (remoteInputState->Touches & ovrTouch_B) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_B_TOUCH);
        }
        if (remoteInputState->Touches & ovrTouch_X) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_X_TOUCH);
        }
        if (remoteInputState->Touches & ovrTouch_Y) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_Y_TOUCH);
        }
        if (remoteInputState->Touches & ovrTouch_IndexTrigger) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_TOUCH);
        }
        if (remoteInputState->Touches & ovrTouch_Joystick) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_JOYSTICK_TOUCH);
        }
        if (remoteInputState->Touches & ovrTouch_ThumbRest) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_THUMB_REST_TOUCH);
        }
    } else {
        // GearVR or Oculus Go Controller
        if (remoteInputState->Buttons & ovrButton_A) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_TOUCH);
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_CLICK);
        }
        if (remoteInputState->Buttons & ovrButton_Enter) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_A_CLICK);
        }
        if (remoteInputState->Buttons & ovrButton_Back) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_B_TOUCH);
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_B_CLICK);
        }
        if (remoteInputState->TrackpadStatus) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_TRACKPAD_TOUCH);
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_A_TOUCH);
        }
    }
    return buttons;
}


void setControllerInfo(TrackingInfo *packet, double displayTime) {
    ovrInputCapabilityHeader curCaps;
    ovrResult result;
    int controller = 0;

    for (uint32_t deviceIndex = 0;
         vrapi_EnumerateInputDevices(g_ctx.Ovr, deviceIndex, &curCaps) >= 0; deviceIndex++) {
        LOG("Device %d: Type=%d ID=%d", deviceIndex, curCaps.Type, curCaps.DeviceID);
        if (curCaps.Type == ovrControllerType_Hand) {  //A3
            ovrInputHandCapabilities handCapabilities;
            ovrInputStateHand inputStateHand;
            handCapabilities.Header = curCaps;

            result = vrapi_GetInputDeviceCapabilities(g_ctx.Ovr, &handCapabilities.Header);

            if (result != ovrSuccess) {
                continue;
            }

            if ((handCapabilities.HandCapabilities & ovrHandCaps_LeftHand) != 0) {
                controller = 0;
            } else {
                controller = 1;
            }
            inputStateHand.Header.ControllerType = handCapabilities.Header.Type;

            result = vrapi_GetCurrentInputState(g_ctx.Ovr, handCapabilities.Header.DeviceID,
                                                &inputStateHand.Header);
            if (result != ovrSuccess) {
                continue;
            }

            auto &c = packet->controller[controller];

            c.enabled = true;
            c.isHand = true;

            memcpy(&c.orientation, &inputStateHand.PointerPose.Orientation,
                   sizeof(inputStateHand.PointerPose.Orientation));
            memcpy(&c.position, &inputStateHand.PointerPose.Position,
                   sizeof(inputStateHand.PointerPose.Position));

            ovrHandedness handedness =
                    handCapabilities.HandCapabilities & ovrHandCaps_LeftHand ? VRAPI_HAND_LEFT
                                                                             : VRAPI_HAND_RIGHT;
            ovrHandSkeleton handSkeleton;
            handSkeleton.Header.Version = ovrHandVersion_1;
            if (vrapi_GetHandSkeleton(g_ctx.Ovr, handedness, &handSkeleton.Header) != ovrSuccess) {
                LOG("VrHands - failed to get hand skeleton");
            } else {
                for (int i = 0; i < ovrHandBone_MaxSkinnable; i++) {
                    memcpy(&c.bonePositionsBase[i], &handSkeleton.BonePoses[i].Position,
                           sizeof(handSkeleton.BonePoses[i].Position));
                }
            }

            ovrHandPose handPose;
            handPose.Header.Version = ovrHandVersion_1;
            if (vrapi_GetHandPose(g_ctx.Ovr, handCapabilities.Header.DeviceID, displayTime,
                                  &handPose.Header) !=
                ovrSuccess) {
                LOG("VrHands - failed to get hand pose");
            } else {
                if (handPose.HandConfidence == ovrConfidence_HIGH) {
                    c.handFingerConfidences |= alvrHandConfidence_High;
                }
                for (int i = 0; i < ovrHandFinger_Max; i++) {
                    c.handFingerConfidences |=
                            handPose.FingerConfidences[i] == ovrConfidence_HIGH ? (1 << i) : 0;
                }
                if (handPose.Status&ovrHandTrackingStatus_Tracked) {
                    memcpy(&c.boneRootOrientation, &handPose.RootPose.Orientation,
                           sizeof(handPose.RootPose.Orientation));
                    memcpy(&c.boneRootPosition, &handPose.RootPose.Position,
                           sizeof(handPose.RootPose.Position));
                    for (int i = 0; i < ovrHandBone_MaxSkinnable; i++) {
                        memcpy(&c.boneRotations[i], &handPose.BoneRotations[i],
                               sizeof(handPose.BoneRotations[i]));
                    }
                    memcpy(&g_ctx.lastHandPose[controller], &handPose,
                           sizeof(handPose));
                } else if (g_ctx.lastHandPose[controller].Status&ovrHandTrackingStatus_Tracked) {
                    memcpy(&c.boneRootOrientation, &g_ctx.lastHandPose[controller].RootPose.Orientation,
                           sizeof(g_ctx.lastHandPose[controller].RootPose.Orientation));
                    memcpy(&c.boneRootPosition, &g_ctx.lastHandPose[controller].RootPose.Position,
                           sizeof(g_ctx.lastHandPose[controller].RootPose.Position));
                    for (int i = 0; i < ovrHandBone_MaxSkinnable; i++) {
                        memcpy(&c.boneRotations[i], &g_ctx.lastHandPose[controller].BoneRotations[i],
                               sizeof(g_ctx.lastHandPose[controller].BoneRotations[i]));
                    }
                }
            }
        }
        if (curCaps.Type == ovrControllerType_TrackedRemote) {
            ovrInputTrackedRemoteCapabilities remoteCapabilities;
            ovrInputStateTrackedRemote remoteInputState;

            remoteCapabilities.Header = curCaps;
            result = vrapi_GetInputDeviceCapabilities(g_ctx.Ovr, &remoteCapabilities.Header);
            if (result != ovrSuccess) {
                continue;
            }
            remoteInputState.Header.ControllerType = remoteCapabilities.Header.Type;

            result = vrapi_GetCurrentInputState(g_ctx.Ovr, remoteCapabilities.Header.DeviceID,
                                                &remoteInputState.Header);
            if (result != ovrSuccess) {
                continue;
            }

            LOG("ID=%d Cap Controller=%08X Button=%08X Touch=%08X",
                curCaps.DeviceID,
                remoteCapabilities.ControllerCapabilities,
                remoteCapabilities.ButtonCapabilities,
                remoteCapabilities.TouchCapabilities);
            LOG("ID=%d Sta Button=%08X Touch=%08X Joystick=(%f,%f) IndexValue=%f GripValue=%f",
                curCaps.DeviceID,
                remoteInputState.Buttons, remoteInputState.Touches,
                remoteInputState.JoystickNoDeadZone.x, remoteInputState.JoystickNoDeadZone.y,
                remoteInputState.IndexTrigger, remoteInputState.GripTrigger);

            uint64_t hand_path;
            if ((remoteCapabilities.ControllerCapabilities & ovrControllerCaps_LeftHand) != 0) {
                hand_path = LEFT_HAND_PATH;
                controller = 0;
            } else {
                hand_path = RIGHT_HAND_PATH;
                controller = 1;
            }

            auto &c = packet->controller[controller];

            c.enabled = true;

            c.buttons = mapButtons(&remoteCapabilities, &remoteInputState);

            if ((remoteCapabilities.ControllerCapabilities & ovrControllerCaps_HasJoystick) !=
                0) {
                c.trackpadPosition.x = remoteInputState.JoystickNoDeadZone.x;
                c.trackpadPosition.y = remoteInputState.JoystickNoDeadZone.y;
            } else {
                // Normalize to -1.0 - +1.0 for OpenVR Input. y-asix should be reversed.
                c.trackpadPosition.x =
                        remoteInputState.TrackpadPosition.x / remoteCapabilities.TrackpadMaxX *
                        2.0f - 1.0f;
                c.trackpadPosition.y =
                        remoteInputState.TrackpadPosition.y / remoteCapabilities.TrackpadMaxY *
                        2.0f - 1.0f;
                c.trackpadPosition.y = -c.trackpadPosition.y;
            }
            c.triggerValue = remoteInputState.IndexTrigger;
            c.gripValue = remoteInputState.GripTrigger;

            if (hand_path == LEFT_HAND_PATH) {
                if (remoteInputState.BatteryPercentRemaining != g_ctx.lastLeftControllerBattery) {
                    batterySend(hand_path, (float)remoteInputState.BatteryPercentRemaining / 100.0, false);
                    g_ctx.lastLeftControllerBattery = remoteInputState.BatteryPercentRemaining;
                }
            } else {
                 if (remoteInputState.BatteryPercentRemaining != g_ctx.lastRightControllerBattery) {
                    batterySend(hand_path, (float)remoteInputState.BatteryPercentRemaining / 100.0, false);
                    g_ctx.lastRightControllerBattery = remoteInputState.BatteryPercentRemaining;
                }
            }

            ovrTracking tracking;
            if (vrapi_GetInputTrackingState(g_ctx.Ovr, remoteCapabilities.Header.DeviceID,
                                            displayTime, &tracking) != ovrSuccess) {
                LOG("vrapi_GetInputTrackingState failed. Device was disconnected?");
            } else {

                memcpy(&c.orientation,
                        &tracking.HeadPose.Pose.Orientation,
                        sizeof(tracking.HeadPose.Pose.Orientation));

                if ((tracking.Status & VRAPI_TRACKING_STATUS_POSITION_TRACKED) ||
                        (remoteCapabilities.ControllerCapabilities & ovrControllerCaps_ModelOculusGo)) {
                    memcpy(&c.position,
                           &tracking.HeadPose.Pose.Position,
                           sizeof(tracking.HeadPose.Pose.Position));
                    memcpy(&g_ctx.lastTrackingPos[controller],
                           &tracking,
                           sizeof(tracking));
                } else {
                    memcpy(&c.position,
                           &g_ctx.lastTrackingPos[controller].HeadPose.Pose.Position,
                           sizeof(g_ctx.lastTrackingPos[controller].HeadPose.Pose.Position));
                }

                memcpy(&c.angularVelocity,
                       &tracking.HeadPose.AngularVelocity,
                       sizeof(tracking.HeadPose.AngularVelocity));

                memcpy(&c.linearVelocity,
                       &tracking.HeadPose.LinearVelocity,
                       sizeof(tracking.HeadPose.LinearVelocity));
            }
        }
    }
}

float getIPD() {
    ovrTracking2 tracking = vrapi_GetPredictedTracking2(g_ctx.Ovr, 0.0);
    float ipd = vrapi_GetInterpupillaryDistance(&tracking);
    return ipd;
}

// return fov in OpenXR convention
std::pair<EyeFov, EyeFov> getFov() {
    ovrTracking2 tracking = vrapi_GetPredictedTracking2(g_ctx.Ovr, 0.0);

    EyeFov fov[2];

    for (int eye = 0; eye < 2; eye++) {
        auto projection = tracking.Eye[eye].ProjectionMatrix;
        double a = projection.M[0][0];
        double b = projection.M[1][1];
        double c = projection.M[0][2];
        double d = projection.M[1][2];
        double n = -projection.M[2][3];
        double w1 = 2.0 * n / a;
        double h1 = 2.0 * n / b;
        double w2 = c * w1;
        double h2 = d * h1;

        double maxX = (w1 + w2) / 2.0;
        double minX = w2 - maxX;
        double maxY = (h1 + h2) / 2.0;
        double minY = h2 - maxY;

        fov[eye].left = (float)atan(minX / n);
        fov[eye].right = (float)atan(maxX / n);
        fov[eye].top = -(float)atan(minY / n);
        fov[eye].bottom = -(float)atan(maxY / n);
    }
    return {fov[0], fov[1]};
}

// Called from TrackingThread
void sendTrackingInfo(bool clientsidePrediction) {
    // vrapi_GetTimeInSeconds doesn't match getTimestampUs
    uint64_t targetTimestampNs = vrapi_GetTimeInSeconds() * 1e9 + getPredictionOffsetNs();
    auto tracking = vrapi_GetPredictedTracking2(g_ctx.Ovr, (double)targetTimestampNs / 1e9);

    {
        std::lock_guard<std::mutex> lock(g_ctx.trackingFrameMutex);
        g_ctx.trackingFrameMap.insert({ targetTimestampNs, tracking });
        if (g_ctx.trackingFrameMap.size() > MAXIMUM_TRACKING_FRAMES) {
            g_ctx.trackingFrameMap.erase(g_ctx.trackingFrameMap.cbegin());
        }
    }

    TrackingInfo info = {};
    info.targetTimestampNs = targetTimestampNs;

    info.mounted = vrapi_GetSystemStatusInt(&g_ctx.java, VRAPI_SYS_STATUS_MOUNTED);

    memcpy(&info.HeadPose_Pose_Orientation, &tracking.HeadPose.Pose.Orientation,
           sizeof(ovrQuatf));
    memcpy(&info.HeadPose_Pose_Position, &tracking.HeadPose.Pose.Position,
           sizeof(ovrVector3f));

    setControllerInfo(&info, clientsidePrediction ? (double)targetTimestampNs / 1e9 : 0.);

    inputSend(info);

    float new_ipd = getIPD();
    auto new_fov = getFov();
    if (abs(new_ipd - g_ctx.lastIpd) > 0.001 || abs(new_fov.first.left - g_ctx.lastFov.left) > 0.001) {
        EyeFov fov[2] = { new_fov.first, new_fov.second };
        viewsConfigSend(fov, new_ipd);
        g_ctx.lastIpd = new_ipd;
        g_ctx.lastFov = new_fov.first;
    }
}

OnResumeResult onResumeNative(void *v_surface, bool darkMode) {
    auto surface = (jobject) v_surface;

    g_ctx.window = ANativeWindow_fromSurface(g_ctx.env, surface);

    LOGI("Entering VR mode.");

    ovrModeParms parms = vrapi_DefaultModeParms(&g_ctx.java);

    parms.Flags |= VRAPI_MODE_FLAG_RESET_WINDOW_FULLSCREEN;

    parms.Flags |= VRAPI_MODE_FLAG_NATIVE_WINDOW;
    parms.Display = (size_t) egl.Display;
    parms.WindowSurface = (size_t) g_ctx.window;
    parms.ShareContext = (size_t) egl.Context;

    g_ctx.Ovr = vrapi_EnterVrMode(&parms);

    if (g_ctx.Ovr == nullptr) {
        LOGE("Invalid ANativeWindow");
    }

    {
        // set Color Space
        ovrHmdColorDesc colorDesc{};
        colorDesc.ColorSpace = VRAPI_COLORSPACE_RIFT_S;
        vrapi_SetClientColorDesc(g_ctx.Ovr, &colorDesc);
    }

    vrapi_SetPerfThread(g_ctx.Ovr, VRAPI_PERF_THREAD_TYPE_MAIN, gettid());

    vrapi_SetTrackingSpace(g_ctx.Ovr, VRAPI_TRACKING_SPACE_STAGE);

    auto eyeWidth = vrapi_GetSystemPropertyInt(&g_ctx.java, VRAPI_SYS_PROP_DISPLAY_PIXELS_WIDE) / 2;
    auto eyeHeight = vrapi_GetSystemPropertyInt(&g_ctx.java,
                                                VRAPI_SYS_PROP_DISPLAY_PIXELS_HIGH);
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wmissing-field-initializers"
    ovrRenderer_Create(&g_ctx.Renderer, eyeWidth, eyeHeight, g_ctx.streamTexture.get(),
                       g_ctx.loadingTexture->GetGLTexture(), {false});
#pragma clang diagnostic pop

    ovrRenderer_CreateScene(&g_ctx.Renderer, darkMode);

    g_ctx.darkMode = darkMode;

    auto result = OnResumeResult();

    result.recommendedEyeWidth = eyeWidth;
    result.recommendedEyeHeight = eyeHeight;

    result.refreshRatesCount = vrapi_GetSystemPropertyInt(&g_ctx.java,
                                                          VRAPI_SYS_PROP_NUM_SUPPORTED_DISPLAY_REFRESH_RATES);
    g_ctx.refreshRatesBuffer = vector<float>(result.refreshRatesCount);
    vrapi_GetSystemPropertyFloatArray(&g_ctx.java, VRAPI_SYS_PROP_SUPPORTED_DISPLAY_REFRESH_RATES,
                                      &g_ctx.refreshRatesBuffer[0], result.refreshRatesCount);
    result.refreshRates = &g_ctx.refreshRatesBuffer[0];

    return result;
}

void setStreamConfig(StreamConfig config) {
    g_ctx.streamConfig = config;
}

void onStreamStartNative() {
    ovrRenderer_Destroy(&g_ctx.Renderer);
    ovrRenderer_Create(&g_ctx.Renderer, g_ctx.streamConfig.eyeWidth, g_ctx.streamConfig.eyeHeight,
                       g_ctx.streamTexture.get(), g_ctx.loadingTexture->GetGLTexture(),
                       {g_ctx.streamConfig.enableFoveation,
                        g_ctx.streamConfig.eyeWidth, g_ctx.streamConfig.eyeHeight,
                        g_ctx.streamConfig.foveationCenterSizeX, g_ctx.streamConfig.foveationCenterSizeY,
                        g_ctx.streamConfig.foveationCenterShiftX, g_ctx.streamConfig.foveationCenterShiftY,
                        g_ctx.streamConfig.foveationEdgeRatioX, g_ctx.streamConfig.foveationEdgeRatioY});
    ovrRenderer_CreateScene(&g_ctx.Renderer, g_ctx.darkMode);

    // On Oculus Quest, without ExtraLatencyMode frames passed to vrapi_SubmitFrame2 are sometimes discarded from VrAPI(?).
    // Which introduces stutter animation.
    // I think the number of discarded frames is shown as Stale in Logcat like following:
    //    I/VrApi: FPS=72,Prd=63ms,Tear=0,Early=0,Stale=8,VSnc=1,Lat=0,Fov=0,CPU4/GPU=3/3,1958/515MHz,OC=FF,TA=0/E0/0,SP=N/F/N,Mem=1804MHz,Free=989MB,PSM=0,PLS=0,Temp=36.0C/0.0C,TW=1.90ms,App=2.74ms,GD=0.00ms
    // After enabling ExtraLatencyMode:
    //    I/VrApi: FPS=71,Prd=76ms,Tear=0,Early=66,Stale=0,VSnc=1,Lat=1,Fov=0,CPU4/GPU=3/3,1958/515MHz,OC=FF,TA=0/E0/0,SP=N/N/N,Mem=1804MHz,Free=906MB,PSM=0,PLS=0,Temp=38.0C/0.0C,TW=1.93ms,App=1.46ms,GD=0.00ms
    // We need to set ExtraLatencyMode On to workaround for this issue.
    vrapi_SetExtraLatencyMode(g_ctx.Ovr,
                              (ovrExtraLatencyMode) g_ctx.streamConfig.extraLatencyMode);

    ovrResult result = vrapi_SetDisplayRefreshRate(g_ctx.Ovr, g_ctx.streamConfig.refreshRate);
    if (result != ovrSuccess) {
        LOGE("Failed to set refresh rate requested by the server: %d", result);
    }

    g_ctx.m_LastHMDRecenterCount = -1; // make sure we send guardian data

    // reset battery and view config to make sure they get sent
    g_ctx.lastIpd = 0;
    g_ctx.lastLeftControllerBattery = 0;
    g_ctx.lastRightControllerBattery = 0;
}

void onPauseNative() {
    ovrRenderer_Destroy(&g_ctx.Renderer);

    LOGI("Leaving VR mode.");

    vrapi_LeaveVrMode(g_ctx.Ovr);

    g_ctx.Ovr = nullptr;

    if (g_ctx.window != nullptr) {
        ANativeWindow_release(g_ctx.window);
    }
    g_ctx.window = nullptr;
}

void finishHapticsBuffer(ovrDeviceID DeviceID) {
    uint8_t hapticBuffer[1] = {0};
    ovrHapticBuffer buffer;
    buffer.BufferTime = vrapi_GetPredictedDisplayTime(g_ctx.Ovr, g_ctx.ovrFrameIndex);
    buffer.HapticBuffer = &hapticBuffer[0];
    buffer.NumSamples = 1;
    buffer.Terminated = true;

    auto result = vrapi_SetHapticVibrationBuffer(g_ctx.Ovr, DeviceID, &buffer);
    if (result != ovrSuccess) {
        LOGI("vrapi_SetHapticVibrationBuffer: Failed. result=%d", result);
    }
}

void updateHapticsState() {
    ovrInputCapabilityHeader curCaps;
    ovrResult result;

    for (uint32_t deviceIndex = 0;
         vrapi_EnumerateInputDevices(g_ctx.Ovr, deviceIndex, &curCaps) >= 0; deviceIndex++) {
        if (curCaps.Type == ovrControllerType_Gamepad) continue;
        ovrInputTrackedRemoteCapabilities remoteCapabilities;

        remoteCapabilities.Header = curCaps;
        result = vrapi_GetInputDeviceCapabilities(g_ctx.Ovr, &remoteCapabilities.Header);
        if (result != ovrSuccess) {
            continue;
        }

        int curHandIndex = (remoteCapabilities.ControllerCapabilities & ovrControllerCaps_LeftHand)
                           ? 1 : 0;
        auto &s = g_ctx.mHapticsState[curHandIndex];

        uint64_t currentUs = getTimestampUs();

        if (s.fresh) {
            s.startUs = s.startUs + currentUs;
            s.endUs = s.startUs + s.endUs;
            s.fresh = false;
        }

        if (s.startUs <= 0) {
            // No requested haptics for this hand.
            if (s.buffered) {
                finishHapticsBuffer(curCaps.DeviceID);
                s.buffered = false;
            }
            continue;
        }

        if (currentUs >= s.endUs) {
            // No more haptics is needed.
            s.startUs = 0;
            if (s.buffered) {
                finishHapticsBuffer(curCaps.DeviceID);
                s.buffered = false;
            }
            continue;
        }

        if (remoteCapabilities.ControllerCapabilities &
            ovrControllerCaps_HasBufferedHapticVibration) {
            // Note: HapticSamplesMax=25 HapticSampleDurationMS=2 on Quest

            // First, call with buffer.Terminated = false and when haptics is no more needed call with buffer.Terminated = true (to stop haptics?).
            LOG("Send haptic buffer. HapticSamplesMax=%d HapticSampleDurationMS=%d",
                remoteCapabilities.HapticSamplesMax, remoteCapabilities.HapticSampleDurationMS);

            auto requiredHapticsBuffer = static_cast<uint32_t >((s.endUs - currentUs) /
                                                                (remoteCapabilities.HapticSampleDurationMS *
                                                                1000));

            std::vector<uint8_t> hapticBuffer(remoteCapabilities.HapticSamplesMax);
            ovrHapticBuffer buffer;
            buffer.BufferTime = vrapi_GetPredictedDisplayTime(g_ctx.Ovr, g_ctx.ovrFrameIndex);
            buffer.HapticBuffer = &hapticBuffer[0];
            buffer.NumSamples = std::min(remoteCapabilities.HapticSamplesMax,
                                         requiredHapticsBuffer);
            buffer.Terminated = false;

            for (uint32_t i = 0; i < buffer.NumSamples; i++) {
                if (s.amplitude > 1.0f)
                    hapticBuffer[i] = 255;
                else
                    hapticBuffer[i] = static_cast<uint8_t>(255 * s.amplitude);
            }

            result = vrapi_SetHapticVibrationBuffer(g_ctx.Ovr, curCaps.DeviceID, &buffer);
            if (result != ovrSuccess) {
                LOGI("vrapi_SetHapticVibrationBuffer: Failed. result=%d", result);
            }
            s.buffered = true;
        } else if (remoteCapabilities.ControllerCapabilities &
                   ovrControllerCaps_HasSimpleHapticVibration) {
            LOG("Send simple haptic. amplitude=%f", s.amplitude);
            vrapi_SetHapticVibrationSimple(g_ctx.Ovr, curCaps.DeviceID, s.amplitude);
        }
    }
}

void renderNative(long long targetTimespampNs) {
    g_ctx.ovrFrameIndex++;

    FrameLog(targetTimespampNs, "Got frame for render.");

    updateHapticsState();

    ovrTracking2 tracking;
    {
        std::lock_guard<std::mutex> lock(g_ctx.trackingFrameMutex);

        const auto it = g_ctx.trackingFrameMap.find(targetTimespampNs);
        if (it != g_ctx.trackingFrameMap.end()) {
            tracking = it->second;
        } else {
            if (!g_ctx.trackingFrameMap.empty())
                tracking = g_ctx.trackingFrameMap.cbegin()->second;
            else
                return;
        }
    }

// Render eye images and setup the primary layer using ovrTracking2.
    const ovrLayerProjection2 worldLayer =
            ovrRenderer_RenderFrame(&g_ctx.Renderer, &tracking, false);

    double vsyncQueueS = vrapi_GetPredictedDisplayTime(g_ctx.Ovr, g_ctx.ovrFrameIndex) - vrapi_GetTimeInSeconds();
    reportSubmit(targetTimespampNs, vsyncQueueS * 1e9);

    const ovrLayerHeader2 *layers2[] =
            {
                    &worldLayer.Header
            };

    ovrSubmitFrameDescription2 frameDesc = {};
    frameDesc.Flags = 0;
    frameDesc.SwapInterval = 1;
    frameDesc.FrameIndex = g_ctx.ovrFrameIndex;
    frameDesc.DisplayTime = (double)targetTimespampNs / 1e9;
    frameDesc.LayerCount = 1;
    frameDesc.Layers = layers2;

    vrapi_SubmitFrame2(g_ctx.Ovr, &frameDesc);
}

void updateLoadingTexuture(const unsigned char *data) {
    std::lock_guard<std::mutex> lock(g_ctx.loadingTextureMutex);

    g_ctx.loadingTextureBitmap.resize(LOADING_TEXTURE_WIDTH * LOADING_TEXTURE_HEIGHT * 4);

    memcpy(&g_ctx.loadingTextureBitmap[0], data,
            LOADING_TEXTURE_WIDTH * LOADING_TEXTURE_HEIGHT * 4);
}

void renderLoadingNative() {
    // update text image
    {
        std::lock_guard<std::mutex> lock(g_ctx.loadingTextureMutex);

        if (!g_ctx.loadingTextureBitmap.empty()) {
            glBindTexture(GL_TEXTURE_2D, g_ctx.loadingTexture->GetGLTexture());
            glTexSubImage2D(GL_TEXTURE_2D, 0, 0, 0, LOADING_TEXTURE_WIDTH, LOADING_TEXTURE_HEIGHT,
                            GL_RGBA, GL_UNSIGNED_BYTE, &g_ctx.loadingTextureBitmap[0]);
        }
        g_ctx.loadingTextureBitmap.clear();
    }

    // Show a loading icon.
    g_ctx.ovrFrameIndex++;

    double displayTime = vrapi_GetPredictedDisplayTime(g_ctx.Ovr, g_ctx.ovrFrameIndex);
    ovrTracking2 headTracking = vrapi_GetPredictedTracking2(g_ctx.Ovr, displayTime);

    const ovrLayerProjection2 worldLayer = ovrRenderer_RenderFrame(&g_ctx.Renderer, &headTracking,
                                                                   true);

    const ovrLayerHeader2 *layers[] =
            {
                    &worldLayer.Header
            };


    ovrSubmitFrameDescription2 frameDesc = {};
    frameDesc.Flags = 0;
    frameDesc.SwapInterval = 1;
    frameDesc.FrameIndex = g_ctx.ovrFrameIndex;
    frameDesc.DisplayTime = displayTime;
    frameDesc.LayerCount = 1;
    frameDesc.Layers = layers;

    vrapi_SubmitFrame2(g_ctx.Ovr, &frameDesc);
}

void onHapticsFeedbackNative(unsigned long long path,
                            float duration_s,
                            float frequency,
                            float amplitude) {
    int curHandIndex = (path == RIGHT_CONTROLLER_HAPTICS_PATH ? 0 : 1);
    auto &s = g_ctx.mHapticsState[curHandIndex];
    s.startUs = 0;
    s.endUs = (uint64_t)(duration_s * 1000'000);
    s.amplitude = amplitude;
    s.frequency = frequency;
    s.fresh = true;
    s.buffered = false;
}

void onBatteryChangedNative(int battery, int plugged) {
    batterySend(HEAD_PATH, (float)battery / 100.0, (bool)plugged);
}

GuardianData getGuardianData() {
    int recenterCount = vrapi_GetSystemStatusInt(&g_ctx.java, VRAPI_SYS_STATUS_RECENTER_COUNT);
    bool shouldSync;
    if (recenterCount != g_ctx.m_LastHMDRecenterCount) {
        shouldSync = true;
        g_ctx.m_LastHMDRecenterCount = recenterCount;
    } else {
        shouldSync = false;
    }

    auto data = GuardianData{};
    data.shouldSync = shouldSync;

    if (shouldSync) {
        ovrPosef spacePose;
        ovrVector3f bboxScale;
        // Theoretically pose (the 2nd parameter) could be nullptr, since we already have that, but
        // then this function gives us 0-size bounding box, so it has to be provided.
        vrapi_GetBoundaryOrientedBoundingBox(g_ctx.Ovr, &spacePose, &bboxScale);
        data.areaWidth = 2.0f * bboxScale.x;
        data.areaHeight = 2.0f * bboxScale.z;
    }

    return data;
}

void onTrackingNative(bool clientsidePrediction) {
    if (g_ctx.Ovr != nullptr) {
        sendTrackingInfo(clientsidePrediction);
    }
}
