#include "Controller.h"
#include "Logger.h"
#include "Paths.h"
#include "Settings.h"
#include "Utils.h"
#include "include/openvr_math.h"
#include <algorithm>
#include <cstring>
#include <string_view>

vr::ETrackedDeviceClass Controller::getControllerDeviceClass() {
    if (Settings::Instance().m_controllerIsTracker)
        return vr::TrackedDeviceClass_GenericTracker;
    return vr::TrackedDeviceClass_Controller;
}

Controller::Controller(uint64_t deviceID) : TrackedDevice(deviceID) {
    m_pose = vr::DriverPose_t{};
    m_pose.poseIsValid = false;
    m_pose.deviceIsConnected = false;
    m_pose.result = vr::TrackingResult_Uninitialized;

    m_pose.qDriverFromHeadRotation = HmdQuaternion_Init(1, 0, 0, 0);
    m_pose.qWorldFromDriverRotation = HmdQuaternion_Init(1, 0, 0, 0);
    m_pose.qRotation = HmdQuaternion_Init(1, 0, 0, 0);
}

//
// ITrackedDeviceServerDriver
//

vr::EVRInitError Controller::Activate(vr::TrackedDeviceIndex_t unObjectId) {
    Debug("RemoteController::Activate. objectId=%d\n", unObjectId);

    auto vr_properties = vr::VRProperties();
    auto vr_driver_input = vr::VRDriverInput();

    this->object_id = unObjectId;
    this->prop_container = vr_properties->TrackedDeviceToPropertyContainer(this->object_id);

    SetOpenvrProps(this->device_id);

    RegisterButtons(this->device_id);

    vr_driver_input->CreateHapticComponent(this->prop_container, "/output/haptic", &m_compHaptic);

    vr_driver_input->CreateScalarComponent(this->prop_container,
                                           "/input/finger/index",
                                           &m_buttonHandles[ALVR_INPUT_FINGER_INDEX],
                                           vr::VRScalarType_Absolute,
                                           vr::VRScalarUnits_NormalizedOneSided);
    vr_driver_input->CreateScalarComponent(this->prop_container,
                                           "/input/finger/middle",
                                           &m_buttonHandles[ALVR_INPUT_FINGER_MIDDLE],
                                           vr::VRScalarType_Absolute,
                                           vr::VRScalarUnits_NormalizedOneSided);
    vr_driver_input->CreateScalarComponent(this->prop_container,
                                           "/input/finger/ring",
                                           &m_buttonHandles[ALVR_INPUT_FINGER_RING],
                                           vr::VRScalarType_Absolute,
                                           vr::VRScalarUnits_NormalizedOneSided);
    vr_driver_input->CreateScalarComponent(this->prop_container,
                                           "/input/finger/pinky",
                                           &m_buttonHandles[ALVR_INPUT_FINGER_PINKY],
                                           vr::VRScalarType_Absolute,
                                           vr::VRScalarUnits_NormalizedOneSided);

    if (this->device_id == HAND_LEFT_ID) {
        vr_driver_input->CreateSkeletonComponent(
            this->prop_container,
            "/input/skeleton/left",
            "/skeleton/hand/left",
            "/pose/raw",
            vr::EVRSkeletalTrackingLevel::VRSkeletalTracking_Partial,
            nullptr,
            0U,
            &m_compSkeleton);
    } else {
        vr_driver_input->CreateSkeletonComponent(
            this->prop_container,
            "/input/skeleton/right",
            "/skeleton/hand/right",
            "/pose/raw",
            vr::EVRSkeletalTrackingLevel::VRSkeletalTracking_Partial,
            nullptr,
            0U,
            &m_compSkeleton);
    }

    // NB: here we set some initial values for the hand skeleton to fix the frozen hand bug
    {
        vr::VRBoneTransform_t boneTransforms[SKELETON_BONE_COUNT];
        GetBoneTransform(false, boneTransforms);

        vr_driver_input->UpdateSkeletonComponent(m_compSkeleton,
                                                 vr::VRSkeletalMotionRange_WithController,
                                                 boneTransforms,
                                                 SKELETON_BONE_COUNT);
        vr_driver_input->UpdateSkeletonComponent(m_compSkeleton,
                                                 vr::VRSkeletalMotionRange_WithoutController,
                                                 boneTransforms,
                                                 SKELETON_BONE_COUNT);
    }

    return vr::VRInitError_None;
}

void Controller::Deactivate() {
    Debug("RemoteController::Deactivate\n");
    this->object_id = vr::k_unTrackedDeviceIndexInvalid;
}

void Controller::EnterStandby() {}

void *Controller::GetComponent(const char *pchComponentNameAndVersion) {
    Debug("RemoteController::GetComponent. Name=%hs\n", pchComponentNameAndVersion);

    return NULL;
}

void PowerOff() {}

/** debug request from a client */
void Controller::DebugRequest(const char * /*pchRequest*/,
                              char *pchResponseBuffer,
                              uint32_t unResponseBufferSize) {
    if (unResponseBufferSize >= 1)
        pchResponseBuffer[0] = 0;
}

vr::DriverPose_t Controller::GetPose() { return m_pose; }

vr::VRInputComponentHandle_t Controller::getHapticComponent() { return m_compHaptic; }

void Controller::RegisterButton(uint64_t id) {
    ButtonInfo buttonInfo;
    if (device_id == HAND_LEFT_ID) {
        buttonInfo = LEFT_CONTROLLER_BUTTON_MAPPING[id];
    } else {
        buttonInfo = RIGHT_CONTROLLER_BUTTON_MAPPING[id];
    }

    if (buttonInfo.type == ButtonType::Binary) {
        for (auto path : buttonInfo.steamvr_paths) {
            vr::VRDriverInput()->CreateBooleanComponent(
                this->prop_container, path, &m_buttonHandles[PathStringToHash(path)]);
        }
    } else {
        auto scalarType = buttonInfo.type == ButtonType::ScalarOneSided
                              ? vr::VRScalarUnits_NormalizedOneSided
                              : vr::VRScalarUnits_NormalizedTwoSided;

        for (auto path : buttonInfo.steamvr_paths) {
            vr::VRDriverInput()->CreateScalarComponent(this->prop_container,
                                                       path,
                                                       &m_buttonHandles[PathStringToHash(path)],
                                                       vr::VRScalarType_Absolute,
                                                       scalarType);
        }
    }
}

void Controller::SetButton(uint64_t id, FfiButtonValue value) {
    for (auto id : ALVR_TO_STEAMVR_PATH_IDS[id]) {
        if (value.type == BUTTON_TYPE_BINARY) {
            vr::VRDriverInput()->UpdateBooleanComponent(
                m_buttonHandles[id], (bool)value.binary, 0.0);
        } else {
            vr::VRDriverInput()->UpdateScalarComponent(m_buttonHandles[id], value.scalar, 0.0);
        }
    }

    // todo: remove when moving inferred controller hand skeleton to rust
    if (id == LEFT_A_TOUCH_ID || id == LEFT_B_TOUCH_ID || id == LEFT_X_TOUCH_ID ||
        id == LEFT_Y_TOUCH_ID || id == LEFT_TRACKPAD_TOUCH_ID || id == LEFT_THUMBSTICK_TOUCH_ID ||
        id == LEFT_THUMBREST_TOUCH_ID || id == RIGHT_A_TOUCH_ID || id == RIGHT_B_TOUCH_ID ||
        id == RIGHT_TRACKPAD_TOUCH_ID || id == RIGHT_THUMBSTICK_TOUCH_ID ||
        id == RIGHT_THUMBREST_TOUCH_ID) {
        m_currentThumbTouch = value.binary;
    } else if (id == LEFT_TRIGGER_TOUCH_ID || id == RIGHT_TRIGGER_TOUCH_ID) {
        m_currentTriggerTouch = value.binary;
    } else if (id == LEFT_TRIGGER_VALUE_ID || id == RIGHT_TRIGGER_VALUE_ID) {
        m_triggerValue = value.scalar;
    } else if (id == LEFT_SQUEEZE_VALUE_ID || id == RIGHT_SQUEEZE_VALUE_ID) {
        m_gripValue = value.scalar;
    }
}

