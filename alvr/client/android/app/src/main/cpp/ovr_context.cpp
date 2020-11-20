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
#include "OVR_Platform.h"
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
#include "latency_collector.h"
#include "packet_types.h"
#include "asset.h"
#include <inttypes.h>
#include <glm/gtx/euler_angles.hpp>
#include <mutex>

using namespace std;

const chrono::duration<float> MENU_BUTTON_LONG_PRESS_DURATION = 1s;
const int DEFAULT_REFRESH_RATE = 72;
const uint32_t ovrButton_Unknown1 = 0x01000000;
const int MAXIMUM_TRACKING_FRAMES = 180;

struct TrackingFrame {
    ovrTracking2 tracking;
    uint64_t frameIndex;
    uint64_t fetchTime;
    double displayTime;
};

class OvrContext {
public:
    ANativeWindow *window = nullptr;
    ovrMobile *Ovr{};
    ovrJava java{};
    JNIEnv *env{};


    int16_t *micBuffer{};
    bool mStreamMic{};
    size_t mMicMaxElements{};

    ovrMicrophoneHandle mMicHandle{};

    GLuint SurfaceTextureID = 0;
    GLuint webViewSurfaceTexture = 0;
    GLuint loadingTexture = 0;
    int suspend = 0;
    bool mShowDashboard = false;
    std::function<void(InteractionType, glm::vec2)> mWebViewInteractionCallback;

    bool mExtraLatencyMode = false;
    int m_currentRefreshRate = DEFAULT_REFRESH_RATE;

    uint64_t FrameIndex = 0;

    // Oculus guardian
    int m_LastHMDRecenterCount = -1;
    bool m_ShouldSyncGuardian = false;
    bool m_GuardianSyncing = false;
    uint32_t m_AckedGuardianSegment = -1;
    uint64_t m_GuardianTimestamp = 0;
    uint32_t m_GuardianPointCount = 0;
    ovrVector3f *m_GuardianPoints = nullptr;
    double m_LastGuardianSyncTry = 0.0;

    typedef std::map<uint64_t, std::shared_ptr<TrackingFrame> > TRACKING_FRAME_MAP;

    TRACKING_FRAME_MAP trackingFrameMap;
    std::mutex trackingFrameMutex;

    ovrRenderer Renderer;

    jmethodID mServerConnection_send{};

