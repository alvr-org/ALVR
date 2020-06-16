#include <unistd.h>
#include <jni.h>
#include <VrApi.h>
#include <VrApi_Types.h>
#include <VrApi_Helpers.h>
#include <VrApi_SystemUtils.h>
#include <VrApi_Input.h>

#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <GLES3/gl3.h>
#include <GLES2/gl2ext.h>
#include <string>
#include <map>
#include <vector>
#include <packet_types.h>
#include "utils.h"
#include "render.h"
#include "ovr_context.h"
#include "latency_collector.h"
#include "packet_types.h"
#include "ServerConnectionNative.h"
#include "asset.h"
#include <inttypes.h>

void OvrContext::initialize(JNIEnv *env, jobject activity, jobject assetManager, jobject vrThread,
                            bool ARMode, int initialRefreshRate) {
    LOG("Initializing EGL.");

    setAssetManager(env, assetManager);

    this->env = env;
    java.Env = env;
    env->GetJavaVM(&java.Vm);
    java.ActivityObject = env->NewGlobalRef(activity);

    mVrThread = env->NewGlobalRef(vrThread);

    eglInit();

    bool multi_view;
    EglInitExtensions(&multi_view);

    const ovrInitParms initParms = vrapi_DefaultInitParms(&java);
    int32_t initResult = vrapi_Initialize(&initParms);
    if (initResult != VRAPI_INITIALIZE_SUCCESS) {
        // If initialization failed, vrapi_* function calls will not be available.
        LOGE("vrapi_Initialize failed");
        return;
    }

    UseMultiview = multi_view;
    LOG("UseMultiview:%d", UseMultiview);

    GLint textureUnits;
    glGetIntegerv(GL_MAX_TEXTURE_IMAGE_UNITS, &textureUnits);
    LOGI("GL_VENDOR=%s", glGetString(GL_VENDOR));
    LOGI("GL_RENDERER=%s", glGetString(GL_RENDERER));
    LOGI("GL_VERSION=%s", glGetString(GL_VERSION));
    LOGI("GL_MAX_TEXTURE_IMAGE_UNITS=%d", textureUnits);

    m_currentRefreshRate = DEFAULT_REFRESH_RATE;
    setInitialRefreshRate(initialRefreshRate);

    //
    // Generate texture for SurfaceTexture which is output of MediaCodec.
    //
    m_ARMode = ARMode;

    GLuint textures[2];
    glGenTextures(2, textures);

    SurfaceTextureID = textures[0];
    loadingTexture = textures[1];

    glBindTexture(GL_TEXTURE_EXTERNAL_OES, SurfaceTextureID);

    glTexParameterf(GL_TEXTURE_EXTERNAL_OES, GL_TEXTURE_MIN_FILTER,
                    GL_LINEAR);
    glTexParameterf(GL_TEXTURE_EXTERNAL_OES, GL_TEXTURE_MAG_FILTER,
                    GL_LINEAR);
    glTexParameteri(GL_TEXTURE_EXTERNAL_OES, GL_TEXTURE_WRAP_S,
                    GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_EXTERNAL_OES, GL_TEXTURE_WRAP_T,
                    GL_CLAMP_TO_EDGE);

    glBindTexture(GL_TEXTURE_2D, loadingTexture);

    glTexParameterf(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER,
                    GL_NEAREST);
    glTexParameterf(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER,
                    GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S,
                    GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T,
                    GL_CLAMP_TO_EDGE);


    if (m_ARMode) {
        glGenTextures(1, &CameraTexture);
        glBindTexture(GL_TEXTURE_EXTERNAL_OES, CameraTexture);

        glTexParameterf(GL_TEXTURE_EXTERNAL_OES, GL_TEXTURE_MIN_FILTER,
                        GL_NEAREST);
        glTexParameterf(GL_TEXTURE_EXTERNAL_OES, GL_TEXTURE_MAG_FILTER,
                        GL_LINEAR);
        glTexParameteri(GL_TEXTURE_EXTERNAL_OES, GL_TEXTURE_WRAP_S,
                        GL_CLAMP_TO_EDGE);
        glTexParameteri(GL_TEXTURE_EXTERNAL_OES, GL_TEXTURE_WRAP_T,
                        GL_CLAMP_TO_EDGE);
    }

    //use the quest panel resolution and not the downscaled suggestion
    FrameBufferWidth = 1440;//vrapi_GetSystemPropertyInt(&java,VRAPI_SYS_PROP_SUGGESTED_EYE_TEXTURE_WIDTH);
    FrameBufferHeight = 1600;//vrapi_GetSystemPropertyInt(&java,VRAPI_SYS_PROP_SUGGESTED_EYE_TEXTURE_HEIGHT);


    ovrRenderer_Create(&Renderer, UseMultiview, FrameBufferWidth, FrameBufferHeight,
                       SurfaceTextureID, loadingTexture, CameraTexture, m_ARMode, {false});
    ovrRenderer_CreateScene(&Renderer);

    position_offset_y = 0.0;

    jclass clazz = env->FindClass("com/polygraphene/alvr/ServerConnection");
    mServerConnection_send = env->GetMethodID(clazz, "send", "(JI)V");
    env->DeleteLocalRef(clazz);

    memset(mHapticsState, 0, sizeof(mHapticsState));


    ovrPlatformInitializeResult res = ovr_PlatformInitializeAndroid("", activity, env);

    LOGI("ovrPlatformInitializeResult %s", ovrPlatformInitializeResult_ToString(res));


    ovrRequest req;
    req = ovr_User_GetLoggedInUser();


    LOGI("Logged in user is %" PRIu64  "\n", req);

    //init mic
    mMicHandle = ovr_Microphone_Create();

    mMicMaxElements = ovr_Microphone_GetOutputBufferMaxSize(mMicHandle);
    LOGI("Mic_maxElements %zu", mMicMaxElements);
    micBuffer = new int16_t[mMicMaxElements];

}