bool Controller::onPoseUpdate(float predictionS,
                              FfiDeviceMotion motion,
                              const FfiHandSkeleton *handSkeleton,
                              unsigned int controllersTracked) {
    if (this->object_id == vr::k_unTrackedDeviceIndexInvalid) {
        return false;
    }

    auto vr_driver_input = vr::VRDriverInput();

    auto pose = vr::DriverPose_t{};
    pose.poseIsValid = controllersTracked;
    pose.deviceIsConnected = controllersTracked;
    pose.result =
        controllersTracked ? vr::TrackingResult_Running_OK : vr::TrackingResult_Uninitialized;

    pose.qDriverFromHeadRotation = HmdQuaternion_Init(1, 0, 0, 0);
    pose.qWorldFromDriverRotation = HmdQuaternion_Init(1, 0, 0, 0);

    pose.qRotation = HmdQuaternion_Init(motion.orientation.w,
                                        motion.orientation.x,
                                        motion.orientation.y,
                                        motion.orientation.z); // controllerRotation;

    pose.vecPosition[0] = motion.position[0];
    pose.vecPosition[1] = motion.position[1];
    pose.vecPosition[2] = motion.position[2];

    pose.vecVelocity[0] = motion.linearVelocity[0];
    pose.vecVelocity[1] = motion.linearVelocity[1];
    pose.vecVelocity[2] = motion.linearVelocity[2];

    pose.vecAngularVelocity[0] = motion.angularVelocity[0];
    pose.vecAngularVelocity[1] = motion.angularVelocity[1];
    pose.vecAngularVelocity[2] = motion.angularVelocity[2];

    pose.poseTimeOffset = predictionS;

    m_pose = pose;

    if (handSkeleton != nullptr) {
        vr::VRBoneTransform_t boneTransform[SKELETON_BONE_COUNT] = {};
        for (int j = 0; j < 31; j++) {
            boneTransform[j].orientation.w = handSkeleton->jointRotations[j].w;
            boneTransform[j].orientation.x = handSkeleton->jointRotations[j].x;
            boneTransform[j].orientation.y = handSkeleton->jointRotations[j].y;
            boneTransform[j].orientation.z = handSkeleton->jointRotations[j].z;
            boneTransform[j].position.v[0] = handSkeleton->jointPositions[j][0];
            boneTransform[j].position.v[1] = handSkeleton->jointPositions[j][1];
            boneTransform[j].position.v[2] = handSkeleton->jointPositions[j][2];
            boneTransform[j].position.v[3] = 1.0;
        }

        vr_driver_input->UpdateSkeletonComponent(m_compSkeleton,
                                                 vr::VRSkeletalMotionRange_WithController,
                                                 boneTransform,
                                                 SKELETON_BONE_COUNT);
        vr_driver_input->UpdateSkeletonComponent(m_compSkeleton,
                                                 vr::VRSkeletalMotionRange_WithoutController,
                                                 boneTransform,
                                                 SKELETON_BONE_COUNT);

        float rotThumb = (handSkeleton->jointRotations[2].z + handSkeleton->jointRotations[2].y +
                          handSkeleton->jointRotations[3].z + handSkeleton->jointRotations[3].y +
                          handSkeleton->jointRotations[4].z + handSkeleton->jointRotations[4].y) *
                         0.67f;
        float rotIndex = (handSkeleton->jointRotations[7].z + handSkeleton->jointRotations[8].z +
                          handSkeleton->jointRotations[9].z) *
                         0.67f;
        float rotMiddle = (handSkeleton->jointRotations[12].z + handSkeleton->jointRotations[13].z +
                           handSkeleton->jointRotations[14].z) *
                          0.67f;
        float rotRing = (handSkeleton->jointRotations[17].z + handSkeleton->jointRotations[18].z +
                         handSkeleton->jointRotations[19].z) *
                        0.67f;
        float rotPinky = (handSkeleton->jointRotations[22].z + handSkeleton->jointRotations[23].z +
                          handSkeleton->jointRotations[24].z) *
                         0.67f;

        vr_driver_input->UpdateScalarComponent(
            m_buttonHandles[ALVR_INPUT_FINGER_INDEX], rotIndex, 0.0);
        vr_driver_input->UpdateScalarComponent(
            m_buttonHandles[ALVR_INPUT_FINGER_MIDDLE], rotMiddle, 0.0);
        vr_driver_input->UpdateScalarComponent(
            m_buttonHandles[ALVR_INPUT_FINGER_RING], rotRing, 0.0);
        vr_driver_input->UpdateScalarComponent(
            m_buttonHandles[ALVR_INPUT_FINGER_PINKY], rotPinky, 0.0);
    } else {
        if (m_lastThumbTouch != m_currentThumbTouch) {
            m_thumbTouchAnimationProgress += 1.f / ANIMATION_FRAME_COUNT;
            if (m_thumbTouchAnimationProgress > 1.f) {
                m_thumbTouchAnimationProgress = 0;
                m_lastThumbTouch = m_currentThumbTouch;
            }
        } else {
            m_thumbTouchAnimationProgress = 0;
        }

        if (m_lastTriggerTouch != m_currentTriggerTouch) {
            m_indexTouchAnimationProgress += 1.f / ANIMATION_FRAME_COUNT;
            if (m_indexTouchAnimationProgress > 1.f) {
                m_indexTouchAnimationProgress = 0;
                m_lastTriggerTouch = m_currentTriggerTouch;
            }
        } else {
            m_indexTouchAnimationProgress = 0;
        }

        float indexCurl = 0.0;
        if (m_triggerValue > 0.0) {
            indexCurl = 0.5 + m_triggerValue * 0.5;
        } else if (m_lastTriggerTouch == 0) {
            indexCurl = m_indexTouchAnimationProgress * 0.5;
        } else {
            indexCurl = 0.5 - m_indexTouchAnimationProgress * 0.5;
        }
        vr_driver_input->UpdateScalarComponent(
            m_buttonHandles[ALVR_INPUT_FINGER_INDEX], indexCurl, 0.0);

        vr_driver_input->UpdateScalarComponent(
            m_buttonHandles[ALVR_INPUT_FINGER_MIDDLE], m_gripValue, 0.0);

        // Ring and pinky fingers are not tracked. Infer a more natural pose.
        if (m_currentThumbTouch) {
            vr_driver_input->UpdateScalarComponent(m_buttonHandles[ALVR_INPUT_FINGER_RING], 1, 0.0);
            vr_driver_input->UpdateScalarComponent(
                m_buttonHandles[ALVR_INPUT_FINGER_PINKY], 1, 0.0);
        } else {
            vr_driver_input->UpdateScalarComponent(
                m_buttonHandles[ALVR_INPUT_FINGER_RING], m_gripValue, 0.0);
            vr_driver_input->UpdateScalarComponent(
                m_buttonHandles[ALVR_INPUT_FINGER_PINKY], m_gripValue, 0.0);
        }

        vr::VRBoneTransform_t boneTransforms[SKELETON_BONE_COUNT];

        // Perform whatever logic is necessary to convert your device's input into a
        // skeletal pose, first to create a pose "With Controller", that is as close to the
        // pose of the user's real hand as possible
        GetBoneTransform(true, boneTransforms);

        // Then update the WithController pose on the component with those transforms
        vr::EVRInputError err =
            vr_driver_input->UpdateSkeletonComponent(m_compSkeleton,
                                                     vr::VRSkeletalMotionRange_WithController,
                                                     boneTransforms,
                                                     SKELETON_BONE_COUNT);
        if (err != vr::VRInputError_None) {
            // Handle failure case
            Error("UpdateSkeletonComponentfailed.  Error: %i\n", err);
        }

        GetBoneTransform(false, boneTransforms);

        // Then update the WithoutController pose on the component
        err = vr_driver_input->UpdateSkeletonComponent(m_compSkeleton,
                                                       vr::VRSkeletalMotionRange_WithoutController,
                                                       boneTransforms,
                                                       SKELETON_BONE_COUNT);
        if (err != vr::VRInputError_None) {
            // Handle failure case
            Error("UpdateSkeletonComponentfailed.  Error: %i\n", err);
        }
    }

    vr::VRServerDriverHost()->TrackedDevicePoseUpdated(
        this->object_id, pose, sizeof(vr::DriverPose_t));

    return false;
}

void GetThumbBoneTransform(bool withController,
                           bool isLeftHand,
                           bool touch,
                           vr::VRBoneTransform_t outBoneTransform[]) {
    if (isLeftHand) {
        if (touch) {
            if (withController) {
                outBoneTransform[2] = {{-0.017303f, 0.032567f, 0.025281f, 1.f},
                                       {0.317609f, 0.528344f, 0.213134f, 0.757991f}};
                outBoneTransform[3] = {{0.040406f, 0.000000f, -0.000000f, 1.f},
                                       {0.991742f, 0.085317f, 0.019416f, 0.093765f}};
                outBoneTransform[4] = {{0.032517f, -0.000000f, 0.000000f, 1.f},
                                       {0.959385f, -0.012202f, -0.031055f, 0.280120f}};
            } else {
                outBoneTransform[2] = {{-0.016426f, 0.030866f, 0.025118f, 1.f},
                                       {0.403850f, 0.595704f, 0.082451f, 0.689380f}};
                outBoneTransform[3] = {{0.040406f, 0.000000f, -0.000000f, 1.f},
                                       {0.989655f, -0.090426f, 0.028457f, 0.107691f}};
                outBoneTransform[4] = {{0.032517f, 0.000000f, 0.000000f, 1.f},
                                       {0.988590f, 0.143978f, 0.041520f, 0.015363f}};
            }
        } else {
            outBoneTransform[2] = {{-0.012083f, 0.028070f, 0.025050f, 1},
                                   {0.464112f, 0.567418f, 0.272106f, 0.623374f}};
            outBoneTransform[3] = {{0.040406f, 0.000000f, -0.000000f, 1},
                                   {0.994838f, 0.082939f, 0.019454f, 0.055130f}};
            outBoneTransform[4] = {{0.032517f, 0.000000f, 0.000000f, 1},
                                   {0.974793f, -0.003213f, 0.021867f, -0.222015f}};
        }

        outBoneTransform[5] = {{0.030464f, -0.000000f, -0.000000f, 1},
                               {1.000000f, -0.000000f, 0.000000f, 0.000000f}};
    } else {
        if (touch) {
            if (withController) {
                outBoneTransform[2] = {{0.017303f, 0.032567f, 0.025281f, 1},
                                       {0.528344f, -0.317609f, 0.757991f, -0.213134f}};
                outBoneTransform[3] = {{-0.040406f, -0.000000f, 0.000000f, 1},
                                       {0.991742f, 0.085317f, 0.019416f, 0.093765f}};
                outBoneTransform[4] = {{-0.032517f, 0.000000f, -0.000000f, 1},
                                       {0.959385f, -0.012202f, -0.031055f, 0.280120f}};
            } else {
                outBoneTransform[2] = {{0.016426f, 0.030866f, 0.025118f, 1},
                                       {0.595704f, -0.403850f, 0.689380f, -0.082451f}};
                outBoneTransform[3] = {{-0.040406f, -0.000000f, 0.000000f, 1},
                                       {0.989655f, -0.090426f, 0.028457f, 0.107691f}};
                outBoneTransform[4] = {{-0.032517f, -0.000000f, -0.000000f, 1},
                                       {0.988590f, 0.143978f, 0.041520f, 0.015363f}};
            }
        } else {
            outBoneTransform[2] = {{0.012330f, 0.028661f, 0.025049f, 1},
                                   {0.571059f, -0.451277f, 0.630056f, -0.270685f}};
            outBoneTransform[3] = {{-0.040406f, -0.000000f, 0.000000f, 1},
                                   {0.994565f, 0.078280f, 0.018282f, 0.066177f}};
            outBoneTransform[4] = {{-0.032517f, -0.000000f, -0.000000f, 1},
                                   {0.977658f, -0.003039f, 0.020722f, -0.209156f}};
        }

        outBoneTransform[5] = {{-0.030464f, 0.000000f, 0.000000f, 1},
                               {1.000000f, -0.000000f, 0.000000f, 0.000000f}};
    }
}

