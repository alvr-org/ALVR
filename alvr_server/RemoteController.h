#pragma once
#include <openvr_driver.h>
#include "RecenterManager.h"
#include "Logger.h"
#include "Listener.h"
#include "packet_types.h"

class RemoteControllerComponent;

class RemoteControllerServerDriver : public vr::ITrackedDeviceServerDriver
{
public:
	RemoteControllerServerDriver(bool handed, std::shared_ptr<RecenterManager> recenterManager)
		: m_handed(handed)
		, m_recenterManager(recenterManager) {
		m_supportedButtons = vr::ButtonMaskFromId(vr::k_EButton_SteamVR_Trigger)
			| vr::ButtonMaskFromId(vr::k_EButton_SteamVR_Touchpad)
			| vr::ButtonMaskFromId(vr::k_EButton_Dashboard_Back)
			| vr::ButtonMaskFromId(vr::k_EButton_Axis0)
			| vr::ButtonMaskFromId(vr::k_EButton_Axis1);
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

		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_ModelNumber_String, Settings::Instance().m_controllerModelNumber.c_str());
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_RenderModelName_String, Settings::Instance().m_controllerModelNumber.c_str());

		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_AttachedDeviceId_String, Settings::Instance().m_controllerSerialNumber.c_str());
		vr::VRProperties()->SetUint64Property(m_ulPropertyContainer, vr::Prop_SupportedButtons_Uint64, m_supportedButtons);

		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_DeviceProvidesBatteryStatus_Bool, true);

		vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_Axis0Type_Int32, vr::k_eControllerAxis_TrackPad);
		//vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_Axis1Type_Int32, vr::k_eControllerAxis_TrackPad);
		//vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_Axis2Type_Int32, vr::k_eControllerAxis_TrackPad);
		//vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_Axis3Type_Int32, vr::k_eControllerAxis_TrackPad);
		//vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_Axis4Type_Int32, vr::k_eControllerAxis_TrackPad);
		vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_ControllerRoleHint_Int32, m_handed ? vr::TrackedControllerRole_LeftHand : vr::TrackedControllerRole_RightHand);

		m_component = std::make_shared<RemoteControllerComponent>();

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
		if (!_stricmp(pchComponentNameAndVersion, vr::IVRControllerComponent_Version))
		{
			return m_component.get();
		}

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
				m_info.controllerFlags,
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
			//pose.qRotation.x = m_info.controller_Pose_Orientation.x;
			//pose.qRotation.y = m_info.controller_Pose_Orientation.y;
			//pose.qRotation.z = m_info.controller_Pose_Orientation.z;
			//pose.qRotation.w = m_info.controller_Pose_Orientation.w;

			TrackingVector3 position = m_recenterManager->GetRecenteredVector(m_info.controller_Pose_Position);
			pose.vecPosition[0] = position.x;
			pose.vecPosition[1] = position.y;
			pose.vecPosition[2] = position.z;
			//pose.vecPosition[0] = m_info.controller_Pose_Position.x;
			//pose.vecPosition[1] = m_info.controller_Pose_Position.y;
			//pose.vecPosition[2] = m_info.controller_Pose_Position.z;

			pose.poseTimeOffset = 0;
		}

		return pose;
	}

	bool ReportControllerState(const TrackingInfo &info) {
		bool recenterRequest = false;

		m_info = info;

		vr::VRServerDriverHost()->TrackedDevicePoseUpdated(m_unObjectId, GetPose(), sizeof(vr::DriverPose_t));

		vr::EVRButtonId triggerButton = (vr::EVRButtonId)Settings::Instance().m_controllerTriggerMode;
		vr::EVRButtonId trackpadClickButton = (vr::EVRButtonId)Settings::Instance().m_controllerTrackpadClickMode;
		vr::EVRButtonId trackpadTouchButton = (vr::EVRButtonId)Settings::Instance().m_controllerTrackpadTouchMode;

		// Trigger pressed (ovrButton_A)
		if ((m_previousButtons & 0x00000001) != 0) {
			if ((info.controllerButtons & 0x00000001) == 0) {
				if (Settings::Instance().m_controllerTriggerMode != -1) {
					vr::VRServerDriverHost()->TrackedDeviceButtonUnpressed(m_unObjectId, triggerButton, 0.0);
					vr::VRServerDriverHost()->TrackedDeviceButtonUntouched(m_unObjectId, triggerButton, 0.0);
				}
			}
		}
		else {
			if ((info.controllerButtons & 0x00000001) != 0) {
				if (Settings::Instance().m_controllerTriggerMode != -1) {
					vr::VRServerDriverHost()->TrackedDeviceButtonPressed(m_unObjectId, triggerButton, 0.0);
					vr::VRServerDriverHost()->TrackedDeviceButtonTouched(m_unObjectId, triggerButton, 0.0);
				}
				if (Settings::Instance().m_controllerRecenterButton == 1) {
					recenterRequest = true;
				}
			}
		}

		// Trackpad click (ovrButton_Enter)
		if ((m_previousButtons & 0x00100000) != 0) {
			if ((info.controllerButtons & 0x00100000) == 0) {
				if (Settings::Instance().m_controllerTrackpadClickMode != -1) {
					vr::VRServerDriverHost()->TrackedDeviceButtonUnpressed(m_unObjectId, trackpadClickButton, 0.0);
				}
			}
		}
		else {
			if ((info.controllerButtons & 0x00100000) != 0) {
				if (Settings::Instance().m_controllerTrackpadClickMode != -1) {
					vr::VRServerDriverHost()->TrackedDeviceButtonPressed(m_unObjectId, trackpadClickButton, 0.0);
				}
				if (Settings::Instance().m_controllerRecenterButton == 2) {
					recenterRequest = true;
				}
			}
		}

		// Back button (ovrButton_Back)
		// This event is not sent normally.
		// TODO: How we get it work?
		if ((m_previousButtons & 0x00200000) != 0) {
			if ((info.controllerButtons & 0x00200000) == 0) {
				vr::VRServerDriverHost()->TrackedDeviceButtonUnpressed(m_unObjectId, vr::k_EButton_Dashboard_Back, 0.0);
				vr::VRServerDriverHost()->TrackedDeviceButtonUntouched(m_unObjectId, vr::k_EButton_Dashboard_Back, 0.0);
			}
		}
		else {
			if ((info.controllerButtons & 0x00200000) != 0) {
				vr::VRServerDriverHost()->TrackedDeviceButtonPressed(m_unObjectId, vr::k_EButton_Dashboard_Back, 0.0);
				vr::VRServerDriverHost()->TrackedDeviceButtonTouched(m_unObjectId, vr::k_EButton_Dashboard_Back, 0.0);
			}
		}
		// Trackpad touch
		if ((m_previousFlags & TrackingInfo::CONTROLLER_FLAG_TRACKPAD_TOUCH) != 0) {
			if ((info.controllerFlags & TrackingInfo::CONTROLLER_FLAG_TRACKPAD_TOUCH) == 0) {
				if (Settings::Instance().m_controllerTrackpadTouchMode != -1) {
					vr::VRServerDriverHost()->TrackedDeviceButtonUntouched(m_unObjectId, trackpadTouchButton, 0.0);
				}
			}
		}
		else {
			if ((info.controllerFlags & TrackingInfo::CONTROLLER_FLAG_TRACKPAD_TOUCH) != 0) {
				if (Settings::Instance().m_controllerTrackpadTouchMode != -1) {
					vr::VRServerDriverHost()->TrackedDeviceButtonTouched(m_unObjectId, trackpadTouchButton, 0.0);
				}
				if (Settings::Instance().m_controllerRecenterButton == 3) {
					recenterRequest = true;
				}
			}
		}

		vr::VRControllerAxis_t axis;
		// Positions are already normalized to -1.0~+1.0 on client side.
		axis.x = info.controllerTrackpadPosition.x;
		axis.y = info.controllerTrackpadPosition.y;
		vr::VRServerDriverHost()->TrackedDeviceAxisUpdated(m_unObjectId, 0, axis);

		// Battery
		vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_DeviceBatteryPercentage_Float, info.controllerBatteryPercentRemaining / 100.0f);

		m_previousButtons = info.controllerButtons;
		m_previousFlags = info.controllerFlags;

		return recenterRequest;
	}

	std::string GetSerialNumber() {
		return Settings::Instance().m_controllerSerialNumber;
	}

private:
	std::shared_ptr<RecenterManager> m_recenterManager;
	std::shared_ptr<RemoteControllerComponent> m_component;

	vr::TrackedDeviceIndex_t m_unObjectId;
	vr::PropertyContainerHandle_t m_ulPropertyContainer;

	uint32_t m_previousButtons;
	uint32_t m_previousFlags;

	uint64_t m_supportedButtons;
	bool m_handed;

	TrackingInfo m_info;
};

// We really need this implementation???
class RemoteControllerComponent : public vr::IVRControllerComponent
{
public:
	RemoteControllerComponent() {
	}

	/** Gets the current state of a controller. */
	virtual vr::VRControllerState_t GetControllerState() override {
		return vr::VRControllerState_t();
	}

	/** Returns a uint64 property. If the property is not available this function will return 0. */
	virtual bool TriggerHapticPulse(uint32_t unAxisId, uint16_t usPulseDurationMicroseconds) override {
		Log("IVRControllerComponent::TriggerHapticPulse AxisId=%d Duration=%d", unAxisId, usPulseDurationMicroseconds);
		return 0;
	}
};