    // headset battery level
    int batteryLevel;

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

void initializeNative(void *v_env, void *v_activity, void *v_assetManager) {
    auto *env = (JNIEnv *) v_env;
    auto activity = (jobject) v_activity;
    auto assetManager = (jobject) v_assetManager;

    LOG("Initializing EGL.");

    setAssetManager(env, assetManager);

    g_ctx.env = env;
    g_ctx.java.Env = env;
    env->GetJavaVM(&g_ctx.java.Vm);
    g_ctx.java.ActivityObject = env->NewGlobalRef(activity);

    jclass clazz = env->FindClass("com/polygraphene/alvr/OvrActivity");
    auto jWebViewInteractionCallback = env->GetMethodID(clazz, "applyWebViewInteractionEvent",
                                                        "(IFF)V");
    env->DeleteLocalRef(clazz);

    g_ctx.mWebViewInteractionCallback = [jWebViewInteractionCallback](InteractionType type,
                                                                      glm::vec2 coord) {
        if (g_ctx.mShowDashboard) {
            JNIEnv *env;
            jint res = g_ctx.java.Vm->GetEnv((void **) &env, JNI_VERSION_1_6);
            if (res == JNI_OK) {
                env->CallVoidMethod(g_ctx.java.ActivityObject, jWebViewInteractionCallback, (int) type,
                                    coord.x,
                                    coord.y);
            } else {
                LOGE("Failed to get JNI environment for dashboard interaction");
            }
        }
    };

    eglInit();

    const ovrInitParms initParms = vrapi_DefaultInitParms(&g_ctx.java);
    int32_t initResult = vrapi_Initialize(&initParms);
    if (initResult != VRAPI_INITIALIZE_SUCCESS) {
        // If initialization failed, vrapi_* function calls will not be available.
        LOGE("vrapi_Initialize failed");
        return;
    }

    GLint textureUnits;
    glGetIntegerv(GL_MAX_TEXTURE_IMAGE_UNITS, &textureUnits);
    LOGI("GL_VENDOR=%s", glGetString(GL_VENDOR));
    LOGI("GL_RENDERER=%s", glGetString(GL_RENDERER));
    LOGI("GL_VERSION=%s", glGetString(GL_VERSION));
    LOGI("GL_MAX_TEXTURE_IMAGE_UNITS=%d", textureUnits);

    //
    // Generate texture for SurfaceTexture which is output of MediaCodec.
    //

    GLuint textures[3];
    glGenTextures(3, textures);

    g_ctx.SurfaceTextureID = textures[0];
    g_ctx.webViewSurfaceTexture = textures[1];
    g_ctx.loadingTexture = textures[2];

    glBindTexture(GL_TEXTURE_EXTERNAL_OES, g_ctx.SurfaceTextureID);

    glTexParameterf(GL_TEXTURE_EXTERNAL_OES, GL_TEXTURE_MIN_FILTER,
                    GL_LINEAR);
    glTexParameterf(GL_TEXTURE_EXTERNAL_OES, GL_TEXTURE_MAG_FILTER,
                    GL_LINEAR);
    glTexParameteri(GL_TEXTURE_EXTERNAL_OES, GL_TEXTURE_WRAP_S,
                    GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_EXTERNAL_OES, GL_TEXTURE_WRAP_T,
                    GL_CLAMP_TO_EDGE);

    glBindTexture(GL_TEXTURE_EXTERNAL_OES, g_ctx.webViewSurfaceTexture);

    glTexParameterf(GL_TEXTURE_EXTERNAL_OES, GL_TEXTURE_MIN_FILTER,
                    GL_LINEAR);
    glTexParameterf(GL_TEXTURE_EXTERNAL_OES, GL_TEXTURE_MAG_FILTER,
                    GL_LINEAR);
    glTexParameteri(GL_TEXTURE_EXTERNAL_OES, GL_TEXTURE_WRAP_S,
                    GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_EXTERNAL_OES, GL_TEXTURE_WRAP_T,
                    GL_CLAMP_TO_EDGE);

    glBindTexture(GL_TEXTURE_2D, g_ctx.loadingTexture);

    glTexParameterf(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER,
                    GL_NEAREST);
    glTexParameterf(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER,
                    GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S,
                    GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T,
                    GL_CLAMP_TO_EDGE);

    auto eyeWidth = vrapi_GetSystemPropertyInt(&g_ctx.java, VRAPI_SYS_PROP_DISPLAY_PIXELS_WIDE) / 2;
    auto eyeHeight = vrapi_GetSystemPropertyInt(&g_ctx.java,
                                                VRAPI_SYS_PROP_DISPLAY_PIXELS_HIGH);

    ovrRenderer_Create(&g_ctx.Renderer, eyeWidth, eyeHeight, g_ctx.SurfaceTextureID,
                       g_ctx.loadingTexture, g_ctx.webViewSurfaceTexture,
                       g_ctx.mWebViewInteractionCallback, {false});
    ovrRenderer_CreateScene(&g_ctx.Renderer);

    clazz = env->FindClass("com/polygraphene/alvr/ServerConnection");
    g_ctx.mServerConnection_send = env->GetMethodID(clazz, "send", "(JI)V");
    env->DeleteLocalRef(clazz);

    memset(g_ctx.mHapticsState, 0, sizeof(g_ctx.mHapticsState));


    ovrPlatformInitializeResult res = ovr_PlatformInitializeAndroid("", activity, env);

    LOGI("ovrPlatformInitializeResult %s", ovrPlatformInitializeResult_ToString(res));


    ovrRequest req;
    req = ovr_User_GetLoggedInUser();


    LOGI("Logged in user is %" PRIu64 "\n", req);

    //init mic
    g_ctx.mMicHandle = ovr_Microphone_Create();

    g_ctx.mMicMaxElements = ovr_Microphone_GetOutputBufferMaxSize(g_ctx.mMicHandle);
    LOGI("Mic_maxElements %zu", g_ctx.mMicMaxElements);
    g_ctx.micBuffer = new int16_t[g_ctx.mMicMaxElements];

}

void destroyNative(void *v_env) {
    auto *env = (JNIEnv *) v_env;

    LOG("Destroying EGL.");

    ovrRenderer_Destroy(&g_ctx.Renderer);

    GLuint textures[3] = {g_ctx.SurfaceTextureID, g_ctx.webViewSurfaceTexture,
                          g_ctx.loadingTexture};
    glDeleteTextures(3, textures);

    if (g_ctx.mMicHandle) {
        ovr_Microphone_Destroy(g_ctx.mMicHandle);
    }

    eglDestroy();

    vrapi_Shutdown();

    env->DeleteGlobalRef(g_ctx.java.ActivityObject);

    delete[] g_ctx.micBuffer;
    delete[] g_ctx.m_GuardianPoints;
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
    } else {
        // GearVR or Oculus Go Controller
        if (remoteInputState->Buttons & ovrButton_A) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_CLICK);
        }
        if (remoteInputState->Buttons & ovrButton_Enter) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_TRACKPAD_CLICK);
        }
        if (remoteInputState->Buttons & ovrButton_Back) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_BACK_CLICK);
        }
        if (remoteInputState->TrackpadStatus) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_TRACKPAD_TOUCH);
        }
    }
    return buttons;
}


