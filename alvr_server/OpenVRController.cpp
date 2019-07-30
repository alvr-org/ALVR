#include "OpenVRController.h"

OpenVRController::OpenVRController(bool hand, int index)
	: mHand(hand)
	, mPreviousButtons(0)
	, mPreviousFlags(0)
	, mObjectId(vr::k_unTrackedDeviceIndexInvalid)
	, mIndex(index)
{
	memset(&mPose, 0, sizeof(mPose));
	mPose.poseIsValid = true;
	mPose.result = vr::TrackingResult_Running_OK;
	mPose.deviceIsConnected = true;

	mPose.qWorldFromDriverRotation = HmdQuaternion_Init(1, 0, 0, 0);
	mPose.qDriverFromHeadRotation = HmdQuaternion_Init(1, 0, 0, 0);
	mPose.qRotation = HmdQuaternion_Init(1, 0, 0, 0);

	for (int i = 0; i < ALVR_INPUT_COUNT; i++) {
		mHandles[i] = vr::k_ulInvalidInputComponentHandle;
	}
	mIsTouch = Settings::Instance().mControllerType == "oculus_touch";
}

OpenVRController::~OpenVRController() {
}

bool OpenVRController::GetHand() {
	return mHand;
}

vr::EVRInitError OpenVRController::Activate(vr::TrackedDeviceIndex_t unObjectId)
{
	Log(L"RemoteController::Activate. objectId=%d", unObjectId);

	mObjectId = unObjectId;
	mPropertyContainer = vr::VRProperties()->TrackedDeviceToPropertyContainer(mObjectId);

	vr::VRProperties()->SetStringProperty(mPropertyContainer, vr::Prop_TrackingSystemName_String, Settings::Instance().mControllerTrackingSystemName.c_str());
	vr::VRProperties()->SetStringProperty(mPropertyContainer, vr::Prop_ManufacturerName_String, Settings::Instance().mControllerManufacturerName.c_str());
	vr::VRProperties()->SetStringProperty(mPropertyContainer, vr::Prop_ModelNumber_String, mHand ? (Settings::Instance().mControllerModelNumber + " (Left Controller)").c_str() : (Settings::Instance().mControllerModelNumber + " (Right Controller)").c_str());

	vr::VRProperties()->SetStringProperty(mPropertyContainer, vr::Prop_RenderModelName_String, mHand ? Settings::Instance().mControllerRenderModelNameLeft.c_str() : Settings::Instance().mControllerRenderModelNameRight.c_str());

	vr::VRProperties()->SetStringProperty(mPropertyContainer, vr::Prop_SerialNumber_String, GetSerialNumber().c_str());
	vr::VRProperties()->SetStringProperty(mPropertyContainer, vr::Prop_AttachedDeviceId_String, GetSerialNumber().c_str());
	vr::VRProperties()->SetStringProperty(mPropertyContainer, vr::Prop_RegisteredDeviceType_String, Settings::Instance().mControllerRegisteredDeviceType.c_str());

	uint64_t supportedButtons = 0xFFFFFFFFFFFFFFFFULL;
	vr::VRProperties()->SetUint64Property(mPropertyContainer, vr::Prop_SupportedButtons_Uint64, supportedButtons);

	vr::VRProperties()->SetBoolProperty(mPropertyContainer, vr::Prop_DeviceProvidesBatteryStatus_Bool, true);

	if (mIsTouch) {
		vr::VRProperties()->SetInt32Property(mPropertyContainer, vr::Prop_Axis0Type_Int32, vr::k_eControllerAxis_Joystick);
	}
	else {
		vr::VRProperties()->SetInt32Property(mPropertyContainer, vr::Prop_Axis0Type_Int32, vr::k_eControllerAxis_TrackPad);
	}
	vr::VRProperties()->SetInt32Property(mPropertyContainer, vr::Prop_ControllerRoleHint_Int32, mHand ? vr::TrackedControllerRole_LeftHand : vr::TrackedControllerRole_RightHand);

	vr::VRProperties()->SetStringProperty(mPropertyContainer, vr::Prop_ControllerType_String, Settings::Instance().mControllerType.c_str());
	vr::VRProperties()->SetStringProperty(mPropertyContainer, vr::Prop_LegacyInputProfile_String, Settings::Instance().mControllerLegacyInputProfile.c_str());
	vr::VRProperties()->SetStringProperty(mPropertyContainer, vr::Prop_InputProfilePath_String, Settings::Instance().mControllerInputProfilePath.c_str());
	int i = 0;

	if (mIsTouch) {
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/system/click", &mHandles[ALVR_INPUT_SYSTEM_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/application_menu/click", &mHandles[ALVR_INPUT_APPLICATION_MENU_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/grip/click", &mHandles[ALVR_INPUT_GRIP_CLICK]);
		vr::VRDriverInput()->CreateScalarComponent(mPropertyContainer, "/input/grip/value", &mHandles[ALVR_INPUT_GRIP_VALUE], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedOneSided);
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/grip/touch", &mHandles[ALVR_INPUT_GRIP_TOUCH]);

		if (!mHand) {
			// A,B for right hand.
			vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/a/click", &mHandles[ALVR_INPUT_A_CLICK]);
			vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/a/touch", &mHandles[ALVR_INPUT_A_TOUCH]);
			vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/b/click", &mHandles[ALVR_INPUT_B_CLICK]);
			vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/b/touch", &mHandles[ALVR_INPUT_B_TOUCH]);

			vr::VRDriverInput()->CreateSkeletonComponent(mPropertyContainer, "/input/skeleton/right", "/skeleton/hand/right", "/pose/raw", nullptr, SKELTON_BONE_COUNT, &mSkeletonHandle);
		}
		else {
			// X,Y for left hand.
			vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/x/click", &mHandles[ALVR_INPUT_X_CLICK]);
			vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/x/touch", &mHandles[ALVR_INPUT_X_TOUCH]);
			vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/y/click", &mHandles[ALVR_INPUT_Y_CLICK]);
			vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/y/touch", &mHandles[ALVR_INPUT_Y_TOUCH]);

			vr::VRDriverInput()->CreateSkeletonComponent(mPropertyContainer, "/input/skeleton/left", "/skeleton/hand/left", "/pose/raw", nullptr, SKELTON_BONE_COUNT, &mSkeletonHandle);
		}

		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/joystick/click", &mHandles[ALVR_INPUT_JOYSTICK_CLICK]);
		vr::VRDriverInput()->CreateScalarComponent(mPropertyContainer, "/input/joystick/x", &mHandles[ALVR_INPUT_JOYSTICK_X], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);
		vr::VRDriverInput()->CreateScalarComponent(mPropertyContainer, "/input/joystick/y", &mHandles[ALVR_INPUT_JOYSTICK_Y], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/joystick/touch", &mHandles[ALVR_INPUT_JOYSTICK_TOUCH]);

		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/back/click", &mHandles[ALVR_INPUT_BACK_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/guide/click", &mHandles[ALVR_INPUT_GUIDE_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/start/click", &mHandles[ALVR_INPUT_START_CLICK]);

		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/trigger/click", &mHandles[ALVR_INPUT_TRIGGER_CLICK]);
		vr::VRDriverInput()->CreateScalarComponent(mPropertyContainer, "/input/trigger/value", &mHandles[ALVR_INPUT_TRIGGER_VALUE], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedOneSided);
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/trigger/touch", &mHandles[ALVR_INPUT_TRIGGER_TOUCH]);
	}
	else {
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/system/click", &mHandles[ALVR_INPUT_SYSTEM_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/application_menu/click", &mHandles[ALVR_INPUT_APPLICATION_MENU_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/grip/click", &mHandles[ALVR_INPUT_GRIP_CLICK]);
		vr::VRDriverInput()->CreateScalarComponent(mPropertyContainer, "/input/grip/value", &mHandles[ALVR_INPUT_GRIP_VALUE], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedOneSided);
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/grip/touch", &mHandles[ALVR_INPUT_GRIP_TOUCH]);

		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/dpad_left/click", &mHandles[ALVR_INPUT_DPAD_LEFT_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/dpad_up/click", &mHandles[ALVR_INPUT_DPAD_UP_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/dpad_right/click", &mHandles[ALVR_INPUT_DPAD_RIGHT_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/dpad_down/click", &mHandles[ALVR_INPUT_DPAD_DOWN_CLICK]);

		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/a/click", &mHandles[ALVR_INPUT_A_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/a/touch", &mHandles[ALVR_INPUT_A_TOUCH]);
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/b/click", &mHandles[ALVR_INPUT_B_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/b/touch", &mHandles[ALVR_INPUT_B_TOUCH]);
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/x/click", &mHandles[ALVR_INPUT_X_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/x/touch", &mHandles[ALVR_INPUT_X_TOUCH]);
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/y/click", &mHandles[ALVR_INPUT_Y_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/y/touch", &mHandles[ALVR_INPUT_Y_TOUCH]);

		vr::VRDriverInput()->CreateScalarComponent(mPropertyContainer, "/input/trigger_left/value", &mHandles[ALVR_INPUT_TRIGGER_LEFT_VALUE], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedOneSided);
		vr::VRDriverInput()->CreateScalarComponent(mPropertyContainer, "/input/trigger_right/value", &mHandles[ALVR_INPUT_TRIGGER_RIGHT_VALUE], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedOneSided);

		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/shoulder_left/click", &mHandles[ALVR_INPUT_SHOULDER_LEFT_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/shoulder_right/click", &mHandles[ALVR_INPUT_SHOULDER_RIGHT_CLICK]);

		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/joystick_left/click", &mHandles[ALVR_INPUT_JOYSTICK_LEFT_CLICK]);
		vr::VRDriverInput()->CreateScalarComponent(mPropertyContainer, "/input/joystick_left/x", &mHandles[ALVR_INPUT_JOYSTICK_LEFT_X], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);
		vr::VRDriverInput()->CreateScalarComponent(mPropertyContainer, "/input/joystick_left/y", &mHandles[ALVR_INPUT_JOYSTICK_LEFT_Y], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);

		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/joystick_right/click", &mHandles[ALVR_INPUT_JOYSTICK_RIGHT_CLICK]);
		vr::VRDriverInput()->CreateScalarComponent(mPropertyContainer, "/input/joystick_right/x", &mHandles[ALVR_INPUT_JOYSTICK_RIGHT_X], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);
		vr::VRDriverInput()->CreateScalarComponent(mPropertyContainer, "/input/joystick_right/y", &mHandles[ALVR_INPUT_JOYSTICK_RIGHT_Y], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);

		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/joystick/click", &mHandles[ALVR_INPUT_JOYSTICK_CLICK]);
		vr::VRDriverInput()->CreateScalarComponent(mPropertyContainer, "/input/joystick/x", &mHandles[ALVR_INPUT_JOYSTICK_X], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);
		vr::VRDriverInput()->CreateScalarComponent(mPropertyContainer, "/input/joystick/y", &mHandles[ALVR_INPUT_JOYSTICK_Y], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/joystick/touch", &mHandles[ALVR_INPUT_JOYSTICK_TOUCH]);

		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/back/click", &mHandles[ALVR_INPUT_BACK_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/guide/click", &mHandles[ALVR_INPUT_GUIDE_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/start/click", &mHandles[ALVR_INPUT_START_CLICK]);

		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/trigger/click", &mHandles[ALVR_INPUT_TRIGGER_CLICK]);
		vr::VRDriverInput()->CreateScalarComponent(mPropertyContainer, "/input/trigger/value", &mHandles[ALVR_INPUT_TRIGGER_VALUE], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedOneSided);
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/trigger/touch", &mHandles[ALVR_INPUT_TRIGGER_TOUCH]);

		vr::VRDriverInput()->CreateScalarComponent(mPropertyContainer, "/input/trackpad/x", &mHandles[ALVR_INPUT_TRACKPAD_X], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);
		vr::VRDriverInput()->CreateScalarComponent(mPropertyContainer, "/input/trackpad/y", &mHandles[ALVR_INPUT_TRACKPAD_Y], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);

		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/trackpad/click", &mHandles[ALVR_INPUT_TRACKPAD_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(mPropertyContainer, "/input/trackpad/touch", &mHandles[ALVR_INPUT_TRACKPAD_TOUCH]);
	}
	vr::VRDriverInput()->CreateHapticComponent(mPropertyContainer, "/output/haptic", &mHapticHandle);

	return vr::VRInitError_None;
}

void OpenVRController::Deactivate()
{
	Log(L"RemoteController::Deactivate");
	mObjectId = vr::k_unTrackedDeviceIndexInvalid;
}

void OpenVRController::EnterStandby()
{
}

void * OpenVRController::GetComponent(const char * pchComponentNameAndVersion)
{
	Log(L"RemoteController::GetComponent. Name=%hs", pchComponentNameAndVersion);

	return NULL;
}

void OpenVRController::PowerOff()
{
}

/** debug request from a client */

void OpenVRController::DebugRequest(const char * pchRequest, char * pchResponseBuffer, uint32_t unResponseBufferSize)
{
	if (unResponseBufferSize >= 1)
		pchResponseBuffer[0] = 0;
}

vr::DriverPose_t OpenVRController::GetPose()
{
	return mPose;
}

bool OpenVRController::IsMyHapticComponent(uint64_t handle) {
	return mHapticHandle == handle;
}

bool OpenVRController::ReportControllerState(int controllerIndex, const TrackingInfo & info, const vr::HmdQuaternion_t controllerRotation, const TrackingVector3 & controllerPosition, bool enableControllerButton, const FreePIE::FreePIEFileMapping & freePIEData) {
	bool recenterRequest = false;

	if (mObjectId == vr::k_unTrackedDeviceIndexInvalid) {
		return false;
	}

	mPose.qRotation = controllerRotation;

	mPose.vecPosition[0] = controllerPosition.x;
	mPose.vecPosition[1] = controllerPosition.y;
	mPose.vecPosition[2] = controllerPosition.z;
	mPose.vecVelocity[0] = info.controller[controllerIndex].linearVelocity.x;
	mPose.vecVelocity[1] = info.controller[controllerIndex].linearVelocity.y;
	mPose.vecVelocity[2] = info.controller[controllerIndex].linearVelocity.z;
	mPose.vecAcceleration[0] = info.controller[controllerIndex].linearAcceleration.x;
	mPose.vecAcceleration[1] = info.controller[controllerIndex].linearAcceleration.y;
	mPose.vecAcceleration[2] = info.controller[controllerIndex].linearAcceleration.z;
	mPose.vecAngularVelocity[0] = info.controller[controllerIndex].angularVelocity.x;
	mPose.vecAngularVelocity[1] = info.controller[controllerIndex].angularVelocity.y;
	mPose.vecAngularVelocity[2] = info.controller[controllerIndex].angularVelocity.z;
	mPose.vecAngularAcceleration[0] = info.controller[controllerIndex].angularAcceleration.x;
	mPose.vecAngularAcceleration[1] = info.controller[controllerIndex].angularAcceleration.y;
	mPose.vecAngularAcceleration[2] = info.controller[controllerIndex].angularAcceleration.z;

	mPose.poseTimeOffset = 0;

	vr::VRServerDriverHost()->TrackedDevicePoseUpdated(mObjectId, mPose, sizeof(vr::DriverPose_t));

	auto& c = info.controller[controllerIndex];

	// If enableControllerButton is set true by FreePIE, we don't use button assign from GUI but use FreePIE.
	// Second controller is always controlled by FreePIE.
	if (enableControllerButton) {
		for (int i = 0; i < FreePIE::ALVR_FREEPIE_BUTTONS; i++) {
			bool value = (freePIEData.controllerButtons[mIndex] & (1 << i)) != 0;
			vr::VRDriverInput()->UpdateBooleanComponent(mHandles[FreePIE::BUTTON_MAP[i]], value, 0.0);
			if (FreePIE::BUTTON_MAP[i] == ALVR_INPUT_TRIGGER_CLICK) {
				vr::VRDriverInput()->UpdateScalarComponent(mHandles[ALVR_INPUT_TRIGGER_VALUE], value ? 1.0f : 0.0f, 0.0);
			}
		}

		vr::VRDriverInput()->UpdateScalarComponent(mHandles[ALVR_INPUT_TRIGGER_VALUE], (float)freePIEData.trigger[mIndex], 0.0);
		vr::VRDriverInput()->UpdateScalarComponent(mHandles[ALVR_INPUT_TRIGGER_LEFT_VALUE], (float)freePIEData.trigger_left[mIndex], 0.0);
		vr::VRDriverInput()->UpdateScalarComponent(mHandles[ALVR_INPUT_TRIGGER_RIGHT_VALUE], (float)freePIEData.trigger_right[mIndex], 0.0);
		vr::VRDriverInput()->UpdateScalarComponent(mHandles[ALVR_INPUT_JOYSTICK_LEFT_X], (float)freePIEData.joystick_left[mIndex][0], 0.0);
		vr::VRDriverInput()->UpdateScalarComponent(mHandles[ALVR_INPUT_JOYSTICK_LEFT_Y], (float)freePIEData.joystick_left[mIndex][1], 0.0);
		vr::VRDriverInput()->UpdateScalarComponent(mHandles[ALVR_INPUT_JOYSTICK_RIGHT_X], (float)freePIEData.joystick_right[mIndex][0], 0.0);
		vr::VRDriverInput()->UpdateScalarComponent(mHandles[ALVR_INPUT_JOYSTICK_RIGHT_Y], (float)freePIEData.joystick_right[mIndex][1], 0.0);
		vr::VRDriverInput()->UpdateScalarComponent(mHandles[ALVR_INPUT_TRACKPAD_X], (float)freePIEData.trackpad[mIndex][0], 0.0);
		vr::VRDriverInput()->UpdateScalarComponent(mHandles[ALVR_INPUT_TRACKPAD_Y], (float)freePIEData.trackpad[mIndex][1], 0.0);
	}
	else {
		Log(L"Controller%d: %08llX %08X", mIndex, c.buttons, c.flags);
		for (int i = 0; i < ALVR_INPUT_COUNT; i++) {
			uint64_t b = ALVR_BUTTON_FLAG(i);
			if ((mPreviousButtons & b) != (c.buttons & b)) {
				int mapped = i;
				if (!mIsTouch) {
					if (i == ALVR_INPUT_TRIGGER_CLICK) {
						mapped = Settings::Instance().mControllerTriggerMode;
					}
					else if (i == ALVR_INPUT_TRACKPAD_CLICK) {
						mapped = Settings::Instance().mControllerTrackpadClickMode;
					}
					else if (i == ALVR_INPUT_TRACKPAD_TOUCH) {
						mapped = Settings::Instance().mControllerTrackpadTouchMode;
					}
					else if (i == ALVR_INPUT_BACK_CLICK) {
						mapped = Settings::Instance().mControllerBackMode;
					}
				}
				bool value = (c.buttons & b) != 0;
				if (mapped != -1 && mapped <= ALVR_INPUT_MAX && mHandles[mapped] != vr::k_ulInvalidInputComponentHandle) {
					vr::VRDriverInput()->UpdateBooleanComponent(mHandles[mapped], value, 0.0);
					if (mapped == ALVR_INPUT_TRIGGER_CLICK && !mIsTouch) {
						vr::VRDriverInput()->UpdateScalarComponent(mHandles[ALVR_INPUT_TRIGGER_VALUE], value ? 1.0f : 0.0f, 0.0);
					}
				}
				if (value && Settings::Instance().mControllerRecenterButton == i) {
					recenterRequest = true;
				}
			}
		}

		if (mIsTouch) {
			vr::VRDriverInput()->UpdateScalarComponent(mHandles[ALVR_INPUT_JOYSTICK_X], c.trackpadPosition.x, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(mHandles[ALVR_INPUT_JOYSTICK_Y], c.trackpadPosition.y, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(mHandles[ALVR_INPUT_TRIGGER_VALUE], c.triggerValue, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(mHandles[ALVR_INPUT_GRIP_VALUE], c.gripValue, 0.0);
		}
		else {
			// Positions are already normalized to -1.0~+1.0 on client side.
			vr::VRDriverInput()->UpdateScalarComponent(mHandles[ALVR_INPUT_TRACKPAD_X], c.trackpadPosition.x, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(mHandles[ALVR_INPUT_TRACKPAD_Y], c.trackpadPosition.y, 0.0);
		}
	}

	// Battery
	vr::VRProperties()->SetFloatProperty(mPropertyContainer, vr::Prop_DeviceBatteryPercentage_Float, c.batteryPercentRemaining / 100.0f);

	mPreviousButtons = c.buttons;
	mPreviousFlags = c.flags;

	return recenterRequest;
}

std::string OpenVRController::GetSerialNumber() {
	char str[100];
	snprintf(str, sizeof(str), "_%s", mIndex == 0 ? "Left" : "Right");
	return Settings::Instance().mControllerSerialNumber + str;
}