void GetTriggerBoneTransform(bool withController,
                             bool isLeftHand,
                             bool touch,
                             float click,
                             vr::VRBoneTransform_t outBoneTransform[]) {
    if (click) {
        if (withController) {
            if (isLeftHand) {
                outBoneTransform[6] = {{-0.003925f, 0.027171f, 0.014640f, 1},
                                       {0.666448f, 0.430031f, -0.455947f, 0.403772f}};
                outBoneTransform[7] = {{0.076015f, -0.005124f, 0.000239f, 1},
                                       {-0.956011f, -0.000025f, 0.158355f, -0.246913f}};
                outBoneTransform[8] = {{0.043930f, -0.000000f, -0.000000f, 1},
                                       {-0.944138f, -0.043351f, 0.014947f, -0.326345f}};
                outBoneTransform[9] = {{0.028695f, 0.000000f, 0.000000f, 1},
                                       {-0.912149f, 0.003626f, 0.039888f, -0.407898f}};
                outBoneTransform[10] = {{0.022821f, 0.000000f, -0.000000f, 1},
                                        {1.000000f, -0.000000f, -0.000000f, 0.000000f}};
                outBoneTransform[11] = {{0.002177f, 0.007120f, 0.016319f, 1},
                                        {0.529359f, 0.540512f, -0.463783f, 0.461011f}};
                outBoneTransform[12] = {{0.070953f, 0.000779f, 0.000997f, 1},
                                        {0.847397f, -0.257141f, -0.139135f, 0.443213f}};
                outBoneTransform[13] = {{0.043108f, 0.000000f, 0.000000f, 1},
                                        {0.874907f, 0.009875f, 0.026584f, 0.483460f}};
                outBoneTransform[14] = {{0.033266f, -0.000000f, 0.000000f, 1},
                                        {0.894578f, -0.036774f, -0.050597f, 0.442513f}};
                outBoneTransform[15] = {{0.025892f, -0.000000f, 0.000000f, 1},
                                        {0.999195f, -0.000000f, 0.000000f, 0.040126f}};
                outBoneTransform[16] = {{0.000513f, -0.006545f, 0.016348f, 1},
                                        {0.500244f, 0.530784f, -0.516215f, 0.448939f}};
                outBoneTransform[17] = {{0.065876f, 0.001786f, 0.000693f, 1},
                                        {0.831617f, -0.242931f, -0.139695f, 0.479461f}};
                outBoneTransform[18] = {{0.040697f, 0.000000f, 0.000000f, 1},
                                        {0.769163f, -0.001746f, 0.001363f, 0.639049f}};
                outBoneTransform[19] = {{0.028747f, -0.000000f, -0.000000f, 1},
                                        {0.968615f, -0.064538f, -0.046586f, 0.235477f}};
                outBoneTransform[20] = {{0.022430f, -0.000000f, 0.000000f, 1},
                                        {1.000000f, 0.000000f, -0.000000f, -0.000000f}};
                outBoneTransform[21] = {{-0.002478f, -0.018981f, 0.015214f, 1},
                                        {0.474671f, 0.434670f, -0.653212f, 0.398827f}};
                outBoneTransform[22] = {{0.062878f, 0.002844f, 0.000332f, 1},
                                        {0.798788f, -0.199577f, -0.094418f, 0.559636f}};
                outBoneTransform[23] = {{0.030220f, 0.000002f, -0.000000f, 1},
                                        {0.853087f, 0.001644f, -0.000913f, 0.521765f}};
                outBoneTransform[24] = {{0.018187f, -0.000002f, 0.000000f, 1},
                                        {0.974249f, 0.052491f, 0.003591f, 0.219249f}};
                outBoneTransform[25] = {{0.018018f, 0.000000f, -0.000000f, 1},
                                        {1.000000f, 0.000000f, 0.000000f, 0.000000f}};
                outBoneTransform[26] = {{0.006629f, 0.026690f, 0.061870f, 1},
                                        {0.805084f, -0.018369f, 0.584788f, -0.097597f}};
                outBoneTransform[27] = {{-0.007882f, -0.040478f, 0.039337f, 1},
                                        {-0.322494f, 0.932092f, 0.121861f, 0.111140f}};
                outBoneTransform[28] = {{0.017136f, -0.032633f, 0.080682f, 1},
                                        {-0.169466f, 0.800083f, 0.571006f, 0.071415f}};
                outBoneTransform[29] = {{0.011144f, -0.028727f, 0.108366f, 1},
                                        {-0.076328f, 0.788280f, 0.605097f, 0.081527f}};
                outBoneTransform[30] = {{0.011333f, -0.026044f, 0.128585f, 1},
                                        {-0.144791f, 0.737451f, 0.656958f, -0.060069f}};
            } else {
                outBoneTransform[6] = {{-0.003925f, 0.027171f, 0.014640f, 1},
                                       {0.666448f, 0.430031f, -0.455947f, 0.403772f}};
                outBoneTransform[7] = {{0.076015f, -0.005124f, 0.000239f, 1},
                                       {-0.956011f, -0.000025f, 0.158355f, -0.246913f}};
                outBoneTransform[8] = {{0.043930f, -0.000000f, -0.000000f, 1},
                                       {-0.944138f, -0.043351f, 0.014947f, -0.326345f}};
                outBoneTransform[9] = {{0.028695f, 0.000000f, 0.000000f, 1},
                                       {-0.912149f, 0.003626f, 0.039888f, -0.407898f}};
                outBoneTransform[10] = {{0.022821f, 0.000000f, -0.000000f, 1},
                                        {1.000000f, -0.000000f, -0.000000f, 0.000000f}};
                outBoneTransform[11] = {{0.002177f, 0.007120f, 0.016319f, 1},
                                        {0.529359f, 0.540512f, -0.463783f, 0.461011f}};
                outBoneTransform[12] = {{0.070953f, 0.000779f, 0.000997f, 1},
                                        {0.847397f, -0.257141f, -0.139135f, 0.443213f}};
                outBoneTransform[13] = {{0.043108f, 0.000000f, 0.000000f, 1},
                                        {0.874907f, 0.009875f, 0.026584f, 0.483460f}};
                outBoneTransform[14] = {{0.033266f, -0.000000f, 0.000000f, 1},
                                        {0.894578f, -0.036774f, -0.050597f, 0.442513f}};
                outBoneTransform[15] = {{0.025892f, -0.000000f, 0.000000f, 1},
                                        {0.999195f, -0.000000f, 0.000000f, 0.040126f}};
                outBoneTransform[16] = {{0.000513f, -0.006545f, 0.016348f, 1},
                                        {0.500244f, 0.530784f, -0.516215f, 0.448939f}};
                outBoneTransform[17] = {{0.065876f, 0.001786f, 0.000693f, 1},
                                        {0.831617f, -0.242931f, -0.139695f, 0.479461f}};
                outBoneTransform[18] = {{0.040697f, 0.000000f, 0.000000f, 1},
                                        {0.769163f, -0.001746f, 0.001363f, 0.639049f}};
                outBoneTransform[19] = {{0.028747f, -0.000000f, -0.000000f, 1},
                                        {0.968615f, -0.064538f, -0.046586f, 0.235477f}};
                outBoneTransform[20] = {{0.022430f, -0.000000f, 0.000000f, 1},
                                        {1.000000f, 0.000000f, -0.000000f, -0.000000f}};
                outBoneTransform[21] = {{-0.002478f, -0.018981f, 0.015214f, 1},
                                        {0.474671f, 0.434670f, -0.653212f, 0.398827f}};
                outBoneTransform[22] = {{0.062878f, 0.002844f, 0.000332f, 1},
                                        {0.798788f, -0.199577f, -0.094418f, 0.559636f}};
                outBoneTransform[23] = {{0.030220f, 0.000002f, -0.000000f, 1},
                                        {0.853087f, 0.001644f, -0.000913f, 0.521765f}};
                outBoneTransform[24] = {{0.018187f, -0.000002f, 0.000000f, 1},
                                        {0.974249f, 0.052491f, 0.003591f, 0.219249f}};
                outBoneTransform[25] = {{0.018018f, 0.000000f, -0.000000f, 1},
                                        {1.000000f, 0.000000f, 0.000000f, 0.000000f}};
                outBoneTransform[26] = {{0.006629f, 0.026690f, 0.061870f, 1},
                                        {0.805084f, -0.018369f, 0.584788f, -0.097597f}};
                outBoneTransform[27] = {{-0.007882f, -0.040478f, 0.039337f, 1},
                                        {-0.322494f, 0.932092f, 0.121861f, 0.111140f}};
                outBoneTransform[28] = {{0.017136f, -0.032633f, 0.080682f, 1},
                                        {-0.169466f, 0.800083f, 0.571006f, 0.071415f}};
                outBoneTransform[29] = {{0.011144f, -0.028727f, 0.108366f, 1},
                                        {-0.076328f, 0.788280f, 0.605097f, 0.081527f}};
                outBoneTransform[30] = {{0.011333f, -0.026044f, 0.128585f, 1},
                                        {-0.144791f, 0.737451f, 0.656958f, -0.060069f}};
            }
        } else {
            if (isLeftHand) {
                outBoneTransform[6] = {{0.003802f, 0.021514f, 0.012803f, 1},
                                       {0.617314f, 0.395175f, -0.510874f, 0.449185f}};
                outBoneTransform[7] = {{0.074204f, -0.005002f, 0.000234f, 1},
                                       {0.737291f, -0.032006f, -0.115013f, 0.664944f}};
                outBoneTransform[8] = {{0.043287f, -0.000000f, -0.000000f, 1},
                                       {0.611381f, 0.003287f, 0.003823f, 0.791320f}};
                outBoneTransform[9] = {{0.028275f, 0.000000f, 0.000000f, 1},
                                       {0.745389f, -0.000684f, -0.000945f, 0.666629f}};
                outBoneTransform[10] = {{0.022821f, 0.000000f, -0.000000f, 1},
                                        {1.000000f, 0.000000f, -0.000000f, 0.000000f}};
                outBoneTransform[11] = {{0.004885f, 0.006885f, 0.016480f, 1},
                                        {0.522678f, 0.527374f, -0.469333f, 0.477923f}};
                outBoneTransform[12] = {{0.070953f, 0.000779f, 0.000997f, 1},
                                        {0.826071f, -0.121321f, 0.017267f, 0.550082f}};
                outBoneTransform[13] = {{0.043108f, 0.000000f, 0.000000f, 1},
                                        {0.956676f, 0.013210f, 0.009330f, 0.290704f}};
                outBoneTransform[14] = {{0.033266f, 0.000000f, 0.000000f, 1},
                                        {0.979740f, -0.001605f, -0.019412f, 0.199323f}};
                outBoneTransform[15] = {{0.025892f, -0.000000f, 0.000000f, 1},
                                        {0.999195f, 0.000000f, 0.000000f, 0.040126f}};
                outBoneTransform[16] = {{0.001696f, -0.006648f, 0.016418f, 1},
                                        {0.509620f, 0.540794f, -0.504891f, 0.439220f}};
                outBoneTransform[17] = {{0.065876f, 0.001786f, 0.000693f, 1},
                                        {0.955009f, -0.065344f, -0.063228f, 0.282294f}};
                outBoneTransform[18] = {{0.040577f, 0.000000f, 0.000000f, 1},
                                        {0.953823f, -0.000972f, 0.000697f, 0.300366f}};
                outBoneTransform[19] = {{0.028698f, -0.000000f, -0.000000f, 1},
                                        {0.977627f, -0.001163f, -0.011433f, 0.210033f}};
                outBoneTransform[20] = {{0.022430f, -0.000000f, 0.000000f, 1},
                                        {1.000000f, 0.000000f, 0.000000f, 0.000000f}};
                outBoneTransform[21] = {{-0.001792f, -0.019041f, 0.015254f, 1},
                                        {0.518602f, 0.511152f, -0.596086f, 0.338315f}};
                outBoneTransform[22] = {{0.062878f, 0.002844f, 0.000332f, 1},
                                        {0.978584f, -0.045398f, -0.103083f, 0.172297f}};
                outBoneTransform[23] = {{0.030154f, 0.000000f, 0.000000f, 1},
                                        {0.970479f, -0.000068f, -0.002025f, 0.241175f}};
                outBoneTransform[24] = {{0.018187f, 0.000000f, 0.000000f, 1},
                                        {0.997053f, -0.000687f, -0.052009f, -0.056395f}};
                outBoneTransform[25] = {{0.018018f, 0.000000f, -0.000000f, 1},
                                        {1.000000f, -0.000000f, -0.000000f, -0.000000f}};
                outBoneTransform[26] = {{-0.005193f, 0.054191f, 0.060030f, 1},
                                        {0.747374f, 0.182388f, 0.599615f, 0.220518f}};
                outBoneTransform[27] = {{0.000171f, 0.016473f, 0.096515f, 1},
                                        {-0.006456f, 0.022747f, -0.932927f, -0.359287f}};
                outBoneTransform[28] = {{-0.038019f, -0.074839f, 0.046941f, 1},
                                        {-0.199973f, 0.698334f, -0.635627f, -0.261380f}};
                outBoneTransform[29] = {{-0.036836f, -0.089774f, 0.081969f, 1},
                                        {-0.191006f, 0.756582f, -0.607429f, -0.148761f}};
                outBoneTransform[30] = {{-0.030241f, -0.086049f, 0.119881f, 1},
                                        {-0.019037f, 0.779368f, -0.612017f, -0.132881f}};
            } else {
                outBoneTransform[6] = {{-0.003802f, 0.021514f, 0.012803f, 1},
                                       {0.395174f, -0.617314f, 0.449185f, 0.510874f}};
                outBoneTransform[7] = {{-0.074204f, 0.005002f, -0.000234f, 1},
                                       {0.737291f, -0.032006f, -0.115013f, 0.664944f}};
                outBoneTransform[8] = {{-0.043287f, 0.000000f, 0.000000f, 1},
                                       {0.611381f, 0.003287f, 0.003823f, 0.791320f}};
                outBoneTransform[9] = {{-0.028275f, -0.000000f, -0.000000f, 1},
                                       {0.745389f, -0.000684f, -0.000945f, 0.666629f}};
                outBoneTransform[10] = {{-0.022821f, -0.000000f, 0.000000f, 1},
                                        {1.000000f, 0.000000f, -0.000000f, 0.000000f}};
                outBoneTransform[11] = {{-0.004885f, 0.006885f, 0.016480f, 1},
                                        {0.527233f, -0.522513f, 0.478085f, 0.469510f}};
                outBoneTransform[12] = {{-0.070953f, -0.000779f, -0.000997f, 1},
                                        {0.826317f, -0.120120f, 0.019005f, 0.549918f}};
                outBoneTransform[13] = {{-0.043108f, -0.000000f, -0.000000f, 1},
                                        {0.958363f, 0.013484f, 0.007380f, 0.285138f}};
                outBoneTransform[14] = {{-0.033266f, -0.000000f, -0.000000f, 1},
                                        {0.977901f, -0.001431f, -0.018078f, 0.208279f}};
                outBoneTransform[15] = {{-0.025892f, 0.000000f, -0.000000f, 1},
                                        {0.999195f, 0.000000f, 0.000000f, 0.040126f}};
                outBoneTransform[16] = {{-0.001696f, -0.006648f, 0.016418f, 1},
                                        {0.541481f, -0.508179f, 0.441001f, 0.504054f}};
                outBoneTransform[17] = {{-0.065876f, -0.001786f, -0.000693f, 1},
                                        {0.953780f, -0.064506f, -0.058812f, 0.287548f}};
                outBoneTransform[18] = {{-0.040577f, -0.000000f, -0.000000f, 1},
                                        {0.954761f, -0.000983f, 0.000698f, 0.297372f}};
                outBoneTransform[19] = {{-0.028698f, 0.000000f, 0.000000f, 1},
                                        {0.976924f, -0.001344f, -0.010281f, 0.213335f}};
                outBoneTransform[20] = {{-0.022430f, 0.000000f, -0.000000f, 1},
                                        {1.000000f, 0.000000f, 0.000000f, 0.000000f}};
                outBoneTransform[21] = {{0.001792f, -0.019041f, 0.015254f, 1},
                                        {0.510569f, -0.514906f, 0.341115f, 0.598191f}};
                outBoneTransform[22] = {{-0.062878f, -0.002844f, -0.000332f, 1},
                                        {0.979195f, -0.043879f, -0.095103f, 0.173800f}};
                outBoneTransform[23] = {{-0.030154f, -0.000000f, -0.000000f, 1},
                                        {0.971387f, -0.000102f, -0.002019f, 0.237494f}};
                outBoneTransform[24] = {{-0.018187f, -0.000000f, -0.000000f, 1},
                                        {0.997961f, 0.000800f, -0.051911f, -0.037114f}};
                outBoneTransform[25] = {{-0.018018f, -0.000000f, 0.000000f, 1},
                                        {1.000000f, -0.000000f, -0.000000f, -0.000000f}};
                outBoneTransform[26] = {{0.004392f, 0.055515f, 0.060253f, 1},
                                        {0.745924f, 0.156756f, -0.597950f, -0.247953f}};
                outBoneTransform[27] = {{-0.000171f, 0.016473f, 0.096515f, 1},
                                        {-0.006456f, 0.022747f, 0.932927f, 0.359287f}};
                outBoneTransform[28] = {{0.038119f, -0.074730f, 0.046338f, 1},
                                        {-0.207931f, 0.699835f, 0.632631f, 0.258406f}};
                outBoneTransform[29] = {{0.035492f, -0.089519f, 0.081636f, 1},
                                        {-0.197555f, 0.760574f, 0.601098f, 0.145535f}};
                outBoneTransform[30] = {{0.029073f, -0.085957f, 0.119561f, 1},
                                        {-0.031423f, 0.791013f, 0.597190f, 0.129133f}};
            }
        }
    } else if (touch) {
        if (withController) {
            if (isLeftHand) {
                outBoneTransform[6] = {{-0.003925f, 0.027171f, 0.014640f, 1},
                                       {0.666448f, 0.430031f, -0.455947f, 0.403772f}};
                outBoneTransform[7] = {{0.074204f, -0.005002f, 0.000234f, 1},
                                       {-0.951843f, 0.009717f, 0.158611f, -0.262188f}};
                outBoneTransform[8] = {{0.043930f, -0.000000f, -0.000000f, 1},
                                       {-0.973045f, -0.044676f, 0.010341f, -0.226012f}};
                outBoneTransform[9] = {{0.028695f, 0.000000f, 0.000000f, 1},
                                       {-0.935253f, -0.002881f, 0.023037f, -0.353217f}};
                outBoneTransform[10] = {{0.022821f, 0.000000f, -0.000000f, 1},
                                        {1.000000f, -0.000000f, -0.000000f, 0.000000f}};
                outBoneTransform[11] = {{0.002177f, 0.007120f, 0.016319f, 1},
                                        {0.529359f, 0.540512f, -0.463783f, 0.461011f}};
                outBoneTransform[12] = {{0.070953f, 0.000779f, 0.000997f, 1},
                                        {0.847397f, -0.257141f, -0.139135f, 0.443213f}};
                outBoneTransform[13] = {{0.043108f, 0.000000f, 0.000000f, 1},
                                        {0.874907f, 0.009875f, 0.026584f, 0.483460f}};
                outBoneTransform[14] = {{0.033266f, -0.000000f, 0.000000f, 1},
                                        {0.894578f, -0.036774f, -0.050597f, 0.442513f}};
                outBoneTransform[15] = {{0.025892f, -0.000000f, 0.000000f, 1},
                                        {0.999195f, -0.000000f, 0.000000f, 0.040126f}};
                outBoneTransform[16] = {{0.000513f, -0.006545f, 0.016348f, 1},
                                        {0.500244f, 0.530784f, -0.516215f, 0.448939f}};
                outBoneTransform[17] = {{0.065876f, 0.001786f, 0.000693f, 1},
                                        {0.831617f, -0.242931f, -0.139695f, 0.479461f}};
                outBoneTransform[18] = {{0.040697f, 0.000000f, 0.000000f, 1},
                                        {0.769163f, -0.001746f, 0.001363f, 0.639049f}};
                outBoneTransform[19] = {{0.028747f, -0.000000f, -0.000000f, 1},
                                        {0.968615f, -0.064538f, -0.046586f, 0.235477f}};
                outBoneTransform[20] = {{0.022430f, -0.000000f, 0.000000f, 1},
                                        {1.000000f, 0.000000f, -0.000000f, -0.000000f}};
                outBoneTransform[21] = {{-0.002478f, -0.018981f, 0.015214f, 1},
                                        {0.474671f, 0.434670f, -0.653212f, 0.398827f}};
                outBoneTransform[22] = {{0.062878f, 0.002844f, 0.000332f, 1},
                                        {0.798788f, -0.199577f, -0.094418f, 0.559636f}};
                outBoneTransform[23] = {{0.030220f, 0.000002f, -0.000000f, 1},
                                        {0.853087f, 0.001644f, -0.000913f, 0.521765f}};
                outBoneTransform[24] = {{0.018187f, -0.000002f, 0.000000f, 1},
                                        {0.974249f, 0.052491f, 0.003591f, 0.219249f}};
                outBoneTransform[25] = {{0.018018f, 0.000000f, -0.000000f, 1},
                                        {1.000000f, 0.000000f, 0.000000f, 0.000000f}};
                outBoneTransform[26] = {{0.006629f, 0.026690f, 0.061870f, 1},
                                        {0.805084f, -0.018369f, 0.584788f, -0.097597f}};
                outBoneTransform[27] = {{-0.009005f, -0.041708f, 0.037992f, 1},
                                        {-0.338860f, 0.939952f, -0.007564f, 0.040082f}};
                outBoneTransform[28] = {{0.017136f, -0.032633f, 0.080682f, 1},
                                        {-0.169466f, 0.800083f, 0.571006f, 0.071415f}};
                outBoneTransform[29] = {{0.011144f, -0.028727f, 0.108366f, 1},
                                        {-0.076328f, 0.788280f, 0.605097f, 0.081527f}};
                outBoneTransform[30] = {{0.011333f, -0.026044f, 0.128585f, 1},
                                        {-0.144791f, 0.737451f, 0.656958f, -0.060069f}};
            } else {
                outBoneTransform[6] = {{-0.003925f, 0.027171f, 0.014640f, 1},
                                       {0.666448f, 0.430031f, -0.455947f, 0.403772f}};
                outBoneTransform[7] = {{0.074204f, -0.005002f, 0.000234f, 1},
                                       {-0.951843f, 0.009717f, 0.158611f, -0.262188f}};
                outBoneTransform[8] = {{0.043930f, -0.000000f, -0.000000f, 1},
                                       {-0.973045f, -0.044676f, 0.010341f, -0.226012f}};
                outBoneTransform[9] = {{0.028695f, 0.000000f, 0.000000f, 1},
                                       {-0.935253f, -0.002881f, 0.023037f, -0.353217f}};
                outBoneTransform[10] = {{0.022821f, 0.000000f, -0.000000f, 1},
                                        {1.000000f, -0.000000f, -0.000000f, 0.000000f}};
                outBoneTransform[11] = {{0.002177f, 0.007120f, 0.016319f, 1},
                                        {0.529359f, 0.540512f, -0.463783f, 0.461011f}};
                outBoneTransform[12] = {{0.070953f, 0.000779f, 0.000997f, 1},
                                        {0.847397f, -0.257141f, -0.139135f, 0.443213f}};
                outBoneTransform[13] = {{0.043108f, 0.000000f, 0.000000f, 1},
                                        {0.874907f, 0.009875f, 0.026584f, 0.483460f}};
                outBoneTransform[14] = {{0.033266f, -0.000000f, 0.000000f, 1},
                                        {0.894578f, -0.036774f, -0.050597f, 0.442513f}};
                outBoneTransform[15] = {{0.025892f, -0.000000f, 0.000000f, 1},
                                        {0.999195f, -0.000000f, 0.000000f, 0.040126f}};
                outBoneTransform[16] = {{0.000513f, -0.006545f, 0.016348f, 1},
                                        {0.500244f, 0.530784f, -0.516215f, 0.448939f}};
                outBoneTransform[17] = {{0.065876f, 0.001786f, 0.000693f, 1},
                                        {0.831617f, -0.242931f, -0.139695f, 0.479461f}};
                outBoneTransform[18] = {{0.040697f, 0.000000f, 0.000000f, 1},
                                        {0.769163f, -0.001746f, 0.001363f, 0.639049f}};
                outBoneTransform[19] = {{0.028747f, -0.000000f, -0.000000f, 1},
                                        {0.968615f, -0.064538f, -0.046586f, 0.235477f}};
                outBoneTransform[20] = {{0.022430f, -0.000000f, 0.000000f, 1},
                                        {1.000000f, 0.000000f, -0.000000f, -0.000000f}};
                outBoneTransform[21] = {{-0.002478f, -0.018981f, 0.015214f, 1},
                                        {0.474671f, 0.434670f, -0.653212f, 0.398827f}};
                outBoneTransform[22] = {{0.062878f, 0.002844f, 0.000332f, 1},
                                        {0.798788f, -0.199577f, -0.094418f, 0.559636f}};
                outBoneTransform[23] = {{0.030220f, 0.000002f, -0.000000f, 1},
                                        {0.853087f, 0.001644f, -0.000913f, 0.521765f}};
                outBoneTransform[24] = {{0.018187f, -0.000002f, 0.000000f, 1},
                                        {0.974249f, 0.052491f, 0.003591f, 0.219249f}};
                outBoneTransform[25] = {{0.018018f, 0.000000f, -0.000000f, 1},
                                        {1.000000f, 0.000000f, 0.000000f, 0.000000f}};
                outBoneTransform[26] = {{0.006629f, 0.026690f, 0.061870f, 1},
                                        {0.805084f, -0.018369f, 0.584788f, -0.097597f}};
                outBoneTransform[27] = {{-0.009005f, -0.041708f, 0.037992f, 1},
                                        {-0.338860f, 0.939952f, -0.007564f, 0.040082f}};
                outBoneTransform[28] = {{0.017136f, -0.032633f, 0.080682f, 1},
                                        {-0.169466f, 0.800083f, 0.571006f, 0.071415f}};
                outBoneTransform[29] = {{0.011144f, -0.028727f, 0.108366f, 1},
                                        {-0.076328f, 0.788280f, 0.605097f, 0.081527f}};
                outBoneTransform[30] = {{0.011333f, -0.026044f, 0.128585f, 1},
                                        {-0.144791f, 0.737451f, 0.656958f, -0.060069f}};
            }
        } else {
            if (isLeftHand) {
                outBoneTransform[6] = {{0.002693f, 0.023387f, 0.013573f, 1},
                                       {0.626743f, 0.404630f, -0.499840f, 0.440032f}};
                outBoneTransform[7] = {{0.074204f, -0.005002f, 0.000234f, 1},
                                       {0.869067f, -0.019031f, -0.093524f, 0.485400f}};
                outBoneTransform[8] = {{0.043512f, -0.000000f, -0.000000f, 1},
                                       {0.834068f, 0.020722f, 0.003930f, 0.551259f}};
                outBoneTransform[9] = {{0.028422f, 0.000000f, 0.000000f, 1},
                                       {0.890556f, 0.000289f, -0.009290f, 0.454779f}};
                outBoneTransform[10] = {{0.022821f, 0.000000f, -0.000000f, 1},
                                        {1.000000f, 0.000000f, -0.000000f, 0.000000f}};
                outBoneTransform[11] = {{0.003937f, 0.006967f, 0.016424f, 1},
                                        {0.531603f, 0.532690f, -0.459598f, 0.471602f}};
                outBoneTransform[12] = {{0.070953f, 0.000779f, 0.000997f, 1},
                                        {0.906933f, -0.142169f, -0.015445f, 0.396261f}};
                outBoneTransform[13] = {{0.043108f, 0.000000f, 0.000000f, 1},
                                        {0.975787f, 0.014996f, 0.010867f, 0.217936f}};
                outBoneTransform[14] = {{0.033266f, 0.000000f, 0.000000f, 1},
                                        {0.992777f, -0.002096f, -0.021403f, 0.118029f}};
                outBoneTransform[15] = {{0.025892f, -0.000000f, 0.000000f, 1},
                                        {0.999195f, 0.000000f, 0.000000f, 0.040126f}};
                outBoneTransform[16] = {{0.001282f, -0.006612f, 0.016394f, 1},
                                        {0.513688f, 0.543325f, -0.502550f, 0.434011f}};
                outBoneTransform[17] = {{0.065876f, 0.001786f, 0.000693f, 1},
                                        {0.971280f, -0.068108f, -0.073480f, 0.215818f}};
                outBoneTransform[18] = {{0.040619f, 0.000000f, 0.000000f, 1},
                                        {0.976566f, -0.001379f, 0.000441f, 0.215216f}};
                outBoneTransform[19] = {{0.028715f, -0.000000f, -0.000000f, 1},
                                        {0.987232f, -0.000977f, -0.011919f, 0.158838f}};
                outBoneTransform[20] = {{0.022430f, -0.000000f, 0.000000f, 1},
                                        {1.000000f, 0.000000f, 0.000000f, 0.000000f}};
                outBoneTransform[21] = {{-0.002032f, -0.019020f, 0.015240f, 1},
                                        {0.521784f, 0.511917f, -0.594340f, 0.335325f}};
                outBoneTransform[22] = {{0.062878f, 0.002844f, 0.000332f, 1},
                                        {0.982925f, -0.053050f, -0.108004f, 0.139206f}};
                outBoneTransform[23] = {{0.030177f, 0.000000f, 0.000000f, 1},
                                        {0.979798f, 0.000394f, -0.001374f, 0.199982f}};
                outBoneTransform[24] = {{0.018187f, 0.000000f, 0.000000f, 1},
                                        {0.997410f, -0.000172f, -0.051977f, -0.049724f}};
                outBoneTransform[25] = {{0.018018f, 0.000000f, -0.000000f, 1},
                                        {1.000000f, -0.000000f, -0.000000f, -0.000000f}};
                outBoneTransform[26] = {{-0.004857f, 0.053377f, 0.060017f, 1},
                                        {0.751040f, 0.174397f, 0.601473f, 0.209178f}};
                outBoneTransform[27] = {{-0.013234f, -0.004327f, 0.069740f, 1},
                                        {-0.119277f, 0.262590f, -0.888979f, -0.355718f}};
                outBoneTransform[28] = {{-0.037500f, -0.074514f, 0.046899f, 1},
                                        {-0.204942f, 0.706005f, -0.626220f, -0.259623f}};
                outBoneTransform[29] = {{-0.036251f, -0.089302f, 0.081732f, 1},
                                        {-0.194045f, 0.764033f, -0.596592f, -0.150590f}};
                outBoneTransform[30] = {{-0.029633f, -0.085595f, 0.119439f, 1},
                                        {-0.025015f, 0.787219f, -0.601140f, -0.135243f}};
            } else {
                outBoneTransform[6] = {{-0.002693f, 0.023387f, 0.013573f, 1},
                                       {0.404698f, -0.626951f, 0.439894f, 0.499645f}};
                outBoneTransform[7] = {{-0.074204f, 0.005002f, -0.000234f, 1},
                                       {0.870303f, -0.017421f, -0.092515f, 0.483436f}};
                outBoneTransform[8] = {{-0.043512f, 0.000000f, 0.000000f, 1},
                                       {0.835972f, 0.018944f, 0.003312f, 0.548436f}};
                outBoneTransform[9] = {{-0.028422f, -0.000000f, -0.000000f, 1},
                                       {0.890326f, 0.000173f, -0.008504f, 0.455244f}};
                outBoneTransform[10] = {{-0.022821f, -0.000000f, 0.000000f, 1},
                                        {1.000000f, 0.000000f, -0.000000f, 0.000000f}};
                outBoneTransform[11] = {{-0.003937f, 0.006967f, 0.016424f, 1},
                                        {0.532293f, -0.531137f, 0.472074f, 0.460113f}};
                outBoneTransform[12] = {{-0.070953f, -0.000779f, -0.000997f, 1},
                                        {0.908154f, -0.139967f, -0.013210f, 0.394323f}};
                outBoneTransform[13] = {{-0.043108f, -0.000000f, -0.000000f, 1},
                                        {0.977887f, 0.015350f, 0.008912f, 0.208378f}};
                outBoneTransform[14] = {{-0.033266f, -0.000000f, -0.000000f, 1},
                                        {0.992487f, -0.002006f, -0.020888f, 0.120540f}};
                outBoneTransform[15] = {{-0.025892f, 0.000000f, -0.000000f, 1},
                                        {0.999195f, 0.000000f, 0.000000f, 0.040126f}};
                outBoneTransform[16] = {{-0.001282f, -0.006612f, 0.016394f, 1},
                                        {0.544460f, -0.511334f, 0.436935f, 0.501187f}};
                outBoneTransform[17] = {{-0.065876f, -0.001786f, -0.000693f, 1},
                                        {0.971233f, -0.064561f, -0.071188f, 0.217877f}};
                outBoneTransform[18] = {{-0.040619f, -0.000000f, -0.000000f, 1},
                                        {0.978211f, -0.001419f, 0.000451f, 0.207607f}};
                outBoneTransform[19] = {{-0.028715f, 0.000000f, 0.000000f, 1},
                                        {0.987488f, -0.001166f, -0.010852f, 0.157314f}};
                outBoneTransform[20] = {{-0.022430f, 0.000000f, -0.000000f, 1},
                                        {1.000000f, 0.000000f, 0.000000f, 0.000000f}};
                outBoneTransform[21] = {{0.002032f, -0.019020f, 0.015240f, 1},
                                        {0.513640f, -0.518192f, 0.337332f, 0.594860f}};
                outBoneTransform[22] = {{-0.062878f, -0.002844f, -0.000332f, 1},
                                        {0.983501f, -0.050059f, -0.104491f, 0.138930f}};
                outBoneTransform[23] = {{-0.030177f, -0.000000f, -0.000000f, 1},
                                        {0.981170f, 0.000501f, -0.001363f, 0.193138f}};
                outBoneTransform[24] = {{-0.018187f, -0.000000f, -0.000000f, 1},
                                        {0.997801f, 0.000487f, -0.051933f, -0.041173f}};
                outBoneTransform[25] = {{-0.018018f, -0.000000f, 0.000000f, 1},
                                        {1.000000f, -0.000000f, -0.000000f, -0.000000f}};
                outBoneTransform[26] = {{0.004574f, 0.055518f, 0.060226f, 1},
                                        {0.745334f, 0.161961f, -0.597782f, -0.246784f}};
                outBoneTransform[27] = {{0.013831f, -0.004360f, 0.069547f, 1},
                                        {-0.117443f, 0.257604f, 0.890065f, 0.357255f}};
                outBoneTransform[28] = {{0.038220f, -0.074817f, 0.046428f, 1},
                                        {-0.205767f, 0.697939f, 0.635107f, 0.259191f}};
                outBoneTransform[29] = {{0.035802f, -0.089658f, 0.081733f, 1},
                                        {-0.196007f, 0.758396f, 0.604341f, 0.145564f}};
                outBoneTransform[30] = {{0.029364f, -0.086069f, 0.119701f, 1},
                                        {-0.028444f, 0.787767f, 0.601616f, 0.129123f}};
            }
        }
    } else {
        // no touch
        if (isLeftHand) {
            outBoneTransform[6] = {{0.000632f, 0.026866f, 0.015002f, 1},
                                   {0.644251f, 0.421979f, -0.478202f, 0.422133f}};
            outBoneTransform[7] = {{0.074204f, -0.005002f, 0.000234f, 1},
                                   {0.995332f, 0.007007f, -0.039124f, 0.087949f}};
            outBoneTransform[8] = {{0.043930f, -0.000000f, -0.000000f, 1},
                                   {0.997891f, 0.045808f, 0.002142f, -0.045943f}};
            outBoneTransform[9] = {{0.028695f, 0.000000f, 0.000000f, 1},
                                   {0.999649f, 0.001850f, -0.022782f, -0.013409f}};
            outBoneTransform[10] = {{0.022821f, 0.000000f, -0.000000f, 1},
                                    {1.000000f, 0.000000f, -0.000000f, 0.000000f}};
            outBoneTransform[11] = {{0.002177f, 0.007120f, 0.016319f, 1},
                                    {0.546723f, 0.541277f, -0.442520f, 0.460749f}};
            outBoneTransform[12] = {{0.070953f, 0.000779f, 0.000997f, 1},
                                    {0.980294f, -0.167261f, -0.078959f, 0.069368f}};
            outBoneTransform[13] = {{0.043108f, 0.000000f, 0.000000f, 1},
                                    {0.997947f, 0.018493f, 0.013192f, 0.059886f}};
            outBoneTransform[14] = {{0.033266f, 0.000000f, 0.000000f, 1},
                                    {0.997394f, -0.003328f, -0.028225f, -0.066315f}};
            outBoneTransform[15] = {{0.025892f, -0.000000f, 0.000000f, 1},
                                    {0.999195f, 0.000000f, 0.000000f, 0.040126f}};
            outBoneTransform[16] = {{0.000513f, -0.006545f, 0.016348f, 1},
                                    {0.516692f, 0.550144f, -0.495548f, 0.429888f}};
            outBoneTransform[17] = {{0.065876f, 0.001786f, 0.000693f, 1},
                                    {0.990420f, -0.058696f, -0.101820f, 0.072495f}};
            outBoneTransform[18] = {{0.040697f, 0.000000f, 0.000000f, 1},
                                    {0.999545f, -0.002240f, 0.000004f, 0.030081f}};
            outBoneTransform[19] = {{0.028747f, -0.000000f, -0.000000f, 1},
                                    {0.999102f, -0.000721f, -0.012693f, 0.040420f}};
            outBoneTransform[20] = {{0.022430f, -0.000000f, 0.000000f, 1},
                                    {1.000000f, 0.000000f, 0.000000f, 0.000000f}};
            outBoneTransform[21] = {{-0.002478f, -0.018981f, 0.015214f, 1},
                                    {0.526918f, 0.523940f, -0.584025f, 0.326740f}};
            outBoneTransform[22] = {{0.062878f, 0.002844f, 0.000332f, 1},
                                    {0.986609f, -0.059615f, -0.135163f, 0.069132f}};
            outBoneTransform[23] = {{0.030220f, 0.000000f, 0.000000f, 1},
                                    {0.994317f, 0.001896f, -0.000132f, 0.106446f}};
            outBoneTransform[24] = {{0.018187f, 0.000000f, 0.000000f, 1},
                                    {0.995931f, -0.002010f, -0.052079f, -0.073526f}};
            outBoneTransform[25] = {{0.018018f, 0.000000f, -0.000000f, 1},
                                    {1.000000f, -0.000000f, -0.000000f, -0.000000f}};
            outBoneTransform[26] = {{-0.006059f, 0.056285f, 0.060064f, 1},
                                    {0.737238f, 0.202745f, 0.594267f, 0.249441f}};
            outBoneTransform[27] = {{-0.040416f, -0.043018f, 0.019345f, 1},
                                    {-0.290330f, 0.623527f, -0.663809f, -0.293734f}};
            outBoneTransform[28] = {{-0.039354f, -0.075674f, 0.047048f, 1},
                                    {-0.187047f, 0.678062f, -0.659285f, -0.265683f}};
            outBoneTransform[29] = {{-0.038340f, -0.090987f, 0.082579f, 1},
                                    {-0.183037f, 0.736793f, -0.634757f, -0.143936f}};
            outBoneTransform[30] = {{-0.031806f, -0.087214f, 0.121015f, 1},
                                    {-0.003659f, 0.758407f, -0.639342f, -0.126678f}};
        } else {
            outBoneTransform[6] = {{-0.000632f, 0.026866f, 0.015002f, 1},
                                   {0.421833f, -0.643793f, 0.422458f, 0.478661f}};
            outBoneTransform[7] = {{-0.074204f, 0.005002f, -0.000234f, 1},
                                   {0.994784f, 0.007053f, -0.041286f, 0.093009f}};
            outBoneTransform[8] = {{-0.043930f, 0.000000f, 0.000000f, 1},
                                   {0.998404f, 0.045905f, 0.002780f, -0.032767f}};
            outBoneTransform[9] = {{-0.028695f, -0.000000f, -0.000000f, 1},
                                   {0.999704f, 0.001955f, -0.022774f, -0.008282f}};
            outBoneTransform[10] = {{-0.022821f, -0.000000f, 0.000000f, 1},
                                    {1.000000f, 0.000000f, -0.000000f, 0.000000f}};
            outBoneTransform[11] = {{-0.002177f, 0.007120f, 0.016319f, 1},
                                    {0.541874f, -0.547427f, 0.459996f, 0.441701f}};
            outBoneTransform[12] = {{-0.070953f, -0.000779f, -0.000997f, 1},
                                    {0.979837f, -0.168061f, -0.075910f, 0.076899f}};
            outBoneTransform[13] = {{-0.043108f, -0.000000f, -0.000000f, 1},
                                    {0.997271f, 0.018278f, 0.013375f, 0.070266f}};
            outBoneTransform[14] = {{-0.033266f, -0.000000f, -0.000000f, 1},
                                    {0.998402f, -0.003143f, -0.026423f, -0.049849f}};
            outBoneTransform[15] = {{-0.025892f, 0.000000f, -0.000000f, 1},
                                    {0.999195f, 0.000000f, 0.000000f, 0.040126f}};
            outBoneTransform[16] = {{-0.000513f, -0.006545f, 0.016348f, 1},
                                    {0.548983f, -0.519068f, 0.426914f, 0.496920f}};
            outBoneTransform[17] = {{-0.065876f, -0.001786f, -0.000693f, 1},
                                    {0.989791f, -0.065882f, -0.096417f, 0.081716f}};
            outBoneTransform[18] = {{-0.040697f, -0.000000f, -0.000000f, 1},
                                    {0.999102f, -0.002168f, -0.000020f, 0.042317f}};
            outBoneTransform[19] = {{-0.028747f, 0.000000f, 0.000000f, 1},
                                    {0.998584f, -0.000674f, -0.012714f, 0.051653f}};
            outBoneTransform[20] = {{-0.022430f, 0.000000f, -0.000000f, 1},
                                    {1.000000f, 0.000000f, 0.000000f, 0.000000f}};
            outBoneTransform[21] = {{0.002478f, -0.018981f, 0.015214f, 1},
                                    {0.518597f, -0.527304f, 0.328264f, 0.587580f}};
            outBoneTransform[22] = {{-0.062878f, -0.002844f, -0.000332f, 1},
                                    {0.987294f, -0.063356f, -0.125964f, 0.073274f}};
            outBoneTransform[23] = {{-0.030220f, -0.000000f, -0.000000f, 1},
                                    {0.993413f, 0.001573f, -0.000147f, 0.114578f}};
            outBoneTransform[24] = {{-0.018187f, -0.000000f, -0.000000f, 1},
                                    {0.997047f, -0.000695f, -0.052009f, -0.056495f}};
            outBoneTransform[25] = {{-0.018018f, -0.000000f, 0.000000f, 1},
                                    {1.000000f, -0.000000f, -0.000000f, -0.000000f}};
            outBoneTransform[26] = {{0.005198f, 0.054204f, 0.060030f, 1},
                                    {0.747318f, 0.182508f, -0.599586f, -0.220688f}};
            outBoneTransform[27] = {{0.038779f, -0.042973f, 0.019824f, 1},
                                    {-0.297445f, 0.639373f, 0.648910f, 0.285734f}};
            outBoneTransform[28] = {{0.038027f, -0.074844f, 0.046941f, 1},
                                    {-0.199898f, 0.698218f, 0.635767f, 0.261406f}};
            outBoneTransform[29] = {{0.036845f, -0.089781f, 0.081973f, 1},
                                    {-0.190960f, 0.756469f, 0.607591f, 0.148733f}};
            outBoneTransform[30] = {{0.030251f, -0.086056f, 0.119887f, 1},
                                    {-0.018948f, 0.779249f, 0.612180f, 0.132846f}};
        }
    }
}