void setControllerInfo(TrackingInfo *packet, double displayTime, GUIInput *guiInput) {
    ovrInputCapabilityHeader curCaps;
    ovrResult result;
    int controller = 0;

    for (uint32_t deviceIndex = 0;
         vrapi_EnumerateInputDevices(g_ctx.Ovr, deviceIndex, &curCaps) >= 0; deviceIndex++) {
        LOG("Device %d: Type=%d ID=%d", deviceIndex, curCaps.Type, curCaps.DeviceID);
        if (curCaps.Type == ovrControllerType_Hand) {  //A3
            g_ctx.mShowDashboard = false;

            // Oculus Quest Hand Tracking
            if (controller >= 2) {
                LOG("Device %d: Ignore.", deviceIndex);
                continue;
            }

            auto &c = packet->controller[controller];

            ovrInputHandCapabilities handCapabilities;
            ovrInputStateHand inputStateHand;
            handCapabilities.Header = curCaps;

            result = vrapi_GetInputDeviceCapabilities(g_ctx.Ovr, &handCapabilities.Header);

            if (result != ovrSuccess) {
                continue;
            }

            if ((handCapabilities.HandCapabilities & ovrHandCaps_LeftHand) != 0) {
                c.flags |= TrackingInfo::Controller::FLAG_CONTROLLER_LEFTHAND;
            }
            inputStateHand.Header.ControllerType = handCapabilities.Header.Type;

            result = vrapi_GetCurrentInputState(g_ctx.Ovr, handCapabilities.Header.DeviceID,
                                                &inputStateHand.Header);
            if (result != ovrSuccess) {
                continue;
            }

            c.flags |= TrackingInfo::Controller::FLAG_CONTROLLER_ENABLE;

            c.flags |= TrackingInfo::Controller::FLAG_CONTROLLER_OCULUS_HAND;

            c.inputStateStatus = inputStateHand.InputStateStatus;
            memcpy(&c.fingerPinchStrengths, &inputStateHand.PinchStrength,
                   sizeof(c.fingerPinchStrengths));

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
            if (vrapi_GetHandPose(g_ctx.Ovr, handCapabilities.Header.DeviceID, 0,
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

                memcpy(&c.boneRootOrientation, &handPose.RootPose.Orientation,
                       sizeof(handPose.RootPose.Orientation));
                memcpy(&c.boneRootPosition, &handPose.RootPose.Position,
                       sizeof(handPose.RootPose.Position));
                for (int i = 0; i < ovrHandBone_MaxSkinnable; i++) {
                    memcpy(&c.boneRotations[i], &handPose.BoneRotations[i],
                           sizeof(handPose.BoneRotations[i]));
                }
            }
            controller++;
        }
        if (curCaps.Type == ovrControllerType_TrackedRemote) {
            // Gear VR / Oculus Go 3DoF Controller / Oculus Quest Touch Controller
            if (controller >= 2) {
                LOG("Device %d: Ignore.", deviceIndex);
                continue;
            }

            auto &c = packet->controller[controller];

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

            c.flags |= TrackingInfo::Controller::FLAG_CONTROLLER_ENABLE;

            if ((remoteCapabilities.ControllerCapabilities & ovrControllerCaps_LeftHand) != 0) {
                c.flags |= TrackingInfo::Controller::FLAG_CONTROLLER_LEFTHAND;

                if (remoteInputState.Buttons & ovrButton_Enter) {
                    if (!g_ctx.mMenuLongPressActivated && std::chrono::system_clock::now()
                                                          - g_ctx.mMenuNotPressedLastInstant >
                                                          MENU_BUTTON_LONG_PRESS_DURATION) {
                        g_ctx.mShowDashboard = !g_ctx.mShowDashboard;
                        g_ctx.mMenuLongPressActivated = true;

                        if (g_ctx.mShowDashboard) {
                            auto q = packet->HeadPose_Pose_Orientation;
                            auto glQuat = glm::quat(q.w, q.x, q.y, q.z);
                            auto rotEuler = glm::eulerAngles(glQuat);
                            float yaw;
                            if (abs(rotEuler.x) < M_PI_2) {
                                yaw = rotEuler.y;
                            } else {
                                yaw = M_PI - rotEuler.y;
                            }
                            auto rotation = glm::eulerAngleY(yaw);
                            auto pos = glm::vec4(0, 0, -1.5, 1);
                            glm::vec3 position = glm::vec3(rotation * pos) + guiInput->headPosition;
                            g_ctx.Renderer.webViewPanel->SetPoseTransform(position, yaw, 0);
                        }
                    }
                } else {
                    g_ctx.mMenuNotPressedLastInstant = std::chrono::system_clock::now();
                    g_ctx.mMenuLongPressActivated = false;
                }
            }

            if ((remoteCapabilities.ControllerCapabilities & ovrControllerCaps_ModelGearVR) !=
                0) {
                c.flags |= TrackingInfo::Controller::FLAG_CONTROLLER_GEARVR;
            } else if (
                    (remoteCapabilities.ControllerCapabilities & ovrControllerCaps_ModelOculusGo) !=
                    0) {
                c.flags |= TrackingInfo::Controller::FLAG_CONTROLLER_OCULUS_GO;
            } else if ((remoteCapabilities.ControllerCapabilities &
                        ovrControllerCaps_ModelOculusTouch) !=
                       0) {
                c.flags |= TrackingInfo::Controller::FLAG_CONTROLLER_OCULUS_QUEST;
            }

            if (g_ctx.mShowDashboard) {
                guiInput->actionButtonsDown[controller] =
                        remoteInputState.Buttons & (ovrButton_A | ovrButton_X | ovrButton_Trigger);
            } else {
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

                c.batteryPercentRemaining = remoteInputState.BatteryPercentRemaining;
                c.recenterCount = remoteInputState.RecenterCount;
            }

            ovrTracking tracking;
            if (vrapi_GetInputTrackingState(g_ctx.Ovr, remoteCapabilities.Header.DeviceID,
                                            0, &tracking) != ovrSuccess) {
                LOG("vrapi_GetInputTrackingState failed. Device was disconnected?");
            } else {

                memcpy(&c.orientation,
                       &tracking.HeadPose.Pose.Orientation,
                       sizeof(tracking.HeadPose.Pose.Orientation));

                memcpy(&c.position,
                       &tracking.HeadPose.Pose.Position,
                       sizeof(tracking.HeadPose.Pose.Position));

                memcpy(&c.angularVelocity,
                       &tracking.HeadPose.AngularVelocity,
                       sizeof(tracking.HeadPose.AngularVelocity));

                memcpy(&c.linearVelocity,
                       &tracking.HeadPose.LinearVelocity,
                       sizeof(tracking.HeadPose.LinearVelocity));

                memcpy(&c.angularAcceleration,
                       &tracking.HeadPose.AngularAcceleration,
                       sizeof(tracking.HeadPose.AngularAcceleration));

                memcpy(&c.linearAcceleration,
                       &tracking.HeadPose.LinearAcceleration,
                       sizeof(tracking.HeadPose.LinearAcceleration));

                auto pos = tracking.HeadPose.Pose.Position;
                guiInput->controllersPosition[controller] = glm::vec3(pos.x,
                                                                      pos.y - WORLD_VERTICAL_OFFSET,
                                                                      pos.z);
                auto rot = tracking.HeadPose.Pose.Orientation;
                guiInput->controllersRotation[controller] = glm::quat(rot.w, rot.x, rot.y, rot.z);
            }
            controller++;
        }
    }
}

float getIPD() {
    ovrTracking2 tracking = vrapi_GetPredictedTracking2(g_ctx.Ovr, 0.0);
    float ipd = vrapi_GetInterpupillaryDistance(&tracking);
    return ipd;
}

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

        double rr = 180 / M_PI;

        fov[eye].left = (float) (atan(minX / -n) * rr);
        fov[eye].right = (float) (-atan(maxX / -n) * rr);
        fov[eye].top = (float) (atan(minY / -n) * rr);
        fov[eye].bottom = (float) (-atan(maxY / -n) * rr);
    }
    return {fov[0], fov[1]};
}

