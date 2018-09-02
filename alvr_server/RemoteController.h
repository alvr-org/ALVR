#pragma once
#include <openvr_driver.h>
#include <string>
#include "Logger.h"
#include "Listener.h"
#include "packet_types.h"
#include "FreePIE.h"

class RemoteControllerServerDriver : public vr::ITrackedDeviceServerDriver
{
public:
	RemoteControllerServerDriver(bool hand, int index)
		: m_hand(hand)
		, m_previousButtons(0)
		, m_previousFlags(0)
		, m_unObjectId(vr::k_unTrackedDeviceIndexInvalid)
		, m_index(index)
	{
		memset(&m_pose, 0, sizeof(m_pose));
		m_pose.poseIsValid = true;
		m_pose.result = vr::TrackingResult_Running_OK;
		m_pose.deviceIsConnected = true;

		m_pose.qWorldFromDriverRotation = HmdQuaternion_Init(1, 0, 0, 0);
		m_pose.qDriverFromHeadRotation = HmdQuaternion_Init(1, 0, 0, 0);
		m_pose.qRotation = HmdQuaternion_Init(1, 0, 0, 0);

		for (int i = 0; i < ALVR_INPUT_COUNT; i++) {
			m_handles[i] = vr::k_ulInvalidInputComponentHandle;
		}
		mIsTouch = Settings::Instance().m_controllerType == "oculus_touch";
	}

	virtual ~RemoteControllerServerDriver() {
	}

	bool GetHand() {
		return m_hand;
	}

	//
	// ITrackedDeviceServerDriver
	//