void GetGripClickBoneTransform(bool withController,
                               bool isLeftHand,
                               vr::VRBoneTransform_t outBoneTransform[]) {
    if (withController) {
        if (isLeftHand) {
            outBoneTransform[11] = {{0.002177f, 0.007120f, 0.016319f, 1},
                                    {0.529359f, 0.540512f, -0.463783f, 0.461011f}};
            outBoneTransform[12] = {{0.070953f, 0.000779f, 0.000997f, 1},
                                    {-0.831727f, 0.270927f, 0.175647f, -0.451638f}};
            outBoneTransform[13] = {{0.043108f, 0.000000f, 0.000000f, 1},
                                    {-0.854886f, -0.008231f, -0.028107f, -0.517990f}};
            outBoneTransform[14] = {{0.033266f, -0.000000f, 0.000000f, 1},
                                    {-0.825759f, 0.085208f, 0.086456f, -0.550805f}};
            outBoneTransform[15] = {{0.025892f, -0.000000f, 0.000000f, 1},
                                    {0.999195f, -0.000000f, 0.000000f, 0.040126f}};
            outBoneTransform[16] = {{0.000513f, -0.006545f, 0.016348f, 1},
                                    {0.500244f, 0.530784f, -0.516215f, 0.448939f}};
            outBoneTransform[17] = {{0.065876f, 0.001786f, 0.000693f, 1},
                                    {0.831617f, -0.242931f, -0.139695f, 0.479461f}};
            outBoneTransform[18] = {{0.040697f, 0.000000f, 0.000000f, 1},
                                    {0.769163f, -0.001746f, 0.001363f, 0.639049f}};
            outBoneTransform[19] = {{0.028747f, -0.000000f, -0.000000f, 1},
                                    {0.968615f, -0.064537f, -0.046586f, 0.235477f}};
            outBoneTransform[20] = {{0.022430f, -0.000000f, 0.000000f, 1},
                                    {1.000000f, 0.000000f, -0.000000f, -0.000000f}};
            outBoneTransform[21] = {{-0.002478f, -0.018981f, 0.015214f, 1},
                                    {0.474671f, 0.434670f, -0.653212f, 0.398827f}};
            outBoneTransform[22] = {{0.062878f, 0.002844f, 0.000332f, 1},
                                    {0.798788f, -0.199577f, -0.094418f, 0.559636f}};
            outBoneTransform[23] = {{0.030220f, 0.000002f, -0.000000f, 1},
                                    {0.853087f, 0.001644f, -0.000913f, 0.521765f}};
            outBoneTransform[24] = {{0.018187f, -0.000002f, 0.000000f, 1},
                                    {0.974249f, 0.052491f, 0.003591f, 0.219249f}};
            outBoneTransform[25] = {{0.018018f, 0.000000f, -0.000000f, 1},
                                    {1.000000f, 0.000000f, 0.000000f, 0.000000f}};

            outBoneTransform[28] = {{0.016642f, -0.029992f, 0.083200f, 1},
                                    {-0.094577f, 0.694550f, 0.702845f, 0.121100f}};
            outBoneTransform[29] = {{0.011144f, -0.028727f, 0.108366f, 1},
                                    {-0.076328f, 0.788280f, 0.605097f, 0.081527f}};
            outBoneTransform[30] = {{0.011333f, -0.026044f, 0.128585f, 1},
                                    {-0.144791f, 0.737451f, 0.656958f, -0.060069f}};
        } else {
            outBoneTransform[11] = {{0.002177f, 0.007120f, 0.016319f, 1},
                                    {0.529359f, 0.540512f, -0.463783f, 0.461011f}};
            outBoneTransform[12] = {{0.070953f, 0.000779f, 0.000997f, 1},
                                    {-0.831727f, 0.270927f, 0.175647f, -0.451638f}};
            outBoneTransform[13] = {{0.043108f, 0.000000f, 0.000000f, 1},
                                    {-0.854886f, -0.008231f, -0.028107f, -0.517990f}};
            outBoneTransform[14] = {{0.033266f, -0.000000f, 0.000000f, 1},
                                    {-0.825759f, 0.085208f, 0.086456f, -0.550805f}};
            outBoneTransform[15] = {{0.025892f, -0.000000f, 0.000000f, 1},
                                    {0.999195f, -0.000000f, 0.000000f, 0.040126f}};
            outBoneTransform[16] = {{0.000513f, -0.006545f, 0.016348f, 1},
                                    {0.500244f, 0.530784f, -0.516215f, 0.448939f}};
            outBoneTransform[17] = {{0.065876f, 0.001786f, 0.000693f, 1},
                                    {0.831617f, -0.242931f, -0.139695f, 0.479461f}};
            outBoneTransform[18] = {{0.040697f, 0.000000f, 0.000000f, 1},
                                    {0.769163f, -0.001746f, 0.001363f, 0.639049f}};
            outBoneTransform[19] = {{0.028747f, -0.000000f, -0.000000f, 1},
                                    {0.968615f, -0.064537f, -0.046586f, 0.235477f}};
            outBoneTransform[20] = {{0.022430f, -0.000000f, 0.000000f, 1},
                                    {1.000000f, 0.000000f, -0.000000f, -0.000000f}};
            outBoneTransform[21] = {{-0.002478f, -0.018981f, 0.015214f, 1},
                                    {0.474671f, 0.434670f, -0.653212f, 0.398827f}};
            outBoneTransform[22] = {{0.062878f, 0.002844f, 0.000332f, 1},
                                    {0.798788f, -0.199577f, -0.094418f, 0.559636f}};
            outBoneTransform[23] = {{0.030220f, 0.000002f, -0.000000f, 1},
                                    {0.853087f, 0.001644f, -0.000913f, 0.521765f}};
            outBoneTransform[24] = {{0.018187f, -0.000002f, 0.000000f, 1},
                                    {0.974249f, 0.052491f, 0.003591f, 0.219249f}};
            outBoneTransform[25] = {{0.018018f, 0.000000f, -0.000000f, 1},
                                    {1.000000f, 0.000000f, 0.000000f, 0.000000f}};

            outBoneTransform[28] = {{0.016642f, -0.029992f, 0.083200f, 1},
                                    {-0.094577f, 0.694550f, 0.702845f, 0.121100f}};
            outBoneTransform[29] = {{0.011144f, -0.028727f, 0.108366f, 1},
                                    {-0.076328f, 0.788280f, 0.605097f, 0.081527f}};
            outBoneTransform[30] = {{0.011333f, -0.026044f, 0.128585f, 1},
                                    {-0.144791f, 0.737451f, 0.656958f, -0.060069f}};
        }

    } else {
        if (isLeftHand) {
            outBoneTransform[11] = {{0.005787f, 0.006806f, 0.016534f, 1},
                                    {0.514203f, 0.522315f, -0.478348f, 0.483700f}};
            outBoneTransform[12] = {{0.070953f, 0.000779f, 0.000997f, 1},
                                    {0.723653f, -0.097901f, 0.048546f, 0.681458f}};
            outBoneTransform[13] = {{0.043108f, 0.000000f, 0.000000f, 1},
                                    {0.637464f, -0.002366f, -0.002831f, 0.770472f}};
            outBoneTransform[14] = {{0.033266f, 0.000000f, 0.000000f, 1},
                                    {0.658008f, 0.002610f, 0.003196f, 0.753000f}};
            outBoneTransform[15] = {{0.025892f, -0.000000f, 0.000000f, 1},
                                    {0.999195f, 0.000000f, 0.000000f, 0.040126f}};
            outBoneTransform[16] = {{0.004123f, -0.006858f, 0.016563f, 1},
                                    {0.489609f, 0.523374f, -0.520644f, 0.463997f}};
            outBoneTransform[17] = {{0.065876f, 0.001786f, 0.000693f, 1},
                                    {0.759970f, -0.055609f, 0.011571f, 0.647471f}};
            outBoneTransform[18] = {{0.040331f, 0.000000f, 0.000000f, 1},
                                    {0.664315f, 0.001595f, 0.001967f, 0.747449f}};
            outBoneTransform[19] = {{0.028489f, -0.000000f, -0.000000f, 1},
                                    {0.626957f, -0.002784f, -0.003234f, 0.779042f}};
            outBoneTransform[20] = {{0.022430f, -0.000000f, 0.000000f, 1},
                                    {1.000000f, 0.000000f, 0.000000f, 0.000000f}};
            outBoneTransform[21] = {{0.001131f, -0.019295f, 0.015429f, 1},
                                    {0.479766f, 0.477833f, -0.630198f, 0.379934f}};
            outBoneTransform[22] = {{0.062878f, 0.002844f, 0.000332f, 1},
                                    {0.827001f, 0.034282f, 0.003440f, 0.561144f}};
            outBoneTransform[23] = {{0.029874f, 0.000000f, 0.000000f, 1},
                                    {0.702185f, -0.006716f, -0.009289f, 0.711903f}};
            outBoneTransform[24] = {{0.017979f, 0.000000f, 0.000000f, 1},
                                    {0.676853f, 0.007956f, 0.009917f, 0.736009f}};
            outBoneTransform[25] = {{0.018018f, 0.000000f, -0.000000f, 1},
                                    {1.000000f, -0.000000f, -0.000000f, -0.000000f}};

            outBoneTransform[28] = {{0.000448f, 0.001536f, 0.116543f, 1},
                                    {-0.039357f, 0.105143f, -0.928833f, -0.353079f}};
            outBoneTransform[29] = {{0.003949f, -0.014869f, 0.130608f, 1},
                                    {-0.055071f, 0.068695f, -0.944016f, -0.317933f}};
            outBoneTransform[30] = {{0.003263f, -0.034685f, 0.139926f, 1},
                                    {0.019690f, -0.100741f, -0.957331f, -0.270149f}};
        } else {
            outBoneTransform[11] = {{-0.005787f, 0.006806f, 0.016534f, 1},
                                    {0.522315f, -0.514203f, 0.483700f, 0.478348f}};
            outBoneTransform[12] = {{-0.070953f, -0.000779f, -0.000997f, 1},
                                    {0.723653f, -0.097901f, 0.048546f, 0.681458f}};
            outBoneTransform[13] = {{-0.043108f, -0.000000f, -0.000000f, 1},
                                    {0.637464f, -0.002366f, -0.002831f, 0.770472f}};
            outBoneTransform[14] = {{-0.033266f, -0.000000f, -0.000000f, 1},
                                    {0.658008f, 0.002610f, 0.003196f, 0.753000f}};
            outBoneTransform[15] = {{-0.025892f, 0.000000f, -0.000000f, 1},
                                    {0.999195f, 0.000000f, 0.000000f, 0.040126f}};
            outBoneTransform[16] = {{-0.004123f, -0.006858f, 0.016563f, 1},
                                    {0.523374f, -0.489609f, 0.463997f, 0.520644f}};
            outBoneTransform[17] = {{-0.065876f, -0.001786f, -0.000693f, 1},
                                    {0.759970f, -0.055609f, 0.011571f, 0.647471f}};
            outBoneTransform[18] = {{-0.040331f, -0.000000f, -0.000000f, 1},
                                    {0.664315f, 0.001595f, 0.001967f, 0.747449f}};
            outBoneTransform[19] = {{-0.028489f, 0.000000f, 0.000000f, 1},
                                    {0.626957f, -0.002784f, -0.003234f, 0.779042f}};
            outBoneTransform[20] = {{-0.022430f, 0.000000f, -0.000000f, 1},
                                    {1.000000f, 0.000000f, 0.000000f, 0.000000f}};
            outBoneTransform[21] = {{-0.001131f, -0.019295f, 0.015429f, 1},
                                    {0.477833f, -0.479766f, 0.379935f, 0.630198f}};
            outBoneTransform[22] = {{-0.062878f, -0.002844f, -0.000332f, 1},
                                    {0.827001f, 0.034282f, 0.003440f, 0.561144f}};
            outBoneTransform[23] = {{-0.029874f, -0.000000f, -0.000000f, 1},
                                    {0.702185f, -0.006716f, -0.009289f, 0.711903f}};
            outBoneTransform[24] = {{-0.017979f, -0.000000f, -0.000000f, 1},
                                    {0.676853f, 0.007956f, 0.009917f, 0.736009f}};
            outBoneTransform[25] = {{-0.018018f, -0.000000f, 0.000000f, 1},
                                    {1.000000f, -0.000000f, -0.000000f, -0.000000f}};

            outBoneTransform[28] = {{-0.000448f, 0.001536f, 0.116543f, 1},
                                    {-0.039357f, 0.105143f, 0.928833f, 0.353079f}};
            outBoneTransform[29] = {{-0.003949f, -0.014869f, 0.130608f, 1},
                                    {-0.055071f, 0.068695f, 0.944016f, 0.317933f}};
            outBoneTransform[30] = {{-0.003263f, -0.034685f, 0.139926f, 1},
                                    {0.019690f, -0.100741f, 0.957331f, 0.270149f}};
        }
    }
}