// Called TrackingThread. So, we can't use this->env.
void setTrackingInfo(TrackingInfo *packet, double displayTime, ovrTracking2 *tracking) {
    memset(packet, 0, sizeof(TrackingInfo));

    uint64_t clientTime = getTimestampUs();

    packet->type = ALVR_PACKET_TYPE_TRACKING_INFO;
    packet->flags = 0;
    packet->clientTime = clientTime;
    packet->FrameIndex = g_ctx.FrameIndex;
    packet->predictedDisplayTime = displayTime;

    packet->ipd = getIPD();
    auto fovPair = getFov();
    packet->eyeFov[0] = fovPair.first;
    packet->eyeFov[1] = fovPair.second;
    packet->battery = g_ctx.batteryLevel;


    memcpy(&packet->HeadPose_Pose_Orientation, &tracking->HeadPose.Pose.Orientation,
           sizeof(ovrQuatf));
    memcpy(&packet->HeadPose_Pose_Position, &tracking->HeadPose.Pose.Position, sizeof(ovrVector3f));

    GUIInput guiInput = {};
    auto pos = tracking->HeadPose.Pose.Position;
    guiInput.headPosition = glm::vec3(pos.x, pos.y - WORLD_VERTICAL_OFFSET, pos.z);

    setControllerInfo(packet, displayTime, &guiInput);

    g_ctx.Renderer.gui->Update(guiInput);

    FrameLog(g_ctx.FrameIndex, "Sending tracking info.");
}

void checkShouldSyncGuardian() {
    int recenterCount = vrapi_GetSystemStatusInt(&g_ctx.java, VRAPI_SYS_STATUS_RECENTER_COUNT);
    if (recenterCount <= g_ctx.m_LastHMDRecenterCount) {
        return;
    }

    g_ctx.m_ShouldSyncGuardian = true;
    g_ctx.m_GuardianSyncing = false;
    g_ctx.m_GuardianTimestamp = getTimestampUs();
    delete[] g_ctx.m_GuardianPoints;
    g_ctx.m_GuardianPoints = nullptr;
    g_ctx.m_AckedGuardianSegment = -1;

    g_ctx.m_LastHMDRecenterCount = recenterCount;
}

// Called from TrackingThread
void sendTrackingInfo(void *v_env, void *v_udpReceiverThread) {
    auto *env_ = (JNIEnv *) v_env;
    auto udpReceiverThread = (jobject) v_udpReceiverThread;

    std::shared_ptr<TrackingFrame> frame(new TrackingFrame());

    g_ctx.FrameIndex++;

    frame->frameIndex = g_ctx.FrameIndex;
    frame->fetchTime = getTimestampUs();

    frame->displayTime = vrapi_GetPredictedDisplayTime(g_ctx.Ovr, g_ctx.FrameIndex);
    frame->tracking = vrapi_GetPredictedTracking2(g_ctx.Ovr, frame->displayTime);

    {
        std::lock_guard<decltype(g_ctx.trackingFrameMutex)> lock(g_ctx.trackingFrameMutex);
        g_ctx.trackingFrameMap.insert(
                std::pair<uint64_t, std::shared_ptr<TrackingFrame> >(g_ctx.FrameIndex, frame));
        if (g_ctx.trackingFrameMap.size() > MAXIMUM_TRACKING_FRAMES) {
            g_ctx.trackingFrameMap.erase(g_ctx.trackingFrameMap.cbegin());
        }
    }

    TrackingInfo info;
    setTrackingInfo(&info, frame->displayTime, &frame->tracking);


    LatencyCollector::Instance().tracking(frame->frameIndex);

    env_->CallVoidMethod(udpReceiverThread, g_ctx.mServerConnection_send,
                         reinterpret_cast<jlong>(&info),
                         static_cast<jint>(sizeof(info)));
    checkShouldSyncGuardian();
}

// Called from TrackingThread
void sendMicData(void *v_env, void *v_udpReceiverThread) {
    auto *env_ = (JNIEnv *) v_env;
    auto udpReceiverThread = (jobject) v_udpReceiverThread;

    if (!g_ctx.mStreamMic) {
        return;
    }

    size_t outputBufferNumElements = ovr_Microphone_GetPCM(g_ctx.mMicHandle, g_ctx.micBuffer,
                                                           g_ctx.mMicMaxElements);
    if (outputBufferNumElements > 0) {
        int count = 0;

        for (int i = 0; i < outputBufferNumElements; i += 100) {
            int rest = outputBufferNumElements - count * 100;

            MicAudioFrame audio{};
            memset(&audio, 0, sizeof(MicAudioFrame));

            audio.type = ALVR_PACKET_TYPE_MIC_AUDIO;
        audio.packetIndex = count;
            audio.completeSize = outputBufferNumElements;

            if (rest >= 100) {
                audio.outputBufferNumElements = 100;
            } else {
                audio.outputBufferNumElements = rest;
            }

            memcpy(&audio.micBuffer,
                   g_ctx.micBuffer + count * 100,
                   sizeof(int16_t) * audio.outputBufferNumElements);

            env_->CallVoidMethod(udpReceiverThread, g_ctx.mServerConnection_send,
                                 reinterpret_cast<jlong>(&audio),
                                 static_cast<jint>(sizeof(audio)));
            count++;
        }
    }
}

