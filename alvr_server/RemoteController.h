#pragma once
#include <openvr_driver.h>
#include "RecenterManager.h"
#include "Logger.h"
#include "Listener.h"
#include "packet_types.h"

enum {
	INPUT_SYSTEM_CLICK = 0,
	INPUT_APPLICATION_MENU_CLICK = 1,
	INPUT_GRIP_CLICK = 2,
	INPUT_DPAD_LEFT_CLICK = 3,
	INPUT_DPAD_UP_CLICK = 4,
	INPUT_DPAD_RIGHT_CLICK = 5,
	INPUT_DPAD_DOWN_CLICK = 6,
	INPUT_A_CLICK = 7,
	INPUT_B_CLICK = 8,
	INPUT_X_CLICK = 9,
	INPUT_Y_CLICK = 10,
	INPUT_TRIGGER_LEFT_VALUE = 11,
	INPUT_TRIGGER_RIGHT_VALUE = 12,
	INPUT_SHOULDER_LEFT_CLICK = 13,
	INPUT_SHOULDER_RIGHT_CLICK = 14,
	INPUT_JOYSTICK_LEFT_CLICK = 15,
	INPUT_JOYSTICK_LEFT_X = 16,
	INPUT_JOYSTICK_LEFT_Y = 17,
	INPUT_JOYSTICK_RIGHT_CLICK = 18,
	INPUT_JOYSTICK_RIGHT_X = 19,
	INPUT_JOYSTICK_RIGHT_Y = 20,
	INPUT_BACK_CLICK = 21,
	INPUT_GUIDE_CLICK = 22,
	INPUT_START_CLICK = 23,
	INPUT_TRIGGER_CLICK = 24,
	INPUT_TRIGGER_VALUE = 25,
	INPUT_TRACKPAD_X = 26,
	INPUT_TRACKPAD_Y = 27,
	INPUT_TRACKPAD_CLICK = 28,
	INPUT_TRACKPAD_TOUCH = 29,

	INPUT_MAX = 29,
	INPUT_COUNT = 30
};

class RemoteControllerServerDriver : public vr::ITrackedDeviceServerDriver
{
public:
	RemoteControllerServerDriver(bool handed, std::shared_ptr<RecenterManager> recenterManager)
		: m_handed(handed)
		, m_recenterManager(recenterManager)
		, m_previousButtons(0)
		, m_previousFlags(0)
		, m_unObjectId(vr::k_unTrackedDeviceIndexInvalid)
	{
		m_info.type = 0;
	}

	virtual ~RemoteControllerServerDriver() {
	}

	//
	// ITrackedDeviceServerDriver
	//