void Controller::GetBoneTransform(bool withController, vr::VRBoneTransform_t outBoneTransform[]) {
    auto isLeftHand = device_id == HAND_LEFT_ID;

    vr::VRBoneTransform_t boneTransform1[SKELETON_BONE_COUNT];
    vr::VRBoneTransform_t boneTransform2[SKELETON_BONE_COUNT];

    // root and wrist
    outBoneTransform[0] = {{0.000000f, 0.000000f, 0.000000f, 1},
                           {1.000000f, -0.000000f, -0.000000f, 0.000000f}};
    if (isLeftHand) {
        outBoneTransform[1] = {{-0.034038f, 0.036503f, 0.164722f, 1},
                               {-0.055147f, -0.078608f, -0.920279f, 0.379296f}};
    } else {
        outBoneTransform[1] = {{0.034038f, 0.036503f, 0.164722f, 1},
                               {-0.055147f, -0.078608f, 0.920279f, -0.379296f}};
    }

    // thumb
    GetThumbBoneTransform(withController, isLeftHand, m_lastThumbTouch, boneTransform1);
    GetThumbBoneTransform(withController, isLeftHand, m_currentThumbTouch, boneTransform2);
    for (int boneIdx = 2; boneIdx < 6; boneIdx++) {
        outBoneTransform[boneIdx].position = Lerp(boneTransform1[boneIdx].position,
                                                  boneTransform2[boneIdx].position,
                                                  m_thumbTouchAnimationProgress);
        outBoneTransform[boneIdx].orientation = Slerp(boneTransform1[boneIdx].orientation,
                                                      boneTransform2[boneIdx].orientation,
                                                      m_thumbTouchAnimationProgress);
    }

    // trigger (index to pinky)
    if (m_triggerValue > 0) {
        GetTriggerBoneTransform(withController, isLeftHand, true, false, boneTransform1);
        GetTriggerBoneTransform(withController, isLeftHand, true, true, boneTransform2);
        for (int boneIdx = 6; boneIdx < SKELETON_BONE_COUNT; boneIdx++) {
            outBoneTransform[boneIdx].position = Lerp(
                boneTransform1[boneIdx].position, boneTransform2[boneIdx].position, m_triggerValue);
            outBoneTransform[boneIdx].orientation = Slerp(boneTransform1[boneIdx].orientation,
                                                          boneTransform2[boneIdx].orientation,
                                                          m_triggerValue);
        }
    } else {
        GetTriggerBoneTransform(
            withController, isLeftHand, m_lastTriggerTouch, false, boneTransform1);
        GetTriggerBoneTransform(
            withController, isLeftHand, m_currentTriggerTouch, false, boneTransform2);
        for (int boneIdx = 6; boneIdx < SKELETON_BONE_COUNT; boneIdx++) {
            outBoneTransform[boneIdx].position = Lerp(boneTransform1[boneIdx].position,
                                                      boneTransform2[boneIdx].position,
                                                      m_indexTouchAnimationProgress);
            outBoneTransform[boneIdx].orientation = Slerp(boneTransform1[boneIdx].orientation,
                                                          boneTransform2[boneIdx].orientation,
                                                          m_indexTouchAnimationProgress);
        }
    }

    // grip (middle to pinky)
    if (m_gripValue > 0) {
        GetGripClickBoneTransform(withController, isLeftHand, boneTransform2);
        for (int boneIdx = 11; boneIdx < 26; boneIdx++) {
            outBoneTransform[boneIdx].position = Lerp(
                outBoneTransform[boneIdx].position, boneTransform2[boneIdx].position, m_gripValue);
            outBoneTransform[boneIdx].orientation = Slerp(outBoneTransform[boneIdx].orientation,
                                                          boneTransform2[boneIdx].orientation,
                                                          m_gripValue);
        }
        for (int boneIdx = 28; boneIdx < SKELETON_BONE_COUNT; boneIdx++) {
            outBoneTransform[boneIdx].position = Lerp(
                outBoneTransform[boneIdx].position, boneTransform2[boneIdx].position, m_gripValue);
            outBoneTransform[boneIdx].orientation = Slerp(outBoneTransform[boneIdx].orientation,
                                                          boneTransform2[boneIdx].orientation,
                                                          m_gripValue);
        }
    }
}