void reflectExtraLatencyMode(bool always) {
    if (always || (!gDisableExtraLatencyMode) != g_ctx.mExtraLatencyMode) {
        g_ctx.mExtraLatencyMode = !gDisableExtraLatencyMode;
        LOGI("Setting ExtraLatencyMode %s", g_ctx.mExtraLatencyMode ? "On" : "Off");
        ovrResult result = vrapi_SetExtraLatencyMode(g_ctx.Ovr,
                                                     g_ctx.mExtraLatencyMode
                                                     ? VRAPI_EXTRA_LATENCY_MODE_ON
                                                     : VRAPI_EXTRA_LATENCY_MODE_OFF);
        LOGI("vrapi_SetExtraLatencyMode. Result=%d", result);
    }
}

void onResumeNative(void *v_env, void *v_surface) {
    auto *env = (JNIEnv *) v_env;
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
        return;
    }

    LOGI("Setting refresh rate. %d Hz", g_ctx.m_currentRefreshRate);
    ovrResult result = vrapi_SetDisplayRefreshRate(g_ctx.Ovr, g_ctx.m_currentRefreshRate);
    LOGI("vrapi_SetDisplayRefreshRate: Result=%d", result);

    int CpuLevel = 3;
    int GpuLevel = 3;
    vrapi_SetClockLevels(g_ctx.Ovr, CpuLevel, GpuLevel);
    vrapi_SetPerfThread(g_ctx.Ovr, VRAPI_PERF_THREAD_TYPE_MAIN, gettid());

    // On Oculus Quest, without ExtraLatencyMode frames passed to vrapi_SubmitFrame2 are sometimes discarded from VrAPI(?).
    // Which introduces stutter animation.
    // I think the number of discarded frames is shown as Stale in Logcat like following:
    //    I/VrApi: FPS=72,Prd=63ms,Tear=0,Early=0,Stale=8,VSnc=1,Lat=0,Fov=0,CPU4/GPU=3/3,1958/515MHz,OC=FF,TA=0/E0/0,SP=N/F/N,Mem=1804MHz,Free=989MB,PSM=0,PLS=0,Temp=36.0C/0.0C,TW=1.90ms,App=2.74ms,GD=0.00ms
    // After enabling ExtraLatencyMode:
    //    I/VrApi: FPS=71,Prd=76ms,Tear=0,Early=66,Stale=0,VSnc=1,Lat=1,Fov=0,CPU4/GPU=3/3,1958/515MHz,OC=FF,TA=0/E0/0,SP=N/N/N,Mem=1804MHz,Free=906MB,PSM=0,PLS=0,Temp=38.0C/0.0C,TW=1.93ms,App=1.46ms,GD=0.00ms
    // We need to set ExtraLatencyMode On to workaround for this issue.
    reflectExtraLatencyMode(false);

    if (g_ctx.mMicHandle && g_ctx.mStreamMic) {
        ovr_Microphone_Start(g_ctx.mMicHandle);
    }

    checkShouldSyncGuardian();
}

void onStreamStartNative(int width, int height, int refreshRate, unsigned char streamMic,
                         int foveationMode, float foveationStrength, float foveationShape,
                         float foveationVerticalOffset) {
    int eyeWidth = width / 2;

    ovrRenderer_Destroy(&g_ctx.Renderer);
    ovrRenderer_Create(&g_ctx.Renderer, eyeWidth, height,
                       g_ctx.SurfaceTextureID, g_ctx.loadingTexture, g_ctx.webViewSurfaceTexture,
                       g_ctx.mWebViewInteractionCallback,
                       {(bool) foveationMode, (uint32_t) eyeWidth, (uint32_t) height,
                        EyeFov(), foveationStrength, foveationShape, foveationVerticalOffset});
    ovrRenderer_CreateScene(&g_ctx.Renderer);

    ovrResult result = vrapi_SetDisplayRefreshRate(g_ctx.Ovr, (float)refreshRate);
    if (result != ovrSuccess) {
        LOGE("Failed to set refresh rate requested by the server: %d", result);
    }

    LOGI("Setting mic streaming %d", streamMic);
    g_ctx.mStreamMic = streamMic;
    if (g_ctx.mMicHandle) {
        if (g_ctx.mStreamMic) {
            LOG("Starting mic");
            ovr_Microphone_Start(g_ctx.mMicHandle);
        } else {
            ovr_Microphone_Stop(g_ctx.mMicHandle);
        }
    }
}

void onPauseNative() {

    LOGI("Leaving VR mode.");

    vrapi_LeaveVrMode(g_ctx.Ovr);

    LOGI("Leaved VR mode.");
    g_ctx.Ovr = nullptr;

    if (g_ctx.mMicHandle && g_ctx.mStreamMic) {
        ovr_Microphone_Stop(g_ctx.mMicHandle);
    }

    if (g_ctx.window != nullptr) {
        ANativeWindow_release(g_ctx.window);
    }
    g_ctx.window = nullptr;
}