void OvrContext::destroy(JNIEnv *env) {
    LOG("Destroying EGL.");

    ovrRenderer_Destroy(&Renderer);

    GLuint textures[2] = {SurfaceTextureID, loadingTexture};
    glDeleteTextures(2, textures);
    if (m_ARMode) {
        glDeleteTextures(1, &CameraTexture);
    }

    if (mMicHandle){
        ovr_Microphone_Destroy(mMicHandle);
    }

    eglDestroy();

    vrapi_Shutdown();

    env->DeleteGlobalRef(mVrThread);
    env->DeleteGlobalRef(java.ActivityObject);
    env->DeleteGlobalRef(mServerConnection);

    delete[] micBuffer;


}


void OvrContext::setControllerInfo(TrackingInfo *packet, double displayTime) {
    ovrInputCapabilityHeader curCaps;
    ovrResult result;
    int controller = 0;

    for (uint32_t deviceIndex = 0;
         vrapi_EnumerateInputDevices(Ovr, deviceIndex, &curCaps) >= 0; deviceIndex++) {
        LOG("Device %d: Type=%d ID=%d", deviceIndex, curCaps.Type, curCaps.DeviceID);
        if (curCaps.Type == ovrControllerType_Hand) {  //A3
            // Oculus Quest Hand Tracking
            if (controller >= 2) {
                LOG("Device %d: Ignore.", deviceIndex);
                continue;
            }

            auto &c = packet->controller[controller];

            ovrInputHandCapabilities handCapabilities;
            ovrInputStateHand inputStateHand;
            handCapabilities.Header = curCaps;

            result = vrapi_GetInputDeviceCapabilities(Ovr, &handCapabilities.Header );

            if (result != ovrSuccess) {
                continue;
            }

            if ((handCapabilities.HandCapabilities & ovrHandCaps_LeftHand) != 0) {
                c.flags |= TrackingInfo::Controller::FLAG_CONTROLLER_LEFTHAND;
            }
            inputStateHand.Header.ControllerType = handCapabilities.Header.Type;

            result = vrapi_GetCurrentInputState(Ovr, handCapabilities.Header.DeviceID, &inputStateHand.Header);
            if (result != ovrSuccess) {
                continue;
            }

            c.flags |= TrackingInfo::Controller::FLAG_CONTROLLER_ENABLE;

            c.flags |= TrackingInfo::Controller::FLAG_CONTROLLER_OCULUS_HAND;

            c.inputStateStatus = inputStateHand.InputStateStatus;
            memcpy(&c.fingerPinchStrengths, &inputStateHand.PinchStrength, sizeof(c.fingerPinchStrengths));

            memcpy(&c.orientation, &inputStateHand.PointerPose.Orientation, sizeof(inputStateHand.PointerPose.Orientation));
            memcpy(&c.position, &inputStateHand.PointerPose.Position, sizeof(inputStateHand.PointerPose.Position));

            ovrHandedness handedness = handCapabilities.HandCapabilities & ovrHandCaps_LeftHand ? VRAPI_HAND_LEFT : VRAPI_HAND_RIGHT;
            ovrHandSkeleton handSkeleton;
            handSkeleton.Header.Version = ovrHandVersion_1;
            if (vrapi_GetHandSkeleton(Ovr, handedness, &handSkeleton.Header ) != ovrSuccess) {
                LOG("VrHands - failed to get hand skeleton");
            } else {
                for(int i=0;i<ovrHandBone_MaxSkinnable;i++) {
                    memcpy(&c.bonePositionsBase[i], &handSkeleton.BonePoses[i].Position, sizeof(handSkeleton.BonePoses[i].Position));
                }
                //for(int i=0;i<ovrHandBone_MaxSkinnable;i++) {
                //    memcpy(&c.boneRotationsBase[i], &handSkeleton.BonePoses[i].Orientation, sizeof(handSkeleton.BonePoses[i].Orientation));
                //}
            }

            ovrHandPose handPose;
            handPose.Header.Version = ovrHandVersion_1;
            if (vrapi_GetHandPose(Ovr, handCapabilities.Header.DeviceID, 0, &handPose.Header ) != ovrSuccess) {
                LOG("VrHands - failed to get hand pose");
            } else {
                if (handPose.HandConfidence == ovrConfidence_HIGH) {
                    c.handFingerConfidences |= alvrHandConfidence_High;
                }
                for (int i = 0; i < ovrHandFinger_Max; i++) {
                    c.handFingerConfidences |= handPose.FingerConfidences[i] == ovrConfidence_HIGH ? (1 << i) : 0;
                }

                memcpy(&c.boneRootOrientation, &handPose.RootPose.Orientation, sizeof(handPose.RootPose.Orientation));
                memcpy(&c.boneRootPosition, &handPose.RootPose.Position, sizeof(handPose.RootPose.Position));
                for(int i=0;i<ovrHandBone_MaxSkinnable;i++) {
                    memcpy(&c.boneRotations[i], &handPose.BoneRotations[i], sizeof(handPose.BoneRotations[i]));
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
            result = vrapi_GetInputDeviceCapabilities(Ovr, &remoteCapabilities.Header);
            if (result != ovrSuccess) {
                continue;
            }
            remoteInputState.Header.ControllerType = remoteCapabilities.Header.Type;

            result = vrapi_GetCurrentInputState(Ovr, remoteCapabilities.Header.DeviceID,
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

            c.buttons = mapButtons(&remoteCapabilities, &remoteInputState);

            if ((remoteCapabilities.ControllerCapabilities & ovrControllerCaps_HasJoystick) != 0) {
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

            ovrTracking tracking;
            if (vrapi_GetInputTrackingState(Ovr, remoteCapabilities.Header.DeviceID,
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


            }
            controller++;
        }
    }
}

uint64_t OvrContext::mapButtons(ovrInputTrackedRemoteCapabilities *remoteCapabilities,
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


// Called TrackingThread. So, we can't use this->env.
void OvrContext::setTrackingInfo(TrackingInfo *packet, double displayTime, ovrTracking2 *tracking) {
    memset(packet, 0, sizeof(TrackingInfo));

    uint64_t clientTime = getTimestampUs();

    packet->type = ALVR_PACKET_TYPE_TRACKING_INFO;
    packet->flags = 0;
    packet->clientTime = clientTime;
    packet->FrameIndex = FrameIndex;
    packet->predictedDisplayTime = displayTime;

    memcpy(&packet->HeadPose_Pose_Orientation, &tracking->HeadPose.Pose.Orientation,
           sizeof(ovrQuatf));
    memcpy(&packet->HeadPose_Pose_Position, &tracking->HeadPose.Pose.Position, sizeof(ovrVector3f));


    setControllerInfo(packet, displayTime);

    FrameLog(FrameIndex, "Sending tracking info.");
}

// Called TrackingThread. So, we can't use this->env.
void OvrContext::sendTrackingInfo(JNIEnv *env_, jobject udpReceiverThread) {
    std::shared_ptr<TrackingFrame> frame(new TrackingFrame());

    FrameIndex++;

    frame->frameIndex = FrameIndex;
    frame->fetchTime = getTimestampUs();

    frame->displayTime = vrapi_GetPredictedDisplayTime(Ovr, FrameIndex);
    frame->tracking = vrapi_GetPredictedTracking2(Ovr, frame->displayTime);

    /*LOGI("MVP %llu: \nL-V:\n%s\nL-P:\n%s\nR-V:\n%s\nR-P:\n%s", FrameIndex,
         DumpMatrix(&frame->tracking.Eye[0].ViewMatrix).c_str(),
         DumpMatrix(&frame->tracking.Eye[0].ProjectionMatrix).c_str(),
         DumpMatrix(&frame->tracking.Eye[1].ViewMatrix).c_str(),
         DumpMatrix(&frame->tracking.Eye[1].ProjectionMatrix).c_str()
         );*/

    {
        std::lock_guard<decltype(trackingFrameMutex)> lock(trackingFrameMutex);
        trackingFrameMap.insert(std::pair<uint64_t, std::shared_ptr<TrackingFrame> >(FrameIndex, frame));
        if (trackingFrameMap.size() > MAXIMUM_TRACKING_FRAMES)
        {
            trackingFrameMap.erase(trackingFrameMap.cbegin());
        }
    }

    TrackingInfo info;
    setTrackingInfo(&info, frame->displayTime, &frame->tracking);

    LatencyCollector::Instance().tracking(frame->frameIndex);

    env_->CallVoidMethod(udpReceiverThread, mServerConnection_send, reinterpret_cast<jlong>(&info),
                         static_cast<jint>(sizeof(info)));





}

// Called TrackingThread. So, we can't use this->env.
void OvrContext::sendMicData(JNIEnv *env_, jobject udpReceiverThread) {
    if(!mStreamMic) {
        return;
    }

    size_t outputBufferNumElements = ovr_Microphone_GetPCM(mMicHandle, micBuffer, mMicMaxElements);
    if(outputBufferNumElements > 0) {
        int count =0;

        for (int i = 0; i <outputBufferNumElements; i+=100) {
            int rest = outputBufferNumElements - count*100;

            MicAudioFrame audio;
            memset(&audio, 0, sizeof(MicAudioFrame));

            audio.type = ALVR_PACKET_TYPE_MIC_AUDIO;
            audio.packetIndex = count;
            audio.completeSize = outputBufferNumElements;

            if(rest >= 100) {
                audio.outputBufferNumElements = 100;
            } else {
                audio.outputBufferNumElements = rest;
            }

            memcpy(&audio.micBuffer ,
                   micBuffer + count * 100,
                   sizeof(int16_t) * audio.outputBufferNumElements);

            env_->CallVoidMethod(udpReceiverThread, mServerConnection_send,
                                 reinterpret_cast<jlong>(&audio),
                                 static_cast<jint>(sizeof(audio)));
            count++;
        }

    }


}


void OvrContext::onChangeSettings(int Suspend) {
    suspend = Suspend;
    reflectExtraLatencyMode(false);
}

void OvrContext::onSurfaceCreated(jobject surface) {
    LOG("onSurfaceCreated called. Resumed=%d Window=%p Ovr=%p", Resumed, window, Ovr);
    window = ANativeWindow_fromSurface(env, surface);

    onVrModeChange();
}

void OvrContext::onSurfaceDestroyed() {
    LOG("onSurfaceDestroyed called. Resumed=%d Window=%p Ovr=%p", Resumed, window, Ovr);
    if (window != nullptr) {
        ANativeWindow_release(window);
    }
    window = nullptr;

    onVrModeChange();
}

void OvrContext::onSurfaceChanged(jobject surface) {
    LOG("onSurfaceChanged called. Resumed=%d Window=%p Ovr=%p", Resumed, window, Ovr);
    ANativeWindow *newWindow = ANativeWindow_fromSurface(env, surface);
    if (newWindow != window) {
        LOG("Replacing ANativeWindow. %p != %p", newWindow, window);
        ANativeWindow_release(window);
        window = nullptr;
        onVrModeChange();

        window = newWindow;
        if (window != nullptr) {
            onVrModeChange();
        }
    } else if (newWindow != nullptr) {
        LOG("Got same ANativeWindow. %p == %p", newWindow, window);
        ANativeWindow_release(newWindow);
    }
}

void OvrContext::onResume() {
    LOG("onResume called. Resumed=%d Window=%p Ovr=%p", Resumed, window, Ovr);
    Resumed = true;
    onVrModeChange();

    if(mMicHandle && mStreamMic) {
        ovr_Microphone_Start(mMicHandle);
    }
}

void OvrContext::onPause() {
    LOG("onPause called. Resumed=%d Window=%p Ovr=%p", Resumed, window, Ovr);
    Resumed = false;
    onVrModeChange();

    if(mMicHandle && mStreamMic) {
        ovr_Microphone_Stop(mMicHandle);
    }
}

void OvrContext::render(uint64_t renderedFrameIndex) {
    LatencyCollector::Instance().rendered1(renderedFrameIndex);
    FrameLog(renderedFrameIndex, "Got frame for render.");

    updateHapticsState();

    uint64_t oldestFrame = 0;
    uint64_t mostRecentFrame = 0;
    std::shared_ptr<TrackingFrame> frame;
    {
        std::lock_guard<decltype(trackingFrameMutex)> lock(trackingFrameMutex);

        if (!trackingFrameMap.empty())
        {
            oldestFrame = trackingFrameMap.cbegin()->second->frameIndex;
            mostRecentFrame = trackingFrameMap.crbegin()->second->frameIndex;
        }

        const auto it = trackingFrameMap.find(renderedFrameIndex);
        if (it != trackingFrameMap.end())
        {
            frame = it->second;
        }
        else
        {
            // No matching tracking info. Too old frame.
            LOG("Too old frame has arrived. Instead, we use most old tracking data in trackingFrameMap."
                "FrameIndex=%lu trackingFrameMap=(%lu - %lu)",
                renderedFrameIndex, oldestFrame, mostRecentFrame);
            if (!trackingFrameMap.empty())
                frame = trackingFrameMap.cbegin()->second;
            else
                return;
        }
    }

    FrameLog(renderedFrameIndex, "Frame latency is %lu us.",
             getTimestampUs() - frame->fetchTime);

// Render eye images and setup the primary layer using ovrTracking2.
    const ovrLayerProjection2 worldLayer =
            ovrRenderer_RenderFrame(&Renderer, &frame->tracking, false, g_AROverlayMode);

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

    ovrResult res = vrapi_SubmitFrame2(Ovr, &frameDesc);

    LatencyCollector::Instance().submit(renderedFrameIndex);

    FrameLog(renderedFrameIndex, "vrapi_SubmitFrame2 Orientation=(%f, %f, %f, %f)",
             frame->tracking.HeadPose.Pose.Orientation.x,
             frame->tracking.HeadPose.Pose.Orientation.y,
             frame->tracking.HeadPose.Pose.Orientation.z,
             frame->tracking.HeadPose.Pose.Orientation.w
    );

    if (suspend) {
        LOG("submit enter suspend");
        while (suspend) {
            usleep(1000 * 10);
        }
        LOG("submit leave suspend");
    }
}


void OvrContext::renderLoading() {
    double DisplayTime = GetTimeInSeconds();

    // Show a loading icon.
    FrameIndex++;

    double displayTime = vrapi_GetPredictedDisplayTime(Ovr, FrameIndex);
    ovrTracking2 tracking = vrapi_GetPredictedTracking2(Ovr, displayTime);

    const ovrLayerProjection2 worldLayer =
            ovrRenderer_RenderFrame(&Renderer, &tracking, true, g_AROverlayMode);

    const ovrLayerHeader2 *layers[] =
            {
                    &worldLayer.Header
            };


    ovrSubmitFrameDescription2 frameDesc = {};
    frameDesc.Flags = 0;
    frameDesc.SwapInterval = 1;
    frameDesc.FrameIndex = FrameIndex;
    frameDesc.DisplayTime = DisplayTime;
    frameDesc.LayerCount = 1;
    frameDesc.Layers = layers;

    vrapi_SubmitFrame2(Ovr, &frameDesc);
}

void OvrContext::setFrameGeometry(int width, int height) {
    int eye_width = width / 2;

    if (eye_width != FrameBufferWidth || height != FrameBufferHeight ||
            usedFoveationEnabled != mFoveationEnabled ||
            usedFoveationStrength != mFoveationStrength ||
            usedFoveationShape != mFoveationShape ||
            usedFoveationVerticalOffset != mFoveationVerticalOffset) {

        LOG("Changing FrameBuffer geometry. Old=%dx%d New=%dx%d", FrameBufferWidth,
            FrameBufferHeight, eye_width, height);
        FrameBufferWidth = eye_width;
        FrameBufferHeight = height;

        usedFoveationEnabled = mFoveationEnabled;
        usedFoveationStrength = mFoveationStrength;
        usedFoveationShape = mFoveationShape;
        usedFoveationVerticalOffset = mFoveationVerticalOffset;

        ovrRenderer_Destroy(&Renderer);
        ovrRenderer_Create(&Renderer, UseMultiview, FrameBufferWidth, FrameBufferHeight,
                           SurfaceTextureID, loadingTexture, CameraTexture, m_ARMode,
                           {usedFoveationEnabled, (uint32_t)FrameBufferWidth, (uint32_t)FrameBufferHeight,
                            getFov().first, usedFoveationStrength, usedFoveationShape, usedFoveationVerticalOffset});
        ovrRenderer_CreateScene(&Renderer);
    } else {
        LOG("Not Changing FrameBuffer geometry. %dx%d", FrameBufferWidth,
            FrameBufferHeight);
    }
}

void OvrContext::getRefreshRates(JNIEnv *env_, jintArray refreshRates) {
    jint *refreshRates_ = env_->GetIntArrayElements(refreshRates, nullptr);

    // Fill empty entry with 0.
    memset(refreshRates_, 0, sizeof(jint) * ALVR_REFRESH_RATE_LIST_SIZE);

    // Get list.
    int numberOfRefreshRates = vrapi_GetSystemPropertyInt(&java,
                                                          VRAPI_SYS_PROP_NUM_SUPPORTED_DISPLAY_REFRESH_RATES);
    std::vector<float> refreshRatesArray(numberOfRefreshRates);
    vrapi_GetSystemPropertyFloatArray(&java, VRAPI_SYS_PROP_SUPPORTED_DISPLAY_REFRESH_RATES,
                                      &refreshRatesArray[0], numberOfRefreshRates);

    std::string refreshRateList = "";
    char str[100];
    for (int i = 0; i < numberOfRefreshRates; i++) {
        snprintf(str, sizeof(str), "%f%s", refreshRatesArray[i],
                 (i != numberOfRefreshRates - 1) ? ", " : "");
        refreshRateList += str;

        if (i < ALVR_REFRESH_RATE_LIST_SIZE) {
            refreshRates_[i] = (int) refreshRatesArray[i];
        }
    }
    LOG("Supported refresh rates: %s", refreshRateList.c_str());
    std::sort(refreshRates_, refreshRates_ + ALVR_REFRESH_RATE_LIST_SIZE, std::greater<jint>());

    env_->ReleaseIntArrayElements(refreshRates, refreshRates_, 0);
}

void OvrContext::setRefreshRate(int refreshRate, bool forceChange) {
    if (m_currentRefreshRate == refreshRate) {
        LOGI("Refresh rate not changed. %d Hz", refreshRate);
        return;
    }
    ovrResult result = vrapi_SetDisplayRefreshRate(Ovr, refreshRate);
    if (result == ovrSuccess) {
        LOGI("Changed refresh rate. %d Hz", refreshRate);
        m_currentRefreshRate = refreshRate;
    } else {
        LOGE("Failed to change refresh rate. %d Hz Force=%d Result=%d", refreshRate, forceChange,
            result);
    }
}

void OvrContext::setStreamMic(bool streamMic) {
    LOGI("Setting mic streaming %d", streamMic);
    mStreamMic = streamMic;
    if(mMicHandle) {
        if(mStreamMic) {
            LOG("Starting mic");
            ovr_Microphone_Start(mMicHandle);
        } else {
            ovr_Microphone_Stop(mMicHandle);
        }
    }

}

void OvrContext::setFFRParams(int foveationMode, float foveationStrength, float foveationShape, float foveationVerticalOffset) {
    LOGI("SSetting FFR params %d %f %f %f", foveationMode, foveationStrength, foveationShape, foveationVerticalOffset);

    mFoveationEnabled = (bool)foveationMode;
    mFoveationStrength = foveationStrength;
    mFoveationShape = foveationShape;
    mFoveationVerticalOffset = foveationVerticalOffset;
}



void OvrContext::setInitialRefreshRate(int initialRefreshRate) {
    setRefreshRate(initialRefreshRate, false);
}

void OvrContext::onVrModeChange() {
    if (Resumed && window != nullptr) {
        if (Ovr == nullptr) {
            enterVrMode();
        }
    } else {
        if (Ovr != nullptr) {
            leaveVrMode();
        }
    }
}

void OvrContext::enterVrMode() {
    LOGI("Entering VR mode.");

    ovrModeParms parms = vrapi_DefaultModeParms(&java);

    parms.Flags |= VRAPI_MODE_FLAG_RESET_WINDOW_FULLSCREEN;

    parms.Flags |= VRAPI_MODE_FLAG_NATIVE_WINDOW;
    parms.Display = (size_t) egl.Display;
    parms.WindowSurface = (size_t) window;
    parms.ShareContext = (size_t) egl.Context;

    Ovr = vrapi_EnterVrMode(&parms);

    if (Ovr == nullptr) {
        LOGE("Invalid ANativeWindow");
        return;
    }

    {
        // set Color Space
        ovrHmdColorDesc colorDesc{};
        colorDesc.ColorSpace = VRAPI_COLORSPACE_REC_2020;
        vrapi_SetClientColorDesc(Ovr, &colorDesc);
    }

    LOGI("Setting refresh rate. %d Hz", m_currentRefreshRate);
    ovrResult result = vrapi_SetDisplayRefreshRate(Ovr, m_currentRefreshRate);
    LOGI("vrapi_SetDisplayRefreshRate: Result=%d", result);

    int CpuLevel = 3;
    int GpuLevel = 3;
    vrapi_SetClockLevels(Ovr, CpuLevel, GpuLevel);
    vrapi_SetPerfThread(Ovr, VRAPI_PERF_THREAD_TYPE_MAIN, gettid());

    // On Oculus Quest, without ExtraLatencyMode frames passed to vrapi_SubmitFrame2 are sometimes discarded from VrAPI(?).
    // Which introduces stutter animation.
    // I think the number of discarded frames is shown as Stale in Logcat like following:
    //    I/VrApi: FPS=72,Prd=63ms,Tear=0,Early=0,Stale=8,VSnc=1,Lat=0,Fov=0,CPU4/GPU=3/3,1958/515MHz,OC=FF,TA=0/E0/0,SP=N/F/N,Mem=1804MHz,Free=989MB,PSM=0,PLS=0,Temp=36.0C/0.0C,TW=1.90ms,App=2.74ms,GD=0.00ms
    // After enabling ExtraLatencyMode:
    //    I/VrApi: FPS=71,Prd=76ms,Tear=0,Early=66,Stale=0,VSnc=1,Lat=1,Fov=0,CPU4/GPU=3/3,1958/515MHz,OC=FF,TA=0/E0/0,SP=N/N/N,Mem=1804MHz,Free=906MB,PSM=0,PLS=0,Temp=38.0C/0.0C,TW=1.93ms,App=1.46ms,GD=0.00ms
    // We need to set ExtraLatencyMode On to workaround for this issue.
    reflectExtraLatencyMode(false);

    // Calling back VrThread to notify Vr state change.
    jclass clazz = env->GetObjectClass(mVrThread);
    jmethodID id = env->GetMethodID(clazz, "onVrModeChanged", "(Z)V");
    env->CallVoidMethod(mVrThread, id, static_cast<jboolean>(true));
    env->DeleteLocalRef(clazz);
}

void OvrContext::leaveVrMode() {
    LOGI("Leaving VR mode.");

    vrapi_LeaveVrMode(Ovr);

    LOGI("Leaved VR mode.");
    Ovr = nullptr;

    // Calling back VrThread to notify Vr state change.
    jclass clazz = env->GetObjectClass(mVrThread);
    jmethodID id = env->GetMethodID(clazz, "onVrModeChanged", "(Z)V");
    env->CallVoidMethod(mVrThread, id, static_cast<jboolean>(false));
    env->DeleteLocalRef(clazz);
}

// Fill device descriptor.
void OvrContext::getDeviceDescriptor(JNIEnv *env, jobject deviceDescriptor) {
    int renderWidth = 1440;//vrapi_GetSystemPropertyInt(&java,VRAPI_SYS_PROP_SUGGESTED_EYE_TEXTURE_WIDTH);
    renderWidth *= 2;
    int renderHeight = 1600;//vrapi_GetSystemPropertyInt(&java,VRAPI_SYS_PROP_SUGGESTED_EYE_TEXTURE_HEIGHT);

    int deviceType = ALVR_DEVICE_TYPE_OCULUS_MOBILE;
    int deviceSubType = 0;
    int deviceCapabilityFlags = 0;
    int controllerCapabilityFlags = ALVR_CONTROLLER_CAPABILITY_FLAG_ONE_CONTROLLER;

    int ovrDeviceType = vrapi_GetSystemPropertyInt(&java, VRAPI_SYS_PROP_DEVICE_TYPE);
    //if (VRAPI_DEVICE_TYPE_GEARVR_START <= ovrDeviceType &&
    //    ovrDeviceType <= VRAPI_DEVICE_TYPE_GEARVR_END) {
    if (0 <= ovrDeviceType &&
    ovrDeviceType <= VRAPI_DEVICE_TYPE_OCULUSGO_START-1) {
        deviceSubType = ALVR_DEVICE_SUBTYPE_OCULUS_MOBILE_GEARVR;
    } else if (VRAPI_DEVICE_TYPE_OCULUSGO_START <= ovrDeviceType &&
               ovrDeviceType <= VRAPI_DEVICE_TYPE_OCULUSGO_END) {
        // Including MiVR.
        deviceSubType = ALVR_DEVICE_SUBTYPE_OCULUS_MOBILE_GO;
    } else if (VRAPI_DEVICE_TYPE_OCULUSQUEST_START <= ovrDeviceType &&
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
    jintArray refreshRates =
            reinterpret_cast<jintArray>(env->GetObjectField(deviceDescriptor, fieldID));
    getRefreshRates(env, refreshRates);
    env->SetObjectField(deviceDescriptor, fieldID, refreshRates);
    env->DeleteLocalRef(refreshRates);

    fieldID = env->GetFieldID(clazz, "mRenderWidth", "I");
    env->SetIntField(deviceDescriptor, fieldID, renderWidth);
    fieldID = env->GetFieldID(clazz, "mRenderHeight", "I");
    env->SetIntField(deviceDescriptor, fieldID, renderHeight);

    fieldID = env->GetFieldID(clazz, "mFov", "[F");
    jfloatArray fovField = reinterpret_cast<jfloatArray>(
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

    env->DeleteLocalRef(clazz);
}

std::pair<EyeFov, EyeFov> OvrContext::getFov() {
    float fovX = vrapi_GetSystemPropertyFloat(&java, VRAPI_SYS_PROP_SUGGESTED_EYE_FOV_DEGREES_X);
    float fovY = vrapi_GetSystemPropertyFloat(&java, VRAPI_SYS_PROP_SUGGESTED_EYE_FOV_DEGREES_Y);
    LOGI("OvrContext::getFov: X=%f Y=%f", fovX, fovY);

    double displayTime = vrapi_GetPredictedDisplayTime(Ovr, 0);
    ovrTracking2 tracking = vrapi_GetPredictedTracking2(Ovr, displayTime);

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
        LOGI("getFov maxX=%f minX=%f maxY=%f minY=%f a=%f b=%f c=%f d=%f n=%f", maxX, minX, maxY,
             minY, a, b, c, d, n);

        fov[eye].left = (float) (atan(minX / -n) * rr);
        fov[eye].right = (float) (-atan(maxX / -n) * rr);
        fov[eye].top = (float) (atan(minY / -n) * rr);
        fov[eye].bottom = (float) (-atan(maxY / -n) * rr);

        LOGI("getFov[%d](D) r=%f l=%f t=%f b=%f", eye, fov[eye].left, fov[eye].right,
             fov[eye].top, fov[eye].bottom);
    }
    return {fov[0], fov[1]};
}


void OvrContext::updateHapticsState() {
    ovrInputCapabilityHeader curCaps;
    ovrResult result;

    for (uint32_t deviceIndex = 0;
         vrapi_EnumerateInputDevices(Ovr, deviceIndex, &curCaps) >= 0; deviceIndex++) {
        ovrInputTrackedRemoteCapabilities remoteCapabilities;

        remoteCapabilities.Header = curCaps;
        result = vrapi_GetInputDeviceCapabilities(Ovr, &remoteCapabilities.Header);
        if (result != ovrSuccess) {
            continue;
        }

        int curHandIndex = (remoteCapabilities.ControllerCapabilities & ovrControllerCaps_LeftHand) ? 1 : 0;
        auto &s = mHapticsState[curHandIndex];

        uint64_t currentUs = getTimestampUs();

        if(s.startUs <= 0) {
            // No requested haptics for this hand.
            if(s.buffered) {
                finishHapticsBuffer(curCaps.DeviceID);
                s.buffered = false;
            }
            continue;
        }

        if(currentUs >= s.endUs) {
            // No more haptics is needed.
            s.startUs = 0;
            if(s.buffered) {
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

            uint32_t requiredHapticsBuffer = static_cast<uint32_t >((s.endUs - currentUs) / remoteCapabilities.HapticSampleDurationMS * 1000);

            std::vector<uint8_t> hapticBuffer(remoteCapabilities.HapticSamplesMax);
            ovrHapticBuffer buffer;
            buffer.BufferTime = vrapi_GetPredictedDisplayTime(Ovr, FrameIndex);
            buffer.HapticBuffer = &hapticBuffer[0];
            buffer.NumSamples = std::min(remoteCapabilities.HapticSamplesMax, requiredHapticsBuffer);
            buffer.Terminated = false;

            for (int i = 0; i < remoteCapabilities.HapticSamplesMax; i++) {
                float current = ((currentUs - s.startUs) / 1000000.0f) + (remoteCapabilities.HapticSampleDurationMS * i) / 1000.0f;
                float intensity =
                        sinf(static_cast<float>(current * M_PI * 2 * s.frequency)) * s.amplitude;
                if (intensity < 0) {
                    intensity = 0;
                } else if (intensity > 1.0) {
                    intensity = 1.0;
                }
                hapticBuffer[i] = static_cast<uint8_t>(255 * intensity);
            }

            result = vrapi_SetHapticVibrationBuffer(Ovr, curCaps.DeviceID, &buffer);
            if (result != ovrSuccess) {
                LOGI("vrapi_SetHapticVibrationBuffer: Failed. result=%d", result);
            }
            s.buffered = true;
        } else if (remoteCapabilities.ControllerCapabilities &
                   ovrControllerCaps_HasSimpleHapticVibration) {
            LOG("Send simple haptic. amplitude=%f", s.amplitude);
            vrapi_SetHapticVibrationSimple(Ovr, curCaps.DeviceID, s.amplitude);
        }
    }
}

void OvrContext::onHapticsFeedback(uint64_t startTime, float amplitude, float duration, float frequency,
                              int hand) {
    LOGI("OvrContext::onHapticsFeedback: processing haptics. %" PRIu64 " %f %f %f, %d", startTime, amplitude, duration, frequency, hand);

    int curHandIndex = (hand == 0) ? 0 : 1;
    auto &s = mHapticsState[curHandIndex];
    s.startUs = getTimestampUs() + startTime;
    s.endUs = s.startUs + static_cast<uint64_t>(duration * 1000000);
    s.amplitude = amplitude;
    s.frequency = frequency;
    s.buffered = false;
}

void OvrContext::finishHapticsBuffer(ovrDeviceID DeviceID) {
    uint8_t hapticBuffer[1] = {0};
    ovrHapticBuffer buffer;
    buffer.BufferTime = vrapi_GetPredictedDisplayTime(Ovr, FrameIndex);
    buffer.HapticBuffer = &hapticBuffer[0];
    buffer.NumSamples = 1;
    buffer.Terminated = true;

    auto result = vrapi_SetHapticVibrationBuffer(Ovr, DeviceID, &buffer);
    if (result != ovrSuccess) {
        LOGI("vrapi_SetHapticVibrationBuffer: Failed. result=%d", result);
    }
}

void OvrContext::reflectExtraLatencyMode(bool always) {
    if (always || (!gDisableExtraLatencyMode) != mExtraLatencyMode) {
        mExtraLatencyMode = !gDisableExtraLatencyMode;
        LOGI("Setting ExtraLatencyMode %s", mExtraLatencyMode ? "On" : "Off");
        ovrResult result = vrapi_SetExtraLatencyMode(Ovr, mExtraLatencyMode ? VRAPI_EXTRA_LATENCY_MODE_ON : VRAPI_EXTRA_LATENCY_MODE_OFF);
        LOGI("vrapi_SetExtraLatencyMode. Result=%d", result);
    }
}

/// Check if buttons to send launch signal to server is down on current frame.
/// \return true if down at current frame.
bool OvrContext::getButtonDown() {
    ovrInputCapabilityHeader curCaps;
    ovrResult result;
    bool buttonPressed = false;

    for (uint32_t deviceIndex = 0;
         vrapi_EnumerateInputDevices(Ovr, deviceIndex, &curCaps) >= 0; deviceIndex++) {
        if (curCaps.Type == ovrControllerType_TrackedRemote) {
            ovrInputTrackedRemoteCapabilities remoteCapabilities;
            ovrInputStateTrackedRemote remoteInputState;

            remoteCapabilities.Header = curCaps;
            result = vrapi_GetInputDeviceCapabilities(Ovr, &remoteCapabilities.Header);
            if (result != ovrSuccess) {
                continue;
            }
            remoteInputState.Header.ControllerType = remoteCapabilities.Header.Type;

            result = vrapi_GetCurrentInputState(Ovr, remoteCapabilities.Header.DeviceID,
                                                &remoteInputState.Header);
            if (result != ovrSuccess) {
                continue;
            }

            buttonPressed = buttonPressed || (remoteInputState.Buttons &
                    (ovrButton_Trigger | ovrButton_A | ovrButton_B | ovrButton_X | ovrButton_Y)) != 0;
        }
    }

    bool ret = buttonPressed && !mButtonPressed;
    mButtonPressed = buttonPressed;
    return ret;
}

extern "C"
JNIEXPORT jlong JNICALL
Java_com_polygraphene_alvr_OvrContext_initializeNative(JNIEnv *env, jobject instance,
                                                       jobject activity, jobject assetManager,
                                                       jobject vrThread, jboolean ARMode,
                                                       jint initialRefreshRate) {
    OvrContext *context = new OvrContext();
    context->initialize(env, activity, assetManager, vrThread, ARMode, initialRefreshRate);
    return (jlong) context;
}

extern "C"
JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrContext_destroyNative(JNIEnv *env, jobject instance, jlong handle) {
    ((OvrContext *) handle)->destroy(env);
}


extern "C"
JNIEXPORT jint JNICALL
Java_com_polygraphene_alvr_OvrContext_getLoadingTextureNative(JNIEnv *env, jobject instance,
                                                              jlong handle) {
    return ((OvrContext *) handle)->getLoadingTexture();
}

extern "C"
JNIEXPORT jint JNICALL
Java_com_polygraphene_alvr_OvrContext_getSurfaceTextureIDNative(JNIEnv *env, jobject instance,
                                                                jlong handle) {
    return ((OvrContext *) handle)->getSurfaceTextureID();
}

extern "C"
JNIEXPORT jint JNICALL
Java_com_polygraphene_alvr_OvrContext_getCameraTextureNative(JNIEnv *env, jobject instance,
                                                             jlong handle) {
    return ((OvrContext *) handle)->getCameraTexture();
}

extern "C"
JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrContext_renderNative(JNIEnv *env, jobject instance, jlong handle,
                                                   jlong renderedFrameIndex) {
    return ((OvrContext *) handle)->render(static_cast<uint64_t>(renderedFrameIndex));
}

extern "C"
JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrContext_renderLoadingNative(JNIEnv *env, jobject instance,
                                                          jlong handle) {
    return ((OvrContext *) handle)->renderLoading();
}

// Called from TrackingThread
extern "C"
JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrContext_sendTrackingInfoNative(JNIEnv *env, jobject instance,
                                                              jlong handle,
                                                              jobject udpReceiverThread
                                                             ) {
        ((OvrContext *) handle)->sendTrackingInfo(env, udpReceiverThread);
}

// Called from TrackingThread
extern "C"
JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrContext_sendMicDataNative(JNIEnv *env, jobject instance,
                                                             jlong handle,
                                                             jobject udpReceiverThread
) {
    ((OvrContext *) handle)->sendMicData(env, udpReceiverThread);
}

extern "C"
JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrContext_onChangeSettingsNative(JNIEnv *env, jobject instance,
                                                             jlong handle, jint Suspend) {
    ((OvrContext *) handle)->onChangeSettings(Suspend);
}

//
// Life cycle management.
//

extern "C"
JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrContext_onSurfaceCreatedNative(JNIEnv *env, jobject instance,
                                                             jlong handle,
                                                             jobject surface) {
    ((OvrContext *) handle)->onSurfaceCreated(surface);
}

extern "C"
JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrContext_onSurfaceDestroyedNative(JNIEnv *env, jobject instance,
                                                               jlong handle) {
    ((OvrContext *) handle)->onSurfaceDestroyed();
}

extern "C"
JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrContext_onSurfaceChangedNative(JNIEnv *env, jobject instance,
                                                             jlong handle, jobject surface) {
    ((OvrContext *) handle)->onSurfaceChanged(surface);
}

extern "C"
JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrContext_onResumeNative(JNIEnv *env, jobject instance, jlong handle) {
    ((OvrContext *) handle)->onResume();
}

extern "C"
JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrContext_onPauseNative(JNIEnv *env, jobject instance, jlong handle) {
    ((OvrContext *) handle)->onPause();
}

extern "C"
JNIEXPORT jboolean JNICALL
Java_com_polygraphene_alvr_OvrContext_isVrModeNative(JNIEnv *env, jobject instance, jlong handle) {
    return static_cast<jboolean>(((OvrContext *) handle)->isVrMode());
}

extern "C"
JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrContext_getDeviceDescriptorNative(JNIEnv *env, jobject instance,
                                                                jlong handle,
                                                                jobject deviceDescriptor) {
    ((OvrContext *) handle)->getDeviceDescriptor(env, deviceDescriptor);
}

extern "C"
JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrContext_setFrameGeometryNative(JNIEnv *env, jobject instance,
                                                             jlong handle, jint width,
                                                             jint height) {
    ((OvrContext *) handle)->setFrameGeometry(width, height);
}

extern "C"
JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrContext_setRefreshRateNative(JNIEnv *env, jobject instance,
                                                           jlong handle, jint refreshRate) {
    return ((OvrContext *) handle)->setRefreshRate(refreshRate);
}

extern "C"
JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrContext_setStreamMicNative(JNIEnv *env, jobject instance,
                                                           jlong handle, jboolean streamMic) {
    return ((OvrContext *) handle)->setStreamMic(streamMic);
}

extern "C"
JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrContext_setFFRParamsNative(JNIEnv *env, jobject instance,
                                                         jlong handle, jint foveationMode, jfloat foveationStrength, jfloat foveationShape, jfloat foveationVerticalOffset) {
    return ((OvrContext *) handle)->setFFRParams(foveationMode, foveationStrength, foveationShape, foveationVerticalOffset);
}

extern "C"
JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrContext_onHapticsFeedbackNative(JNIEnv *env, jobject instance,
                                                              jlong handle, jlong startTime,
                                                              jfloat amplitude, jfloat duration,
                                                              jfloat frequency, jboolean hand) {
    return ((OvrContext *) handle)->onHapticsFeedback(static_cast<uint64_t>(startTime), amplitude,
                                                      duration, frequency, hand == 0 ? 0 : 1);
}

extern "C"
JNIEXPORT jboolean JNICALL
Java_com_polygraphene_alvr_OvrContext_getButtonDownNative(JNIEnv *env, jobject instance,
                                                          jlong handle) {

    return static_cast<jboolean>(((OvrContext *) handle)->getButtonDown());
}