	virtual vr::EVRInitError Activate(vr::TrackedDeviceIndex_t unObjectId)
	{
		Log("RemoteController::Activate. objectId=%d", unObjectId);

		m_unObjectId = unObjectId;
		m_ulPropertyContainer = vr::VRProperties()->TrackedDeviceToPropertyContainer(m_unObjectId);

		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_TrackingSystemName_String, Settings::Instance().m_controllerTrackingSystemName.c_str());
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_ManufacturerName_String, Settings::Instance().m_controllerManufacturerName.c_str());
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_ModelNumber_String, Settings::Instance().m_controllerModelNumber.c_str());
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_RenderModelName_String, Settings::Instance().m_controllerRenderModelName.c_str());

		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_SerialNumber_String, Settings::Instance().m_controllerSerialNumber.c_str());
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_AttachedDeviceId_String, Settings::Instance().m_controllerSerialNumber.c_str());
		//vr::VRProperties()->SetUint64Property(m_ulPropertyContainer, vr::Prop_SupportedButtons_Uint64, m_supportedButtons);

		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_DeviceProvidesBatteryStatus_Bool, true);

		vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_Axis0Type_Int32, vr::k_eControllerAxis_TrackPad);
		//vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_Axis1Type_Int32, vr::k_eControllerAxis_TrackPad);
		//vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_Axis2Type_Int32, vr::k_eControllerAxis_TrackPad);
		//vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_Axis3Type_Int32, vr::k_eControllerAxis_TrackPad);
		//vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_Axis4Type_Int32, vr::k_eControllerAxis_TrackPad);
		vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_ControllerRoleHint_Int32, m_handed ? vr::TrackedControllerRole_LeftHand : vr::TrackedControllerRole_RightHand);

		int i = 0;

		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/system/click", &m_handles[INPUT_SYSTEM_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/application_menu/click", &m_handles[INPUT_APPLICATION_MENU_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/grip/click", &m_handles[INPUT_GRIP_CLICK]);

		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/dpad_left/click", &m_handles[INPUT_DPAD_LEFT_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/dpad_up/click", &m_handles[INPUT_DPAD_UP_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/dpad_right/click", &m_handles[INPUT_DPAD_RIGHT_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/dpad_down/click", &m_handles[INPUT_DPAD_DOWN_CLICK]);

		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/a/click", &m_handles[INPUT_A_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/b/click", &m_handles[INPUT_B_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/x/click", &m_handles[INPUT_X_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/y/click", &m_handles[INPUT_Y_CLICK]);

		vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/trigger_left/value", &m_handles[INPUT_TRIGGER_LEFT_VALUE], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedOneSided);
		vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/trigger_right/value", &m_handles[INPUT_TRIGGER_RIGHT_VALUE], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedOneSided);
		
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/shoulder_left/click", &m_handles[INPUT_SHOULDER_LEFT_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/shoulder_right/click", &m_handles[INPUT_SHOULDER_RIGHT_CLICK]);

		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/joystick_left/click", &m_handles[INPUT_JOYSTICK_LEFT_CLICK]);
		vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/joystick_left/x", &m_handles[INPUT_JOYSTICK_LEFT_X], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);
		vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/joystick_left/y", &m_handles[INPUT_JOYSTICK_LEFT_Y], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);
		
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/joystick_right/click", &m_handles[INPUT_JOYSTICK_RIGHT_CLICK]);
		vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/joystick_right/x", &m_handles[INPUT_JOYSTICK_RIGHT_X], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);
		vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/joystick_right/y", &m_handles[INPUT_JOYSTICK_RIGHT_Y], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);
		
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/back/click", &m_handles[INPUT_BACK_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/guide/click", &m_handles[INPUT_GUIDE_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/start/click", &m_handles[INPUT_START_CLICK]);

		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/trigger/click", &m_handles[INPUT_TRIGGER_CLICK]);
		vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/trigger/value", &m_handles[INPUT_TRIGGER_VALUE], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedOneSided);
		
		vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/trackpad/x", &m_handles[INPUT_TRACKPAD_X], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);
		vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/trackpad/y", &m_handles[INPUT_TRACKPAD_Y], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);
		
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/trackpad/click", &m_handles[INPUT_TRACKPAD_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/trackpad/touch", &m_handles[INPUT_TRACKPAD_TOUCH]);

		return vr::VRInitError_None;
	}

	virtual void Deactivate()
	{
		Log("RemoteController::Deactivate");
		m_unObjectId = vr::k_unTrackedDeviceIndexInvalid;
	}

	virtual void EnterStandby()
	{
	}

	void *GetComponent(const char *pchComponentNameAndVersion)
	{
		Log("RemoteController::GetComponent. Name=%s", pchComponentNameAndVersion);

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
		vr::DriverPose_t pose = { 0 };
		pose.poseIsValid = true;
		pose.result = vr::TrackingResult_Running_OK;
		pose.deviceIsConnected = true;

		pose.qWorldFromDriverRotation = HmdQuaternion_Init(1, 0, 0, 0);
		pose.qDriverFromHeadRotation = HmdQuaternion_Init(1, 0, 0, 0);
		pose.qRotation = HmdQuaternion_Init(1, 0, 0, 0);

		if (m_info.type != 0) {
			Log("Controller Flags=%d Quot:%f,%f,%f,%f\nPos:%f,%f,%f\nButtons: %08X\n"
				"Trackpad: %f, %f\nBattery=%d Recenter=%d",
				m_info.flags,
				m_info.controller_Pose_Orientation.x,
				m_info.controller_Pose_Orientation.y,
				m_info.controller_Pose_Orientation.z,
				m_info.controller_Pose_Orientation.w,
				m_info.controller_Pose_Position.x,
				m_info.controller_Pose_Position.y,
				m_info.controller_Pose_Position.z,
				m_info.controllerButtons,
				m_info.controllerTrackpadPosition.x,
				m_info.controllerTrackpadPosition.y,
				m_info.controllerBatteryPercentRemaining,
				m_info.controllerRecenterCount
			);

			pose.qRotation = m_recenterManager->GetRecentered(m_info.controller_Pose_Orientation);

			TrackingVector3 position = m_recenterManager->GetRecenteredVector(m_info.controller_Pose_Position);
			pose.vecPosition[0] = position.x;
			pose.vecPosition[1] = position.y;
			pose.vecPosition[2] = position.z;
			if (m_info.flags & TrackingInfo::FLAG_OTHER_TRACKING_SOURCE) {
				pose.vecPosition[0] += m_info.Other_Tracking_Source_Position.x;
				pose.vecPosition[1] += m_info.Other_Tracking_Source_Position.y;
				pose.vecPosition[2] += m_info.Other_Tracking_Source_Position.z;
			}

			if (Settings::Instance().m_EnableOffsetPos) {
				pose.vecPosition[0] += Settings::Instance().m_OffsetPos[0];
				pose.vecPosition[1] += Settings::Instance().m_OffsetPos[1];
				pose.vecPosition[2] += Settings::Instance().m_OffsetPos[2];
			}

			pose.poseTimeOffset = 0;
		}

		return pose;
	}

	bool ReportControllerState(const TrackingInfo &info) {
		bool recenterRequest = false;

		m_info = info;

		if (m_unObjectId == vr::k_unTrackedDeviceIndexInvalid) {
			return false;
		}

		vr::VRServerDriverHost()->TrackedDevicePoseUpdated(m_unObjectId, GetPose(), sizeof(vr::DriverPose_t));

		int32_t triggerButton = Settings::Instance().m_controllerTriggerMode;
		int32_t trackpadClickButton = Settings::Instance().m_controllerTrackpadClickMode;
		int32_t trackpadTouchButton = Settings::Instance().m_controllerTrackpadTouchMode;

		// Trigger pressed (ovrButton_A)
		if ((m_previousButtons & 0x00000001) != (info.controllerButtons & 0x00000001)) {
			bool value = (info.controllerButtons & 0x00000001) != 0;
			if (triggerButton != -1) {
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[triggerButton], value, 0.0);
				if (triggerButton == INPUT_TRIGGER_CLICK) {
					vr::VRDriverInput()->UpdateScalarComponent(m_handles[INPUT_TRIGGER_VALUE], value ? 1.0f : 0.0f, 0.0);
				}
			}
			if (value && Settings::Instance().m_controllerRecenterButton == 1) {
				recenterRequest = true;
			}
		}

		// Trackpad click (ovrButton_Enter)
		if ((m_previousButtons & 0x00100000) != (info.controllerButtons & 0x00100000)) {
			bool value = (info.controllerButtons & 0x00100000) != 0;
			if (triggerButton != -1) {
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[trackpadClickButton], value, 0.0);
			}
			if (value && Settings::Instance().m_controllerRecenterButton == 2) {
				recenterRequest = true;
			}
		}

		// Back button (ovrButton_Back)
		// This event is not sent normally.
		// TODO: How we get it work?
		if ((m_previousButtons & 0x00200000) != (info.controllerButtons & 0x00200000)) {
			bool value = (info.controllerButtons & 0x00200000) != 0;
			if (triggerButton != -1) {
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[vr::k_EButton_Dashboard_Back], value, 0.0);
			}
			if (value && Settings::Instance().m_controllerRecenterButton == 4) {
				recenterRequest = true;
			}
		}
		// Trackpad touch
		if ((m_previousFlags & TrackingInfo::FLAG_CONTROLLER_TRACKPAD_TOUCH) != (info.flags & TrackingInfo::FLAG_CONTROLLER_TRACKPAD_TOUCH)) {
			bool value = (info.flags & TrackingInfo::FLAG_CONTROLLER_TRACKPAD_TOUCH) != 0;
			if (trackpadTouchButton != -1) {
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[trackpadTouchButton], value, 0.0);
			}
			if (value && Settings::Instance().m_controllerRecenterButton == 3) {
				recenterRequest = true;
			}
		}

		// Positions are already normalized to -1.0~+1.0 on client side.
		vr::VRDriverInput()->UpdateScalarComponent(m_handles[INPUT_TRACKPAD_X], info.controllerTrackpadPosition.x, 0.0);
		vr::VRDriverInput()->UpdateScalarComponent(m_handles[INPUT_TRACKPAD_Y], info.controllerTrackpadPosition.y, 0.0);

		// Battery
		vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_DeviceBatteryPercentage_Float, info.controllerBatteryPercentRemaining / 100.0f);

		m_previousButtons = info.controllerButtons;
		m_previousFlags = info.flags;

		return recenterRequest;
	}

	std::string GetSerialNumber() {
		return Settings::Instance().m_controllerSerialNumber;
	}

private:
	std::shared_ptr<RecenterManager> m_recenterManager;

	vr::TrackedDeviceIndex_t m_unObjectId;
	vr::PropertyContainerHandle_t m_ulPropertyContainer;

	uint32_t m_previousButtons;
	uint32_t m_previousFlags;

	bool m_handed;

	TrackingInfo m_info;

	vr::VRInputComponentHandle_t m_handles[INPUT_COUNT];
};