void finishHapticsBuffer(ovrDeviceID DeviceID) {
    uint8_t hapticBuffer[1] = {0};
    ovrHapticBuffer buffer;
    buffer.BufferTime = vrapi_GetPredictedDisplayTime(g_ctx.Ovr, g_ctx.FrameIndex);
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
                                                                    remoteCapabilities.HapticSampleDurationMS *
                                                                    1000);

            std::vector<uint8_t> hapticBuffer(remoteCapabilities.HapticSamplesMax);
            ovrHapticBuffer buffer;
            buffer.BufferTime = vrapi_GetPredictedDisplayTime(g_ctx.Ovr, g_ctx.FrameIndex);
            buffer.HapticBuffer = &hapticBuffer[0];
            buffer.NumSamples = std::min(remoteCapabilities.HapticSamplesMax,
                                         requiredHapticsBuffer);
            buffer.Terminated = false;

            for (int i = 0; i < remoteCapabilities.HapticSamplesMax; i++) {
                float current = ((currentUs - s.startUs) / 1000000.0f) +
                                (remoteCapabilities.HapticSampleDurationMS * i) / 1000.0f;
                float intensity =
                        (sinf(static_cast<float>(current * M_PI * 2 * s.frequency)) + 1.0f) * 0.5f *
                        s.amplitude;
                if (intensity < 0) {
                    intensity = 0;
                } else if (intensity > 1.0) {
                    intensity = 1.0;
                }
                hapticBuffer[i] = static_cast<uint8_t>(255 * intensity);
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

void renderNative(long long renderedFrameIndex) {
    LatencyCollector::Instance().rendered1(renderedFrameIndex);
    FrameLog(renderedFrameIndex, "Got frame for render.");

    updateHapticsState();

    uint64_t oldestFrame = 0;
    uint64_t mostRecentFrame = 0;
    std::shared_ptr<TrackingFrame> frame;
    {
        std::lock_guard<decltype(g_ctx.trackingFrameMutex)> lock(g_ctx.trackingFrameMutex);

        if (!g_ctx.trackingFrameMap.empty()) {
            oldestFrame = g_ctx.trackingFrameMap.cbegin()->second->frameIndex;
            mostRecentFrame = g_ctx.trackingFrameMap.crbegin()->second->frameIndex;
        }

        const auto it = g_ctx.trackingFrameMap.find(renderedFrameIndex);
        if (it != g_ctx.trackingFrameMap.end()) {
            frame = it->second;
        } else {
            if (!g_ctx.trackingFrameMap.empty())
                frame = g_ctx.trackingFrameMap.cbegin()->second;
            else
                return;
        }
    }

    FrameLog(renderedFrameIndex, "Frame latency is %lu us.",
             getTimestampUs() - frame->fetchTime);

// Render eye images and setup the primary layer using ovrTracking2.
    const ovrLayerProjection2 worldLayer =
            ovrRenderer_RenderFrame(&g_ctx.Renderer, &frame->tracking, false, g_ctx.mShowDashboard);

    LatencyCollector::Instance().rendered2(renderedFrameIndex);

    const ovrLayerHeader2 *layers2[] =
            {
                    &worldLayer.Header
            };

    ovrSubmitFrameDescription2 frameDesc = {};
    frameDesc.Flags = 0;
    frameDesc.SwapInterval = 1;
    frameDesc.FrameIndex = renderedFrameIndex;
    frameDesc.DisplayTime = 0.0;
    frameDesc.LayerCount = 1;
    frameDesc.Layers = layers2;

    ovrResult res = vrapi_SubmitFrame2(g_ctx.Ovr, &frameDesc);

    LatencyCollector::Instance().submit(renderedFrameIndex);

    FrameLog(renderedFrameIndex, "vrapi_SubmitFrame2 Orientation=(%f, %f, %f, %f)",
             frame->tracking.HeadPose.Pose.Orientation.x,
             frame->tracking.HeadPose.Pose.Orientation.y,
             frame->tracking.HeadPose.Pose.Orientation.z,
             frame->tracking.HeadPose.Pose.Orientation.w
    );

    if (g_ctx.suspend) {
        LOG("submit enter suspend");
        while (g_ctx.suspend) {
            usleep(1000 * 10);
        }
        LOG("submit leave suspend");
    }
}

void renderLoadingNative() {
    double DisplayTime = GetTimeInSeconds();

    // Show a loading icon.
    g_ctx.FrameIndex++;

    double displayTime = vrapi_GetPredictedDisplayTime(g_ctx.Ovr, g_ctx.FrameIndex);
    ovrTracking2 headTracking = vrapi_GetPredictedTracking2(g_ctx.Ovr, displayTime);

    const ovrLayerProjection2 worldLayer = ovrRenderer_RenderFrame(&g_ctx.Renderer, &headTracking,
                                                                   true, false);

    const ovrLayerHeader2 *layers[] =
            {
                    &worldLayer.Header
            };


    ovrSubmitFrameDescription2 frameDesc = {};
    frameDesc.Flags = 0;
    frameDesc.SwapInterval = 1;
    frameDesc.FrameIndex = g_ctx.FrameIndex;
    frameDesc.DisplayTime = DisplayTime;
    frameDesc.LayerCount = 1;
    frameDesc.Layers = layers;

    vrapi_SubmitFrame2(g_ctx.Ovr, &frameDesc);
}

void getRefreshRates(JNIEnv *env_, jintArray refreshRates) {
    jint *refreshRates_ = env_->GetIntArrayElements(refreshRates, nullptr);

    // Fill empty entry with 0.
    memset(refreshRates_, 0, sizeof(jint) * ALVR_REFRESH_RATE_LIST_SIZE);

    // Get list.
    int numberOfRefreshRates = vrapi_GetSystemPropertyInt(&g_ctx.java,
                                                          VRAPI_SYS_PROP_NUM_SUPPORTED_DISPLAY_REFRESH_RATES);
    std::vector<float> refreshRatesArray(numberOfRefreshRates);
    vrapi_GetSystemPropertyFloatArray(&g_ctx.java, VRAPI_SYS_PROP_SUPPORTED_DISPLAY_REFRESH_RATES,
                                      &refreshRatesArray[0], numberOfRefreshRates);

    std::string refreshRateList;
    char str[100];
    for (int i = 0; i < numberOfRefreshRates; i++) {
        snprintf(str, sizeof(str), "%f%s", refreshRatesArray[i],
                 (i != numberOfRefreshRates - 1) ? ", " : "");
        refreshRateList += str;

        if (i < ALVR_REFRESH_RATE_LIST_SIZE) {
            refreshRates_[i] = (int) refreshRatesArray[i];
        }
    }
    LOGI("Supported refresh rates: %s", refreshRateList.c_str());
    std::sort(refreshRates_, refreshRates_ + ALVR_REFRESH_RATE_LIST_SIZE, std::greater<>());

    env_->ReleaseIntArrayElements(refreshRates, refreshRates_, 0);
}

void getDeviceDescriptorNative(void *v_env, void *v_deviceDescriptor) {
    auto *env = (JNIEnv *) v_env;
    auto deviceDescriptor = (jobject) v_deviceDescriptor;

    int renderWidth = vrapi_GetSystemPropertyInt(&g_ctx.java, VRAPI_SYS_PROP_DISPLAY_PIXELS_WIDE);
    int renderHeight = vrapi_GetSystemPropertyInt(&g_ctx.java, VRAPI_SYS_PROP_DISPLAY_PIXELS_HIGH);

    int deviceType = ALVR_DEVICE_TYPE_OCULUS_MOBILE;
    int deviceSubType = 0;
    int deviceCapabilityFlags = 0;
    int controllerCapabilityFlags = ALVR_CONTROLLER_CAPABILITY_FLAG_ONE_CONTROLLER;

    int ovrDeviceType = vrapi_GetSystemPropertyInt(&g_ctx.java, VRAPI_SYS_PROP_DEVICE_TYPE);
    //if (VRAPI_DEVICE_TYPE_GEARVR_START <= ovrDeviceType &&
    //    ovrDeviceType <= VRAPI_DEVICE_TYPE_GEARVR_END) {
    if (VRAPI_DEVICE_TYPE_OCULUSQUEST_START <= ovrDeviceType &&
        ovrDeviceType <= VRAPI_DEVICE_TYPE_OCULUSQUEST_END) {
        deviceSubType = ALVR_DEVICE_SUBTYPE_OCULUS_MOBILE_QUEST;
    } else {
        // Unknown
        deviceSubType = 0;
    }
    LOGI("getDeviceDescriptor: ovrDeviceType: %d deviceType:%d deviceSubType:%d cap:%08X",
         ovrDeviceType, deviceType, deviceSubType, deviceCapabilityFlags);

    jfieldID fieldID;
    jclass clazz = env->GetObjectClass(deviceDescriptor);

    fieldID = env->GetFieldID(clazz, "mRefreshRates", "[I");

    // Array instance is already set on deviceDescriptor.
    auto refreshRates =
            reinterpret_cast<jintArray>(env->GetObjectField(deviceDescriptor, fieldID));
    getRefreshRates(env, refreshRates);
    env->SetObjectField(deviceDescriptor, fieldID, refreshRates);
    env->DeleteLocalRef(refreshRates);

    fieldID = env->GetFieldID(clazz, "mRenderWidth", "I");
    env->SetIntField(deviceDescriptor, fieldID, renderWidth);
    fieldID = env->GetFieldID(clazz, "mRenderHeight", "I");
    env->SetIntField(deviceDescriptor, fieldID, renderHeight);

    fieldID = env->GetFieldID(clazz, "mFov", "[F");
    auto fovField = reinterpret_cast<jfloatArray>(
            env->GetObjectField(deviceDescriptor, fieldID));
    jfloat *fovArray = env->GetFloatArrayElements(fovField, nullptr);
    auto fov = getFov();
    memcpy(fovArray, &fov, sizeof(fov));
    env->ReleaseFloatArrayElements(fovField, fovArray, 0);
    env->SetObjectField(deviceDescriptor, fieldID, fovField);
    env->DeleteLocalRef(fovField);

    fieldID = env->GetFieldID(clazz, "mDeviceType", "I");
    env->SetIntField(deviceDescriptor, fieldID, deviceType);
    fieldID = env->GetFieldID(clazz, "mDeviceSubType", "I");
    env->SetIntField(deviceDescriptor, fieldID, deviceSubType);
    fieldID = env->GetFieldID(clazz, "mDeviceCapabilityFlags", "I");
    env->SetIntField(deviceDescriptor, fieldID, deviceCapabilityFlags);
    fieldID = env->GetFieldID(clazz, "mControllerCapabilityFlags", "I");
    env->SetIntField(deviceDescriptor, fieldID, controllerCapabilityFlags);
    fieldID = env->GetFieldID(clazz, "mIpd", "F");
    env->SetFloatField(deviceDescriptor, fieldID, getIPD());

    env->DeleteLocalRef(clazz);
}

void onHapticsFeedbackNative(long long startTime, float amplitude, float duration,
                             float frequency, unsigned char hand) {
    int curHandIndex = (hand == 0) ? 0 : 1;
    auto &s = g_ctx.mHapticsState[curHandIndex];
    s.startUs = startTime;
    s.endUs = static_cast<uint64_t>(duration * 1000000);
    s.amplitude = amplitude;
    s.frequency = frequency;
    s.fresh = true;
    s.buffered = false;
}

bool prepareGuardianData() {
    if (g_ctx.m_GuardianPoints != nullptr) {
        return false;
    }

    vrapi_GetBoundaryGeometry(g_ctx.Ovr, 0, &g_ctx.m_GuardianPointCount, nullptr);

    if (g_ctx.m_GuardianPointCount <= 0) {
        return true;
    }

    g_ctx.m_GuardianPoints = new ovrVector3f[g_ctx.m_GuardianPointCount];
    vrapi_GetBoundaryGeometry(g_ctx.Ovr, g_ctx.m_GuardianPointCount, &g_ctx.m_GuardianPointCount,
                              g_ctx.m_GuardianPoints);

    return true;
}

// Called from TrackingThread
void sendGuardianInfo(void *v_env, void *v_udpReceiverThread) {
    auto *env_ = (JNIEnv *) v_env;
    auto udpReceiverThread = (jobject) v_udpReceiverThread;

    if (g_ctx.m_ShouldSyncGuardian) {
        double currentTime = GetTimeInSeconds();
        if (currentTime - g_ctx.m_LastGuardianSyncTry < ALVR_GUARDIAN_RESEND_CD_SEC) {
            return; // Don't spam the sync start packet
        }
        LOGI("Sending Guardian");
        g_ctx.m_LastGuardianSyncTry = currentTime;
        prepareGuardianData();

        GuardianSyncStart packet{};
        packet.type = ALVR_PACKET_TYPE_GUARDIAN_SYNC_START;
        packet.timestamp = g_ctx.m_GuardianTimestamp;
        packet.totalPointCount = g_ctx.m_GuardianPointCount;

        ovrPosef spacePose = vrapi_LocateTrackingSpace(g_ctx.Ovr, VRAPI_TRACKING_SPACE_LOCAL_FLOOR);
        memcpy(&packet.standingPosRotation, &spacePose.Orientation, sizeof(TrackingQuat));
        memcpy(&packet.standingPosPosition, &spacePose.Position, sizeof(TrackingVector3));

        ovrVector3f bboxScale;
        vrapi_GetBoundaryOrientedBoundingBox(g_ctx.Ovr, &spacePose /* dummy variable */,
                                             &bboxScale);
        packet.playAreaSize.x = 2.0f * bboxScale.x;
        packet.playAreaSize.y = 2.0f * bboxScale.z;

        env_->CallVoidMethod(udpReceiverThread, g_ctx.mServerConnection_send,
                             reinterpret_cast<jlong>(&packet), static_cast<jint>(sizeof(packet)));
    } else if (g_ctx.m_GuardianSyncing) {
        GuardianSegmentData packet{};
        packet.type = ALVR_PACKET_TYPE_GUARDIAN_SEGMENT_DATA;
        packet.timestamp = g_ctx.m_GuardianTimestamp;

        uint32_t segmentIndex = g_ctx.m_AckedGuardianSegment + 1;
        packet.segmentIndex = segmentIndex;
        uint32_t remainingPoints =
                g_ctx.m_GuardianPointCount - segmentIndex * ALVR_GUARDIAN_SEGMENT_SIZE;
        size_t countToSend =
                remainingPoints > ALVR_GUARDIAN_SEGMENT_SIZE ? ALVR_GUARDIAN_SEGMENT_SIZE
                                                             : remainingPoints;

        memcpy(&packet.points, g_ctx.m_GuardianPoints + segmentIndex * ALVR_GUARDIAN_SEGMENT_SIZE,
               sizeof(TrackingVector3) * countToSend);

        env_->CallVoidMethod(udpReceiverThread, g_ctx.mServerConnection_send,
                             reinterpret_cast<jlong>(&packet), static_cast<jint>(sizeof(packet)));
    }
}

void onGuardianSyncAckNative(long long timestamp) {
    if (timestamp != g_ctx.m_GuardianTimestamp) {
        return;
    }

    if (g_ctx.m_ShouldSyncGuardian) {
        g_ctx.m_ShouldSyncGuardian = false;
        if (g_ctx.m_GuardianPointCount > 0) {
            g_ctx.m_GuardianSyncing = true;
        }
    }
}

void onGuardianSegmentAckNative(long long timestamp, int segmentIndex) {
    if (timestamp != g_ctx.m_GuardianTimestamp ||
        segmentIndex != g_ctx.m_AckedGuardianSegment + 1) {
        return;
    }

    g_ctx.m_AckedGuardianSegment = segmentIndex;
    uint32_t segments = g_ctx.m_GuardianPointCount / ALVR_GUARDIAN_SEGMENT_SIZE;
    if (g_ctx.m_GuardianPointCount % ALVR_GUARDIAN_SEGMENT_SIZE > 0) {
        segments++;
    }

    if (g_ctx.m_AckedGuardianSegment >= segments - 1) {
        g_ctx.m_GuardianSyncing = false;
    }
}

void onBatteryChangedNative(int battery) {
    g_ctx.batteryLevel = battery;
}

int getLoadingTextureNative() {
    return g_ctx.loadingTexture;
}

int getSurfaceTextureIDNative() {
    return g_ctx.SurfaceTextureID;
}

int getWebViewSurfaceTextureNative() {
    return g_ctx.webViewSurfaceTexture;
}

void onTrackingNative(void *env, void *udpReceiverThread)
{
    if (g_ctx.Ovr != nullptr) {
        sendTrackingInfo(env, udpReceiverThread);

        //TODO: maybe use own thread, but works fine with tracking
        sendMicData(env, udpReceiverThread);

        //TODO: same as above
        sendGuardianInfo(env, udpReceiverThread);
    }
}