	virtual vr::EVRInitError Activate(vr::TrackedDeviceIndex_t unObjectId)
	{
		Log(L"RemoteController::Activate. objectId=%d", unObjectId);

		m_unObjectId = unObjectId;
		m_ulPropertyContainer = vr::VRProperties()->TrackedDeviceToPropertyContainer(m_unObjectId);

		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_TrackingSystemName_String, Settings::Instance().m_controllerTrackingSystemName.c_str());
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_ManufacturerName_String, Settings::Instance().m_controllerManufacturerName.c_str());
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_ModelNumber_String, Settings::Instance().m_controllerModelNumber.c_str());

		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_RenderModelName_String, m_hand ? Settings::Instance().m_controllerRenderModelNameLeft.c_str() : Settings::Instance().m_controllerRenderModelNameRight.c_str());

		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_SerialNumber_String, GetSerialNumber().c_str());
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_AttachedDeviceId_String, GetSerialNumber().c_str());

		uint64_t supportedButtons = 0xFFFFFFFFFFFFFFFFULL;
		vr::VRProperties()->SetUint64Property(m_ulPropertyContainer, vr::Prop_SupportedButtons_Uint64, supportedButtons);

		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_DeviceProvidesBatteryStatus_Bool, true);

		if (mIsTouch) {
			vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_Axis0Type_Int32, vr::k_eControllerAxis_Joystick);
		}
		else {
			vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_Axis0Type_Int32, vr::k_eControllerAxis_TrackPad);
		}
		vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_ControllerRoleHint_Int32, m_hand ? vr::TrackedControllerRole_LeftHand : vr::TrackedControllerRole_RightHand);

		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_ControllerType_String, Settings::Instance().m_controllerType.c_str());
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_LegacyInputProfile_String, Settings::Instance().m_controllerLegacyInputProfile.c_str());
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_InputProfilePath_String, Settings::Instance().m_controllerInputProfilePath.c_str());
		int i = 0;

		if (mIsTouch) {
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/system/click", &m_handles[ALVR_INPUT_SYSTEM_CLICK]);
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/application_menu/click", &m_handles[ALVR_INPUT_APPLICATION_MENU_CLICK]);
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/grip/click", &m_handles[ALVR_INPUT_GRIP_CLICK]);
			vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/grip/value", &m_handles[ALVR_INPUT_GRIP_VALUE], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedOneSided);
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/grip/touch", &m_handles[ALVR_INPUT_GRIP_TOUCH]);

			if (!m_hand) {
				// A,B for right hand.
				vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/a/click", &m_handles[ALVR_INPUT_A_CLICK]);
				vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/a/touch", &m_handles[ALVR_INPUT_A_TOUCH]);
				vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/b/click", &m_handles[ALVR_INPUT_B_CLICK]);
				vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/b/touch", &m_handles[ALVR_INPUT_B_TOUCH]);
			}
			else {
				// X,Y for left hand.
				vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/x/click", &m_handles[ALVR_INPUT_X_CLICK]);
				vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/x/touch", &m_handles[ALVR_INPUT_X_TOUCH]);
				vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/y/click", &m_handles[ALVR_INPUT_Y_CLICK]);
				vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/y/touch", &m_handles[ALVR_INPUT_Y_TOUCH]);
			}

			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/joystick/click", &m_handles[ALVR_INPUT_JOYSTICK_CLICK]);
			vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/joystick/x", &m_handles[ALVR_INPUT_JOYSTICK_X], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);
			vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/joystick/y", &m_handles[ALVR_INPUT_JOYSTICK_Y], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/joystick/touch", &m_handles[ALVR_INPUT_JOYSTICK_TOUCH]);

			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/back/click", &m_handles[ALVR_INPUT_BACK_CLICK]);
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/guide/click", &m_handles[ALVR_INPUT_GUIDE_CLICK]);
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/start/click", &m_handles[ALVR_INPUT_START_CLICK]);

			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/trigger/click", &m_handles[ALVR_INPUT_TRIGGER_CLICK]);
			vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/trigger/value", &m_handles[ALVR_INPUT_TRIGGER_VALUE], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedOneSided);
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/trigger/touch", &m_handles[ALVR_INPUT_TRIGGER_TOUCH]);
		}
		else {
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/system/click", &m_handles[ALVR_INPUT_SYSTEM_CLICK]);
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/application_menu/click", &m_handles[ALVR_INPUT_APPLICATION_MENU_CLICK]);
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/grip/click", &m_handles[ALVR_INPUT_GRIP_CLICK]);
			vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/grip/value", &m_handles[ALVR_INPUT_GRIP_VALUE], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedOneSided);
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/grip/touch", &m_handles[ALVR_INPUT_GRIP_TOUCH]);

			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/dpad_left/click", &m_handles[ALVR_INPUT_DPAD_LEFT_CLICK]);
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/dpad_up/click", &m_handles[ALVR_INPUT_DPAD_UP_CLICK]);
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/dpad_right/click", &m_handles[ALVR_INPUT_DPAD_RIGHT_CLICK]);
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/dpad_down/click", &m_handles[ALVR_INPUT_DPAD_DOWN_CLICK]);

			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/a/click", &m_handles[ALVR_INPUT_A_CLICK]);
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/a/touch", &m_handles[ALVR_INPUT_A_TOUCH]);
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/b/click", &m_handles[ALVR_INPUT_B_CLICK]);
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/b/touch", &m_handles[ALVR_INPUT_B_TOUCH]);
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/x/click", &m_handles[ALVR_INPUT_X_CLICK]);
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/x/touch", &m_handles[ALVR_INPUT_X_TOUCH]);
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/y/click", &m_handles[ALVR_INPUT_Y_CLICK]);
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/y/touch", &m_handles[ALVR_INPUT_Y_TOUCH]);

			vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/trigger_left/value", &m_handles[ALVR_INPUT_TRIGGER_LEFT_VALUE], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedOneSided);
			vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/trigger_right/value", &m_handles[ALVR_INPUT_TRIGGER_RIGHT_VALUE], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedOneSided);

			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/shoulder_left/click", &m_handles[ALVR_INPUT_SHOULDER_LEFT_CLICK]);
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/shoulder_right/click", &m_handles[ALVR_INPUT_SHOULDER_RIGHT_CLICK]);

			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/joystick_left/click", &m_handles[ALVR_INPUT_JOYSTICK_LEFT_CLICK]);
			vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/joystick_left/x", &m_handles[ALVR_INPUT_JOYSTICK_LEFT_X], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);
			vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/joystick_left/y", &m_handles[ALVR_INPUT_JOYSTICK_LEFT_Y], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);

			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/joystick_right/click", &m_handles[ALVR_INPUT_JOYSTICK_RIGHT_CLICK]);
			vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/joystick_right/x", &m_handles[ALVR_INPUT_JOYSTICK_RIGHT_X], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);
			vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/joystick_right/y", &m_handles[ALVR_INPUT_JOYSTICK_RIGHT_Y], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);

			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/joystick/click", &m_handles[ALVR_INPUT_JOYSTICK_CLICK]);
			vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/joystick/x", &m_handles[ALVR_INPUT_JOYSTICK_X], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);
			vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/joystick/y", &m_handles[ALVR_INPUT_JOYSTICK_Y], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/joystick/touch", &m_handles[ALVR_INPUT_JOYSTICK_TOUCH]);

			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/back/click", &m_handles[ALVR_INPUT_BACK_CLICK]);
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/guide/click", &m_handles[ALVR_INPUT_GUIDE_CLICK]);
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/start/click", &m_handles[ALVR_INPUT_START_CLICK]);

			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/trigger/click", &m_handles[ALVR_INPUT_TRIGGER_CLICK]);
			vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/trigger/value", &m_handles[ALVR_INPUT_TRIGGER_VALUE], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedOneSided);
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/trigger/touch", &m_handles[ALVR_INPUT_TRIGGER_TOUCH]);

			vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/trackpad/x", &m_handles[ALVR_INPUT_TRACKPAD_X], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);
			vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/trackpad/y", &m_handles[ALVR_INPUT_TRACKPAD_Y], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);

			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/trackpad/click", &m_handles[ALVR_INPUT_TRACKPAD_CLICK]);
			vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/trackpad/touch", &m_handles[ALVR_INPUT_TRACKPAD_TOUCH]);
		}
		vr::VRDriverInput()->CreateHapticComponent(m_ulPropertyContainer, "/output/haptic", &m_compHaptic);

		return vr::VRInitError_None;
	}

	virtual void Deactivate()
	{
		Log(L"RemoteController::Deactivate");
		m_unObjectId = vr::k_unTrackedDeviceIndexInvalid;
	}

	virtual void EnterStandby()
	{
	}

	void *GetComponent(const char *pchComponentNameAndVersion)
	{
		Log(L"RemoteController::GetComponent. Name=%hs", pchComponentNameAndVersion);

		return NULL;
	}

	virtual void PowerOff()
	{
	}

	/** debug request from a client */
	virtual void DebugRequest(const char *pchRequest, char *pchResponseBuffer, uint32_t unResponseBufferSize)
	{
		if (unResponseBufferSize >= 1)
			pchResponseBuffer[0] = 0;
	}

	virtual vr::DriverPose_t GetPose()
	{
		return m_pose;
	}

	bool IsMyHapticComponent(uint64_t handle){
		return m_compHaptic == handle;
	}

	bool ReportControllerState(int controllerIndex, const TrackingInfo &info
		, const vr::HmdQuaternion_t controllerRotation, const TrackingVector3 &controllerPosition
		, bool enableControllerButton, const FreePIE::FreePIEFileMapping &freePIEData) {
		bool recenterRequest = false;

		if (m_unObjectId == vr::k_unTrackedDeviceIndexInvalid) {
			return false;
		}

		m_pose.qRotation = controllerRotation;

		m_pose.vecPosition[0] = controllerPosition.x;
		m_pose.vecPosition[1] = controllerPosition.y;
		m_pose.vecPosition[2] = controllerPosition.z;

		m_pose.poseTimeOffset = 0;

		vr::VRServerDriverHost()->TrackedDevicePoseUpdated(m_unObjectId, m_pose, sizeof(vr::DriverPose_t));

		auto& c = info.controller[controllerIndex];

		// If enableControllerButton is set true by FreePIE, we don't use button assign from GUI but use FreePIE.
		// Second controller is always controlled by FreePIE.
		if (enableControllerButton) {
			for (int i = 0; i < FreePIE::ALVR_FREEPIE_BUTTONS; i++) {
				bool value = (freePIEData.controllerButtons[m_index] & (1 << i)) != 0;
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[FreePIE::BUTTON_MAP[i]], value, 0.0);
				if (FreePIE::BUTTON_MAP[i] == ALVR_INPUT_TRIGGER_CLICK) {
					vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRIGGER_VALUE], value ? 1.0f : 0.0f, 0.0);
				}
			}

			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRIGGER_VALUE], (float)freePIEData.trigger[m_index], 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRIGGER_LEFT_VALUE], (float)freePIEData.trigger_left[m_index], 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRIGGER_RIGHT_VALUE], (float)freePIEData.trigger_right[m_index], 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_JOYSTICK_LEFT_X], (float)freePIEData.joystick_left[m_index][0], 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_JOYSTICK_LEFT_Y], (float)freePIEData.joystick_left[m_index][1], 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_JOYSTICK_RIGHT_X], (float)freePIEData.joystick_right[m_index][0], 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_JOYSTICK_RIGHT_Y], (float)freePIEData.joystick_right[m_index][1], 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRACKPAD_X], (float)freePIEData.trackpad[m_index][0], 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRACKPAD_Y], (float)freePIEData.trackpad[m_index][1], 0.0);
		}
		else {
			Log(L"Controller%d: %08llX %08X", m_index, c.buttons, c.flags);
			for (int i = 0; i < ALVR_INPUT_COUNT; i++) {
				uint64_t b = ALVR_BUTTON_FLAG(i);
				if ((m_previousButtons & b) != (c.buttons & b)) {
					int mapped = i;
					if (!mIsTouch) {
						if (i == ALVR_INPUT_TRIGGER_CLICK) {
							mapped = Settings::Instance().m_controllerTriggerMode;
						}
						else if (i == ALVR_INPUT_TRACKPAD_CLICK) {
							mapped = Settings::Instance().m_controllerTrackpadClickMode;
						}
						else if (i == ALVR_INPUT_TRACKPAD_TOUCH) {
							mapped = Settings::Instance().m_controllerTrackpadTouchMode;
						}
						else if (i == ALVR_INPUT_BACK_CLICK) {
							mapped = Settings::Instance().m_controllerBackMode;
						}
					}
					bool value = (c.buttons & b) != 0;
					if (mapped != -1 && mapped <= ALVR_INPUT_MAX && m_handles[mapped] != vr::k_ulInvalidInputComponentHandle) {
						vr::VRDriverInput()->UpdateBooleanComponent(m_handles[mapped], value, 0.0);
						if (mapped == ALVR_INPUT_TRIGGER_CLICK && !mIsTouch) {
							vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRIGGER_VALUE], value ? 1.0f : 0.0f, 0.0);
						}
					}
					if (value && Settings::Instance().m_controllerRecenterButton == i) {
						recenterRequest = true;
					}
				}
			}

			if (mIsTouch) {
				vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_JOYSTICK_X], c.trackpadPosition.x, 0.0);
				vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_JOYSTICK_Y], c.trackpadPosition.y, 0.0);
				vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRIGGER_VALUE], c.triggerValue, 0.0);
				vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_GRIP_VALUE], c.gripValue, 0.0);
			}
			else {
				// Positions are already normalized to -1.0~+1.0 on client side.
				vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRACKPAD_X], c.trackpadPosition.x, 0.0);
				vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRACKPAD_Y], c.trackpadPosition.y, 0.0);
			}
		}

		// Battery
		vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_DeviceBatteryPercentage_Float, c.batteryPercentRemaining / 100.0f);

		m_previousButtons = c.buttons;
		m_previousFlags = c.flags;

		return recenterRequest;
	}

	std::string GetSerialNumber() {
		char str[100];
		snprintf(str, sizeof(str), "-%d", m_index);
		return Settings::Instance().m_controllerSerialNumber + str;
	}

private:
	vr::TrackedDeviceIndex_t m_unObjectId;
	vr::PropertyContainerHandle_t m_ulPropertyContainer;

	uint64_t m_previousButtons;
	uint32_t m_previousFlags;

	bool m_hand;
	int m_index;
	bool mIsTouch;

	vr::VRInputComponentHandle_t m_handles[ALVR_INPUT_COUNT];
	vr::VRInputComponentHandle_t m_compHaptic;

	vr::DriverPose_t m_pose;
};
