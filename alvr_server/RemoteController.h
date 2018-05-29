#pragma once
#include <openvr_driver.h>
#include "Logger.h"
#include "Listener.h"

class RemoteController : public vr::ITrackedDeviceServerDriver, public vr::IVRControllerComponent
{
public:
	RemoteController(uint64_t supportedButtons, bool handed, std::shared_ptr<Listener> listener)
	: m_supportedButtons(m_supportedButtons)
	, m_handed(handed)
	, m_Listener(listener) {
	}

	virtual ~RemoteController() {
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
		
		vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_Axis0Type_Int32, vr::k_eControllerAxis_TrackPad);
		vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_Axis1Type_Int32, vr::k_eControllerAxis_TrackPad);
		//vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_Axis2Type_Int32, vr::k_eControllerAxis_TrackPad);
		//vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_Axis3Type_Int32, vr::k_eControllerAxis_TrackPad);
		//vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_Axis4Type_Int32, vr::k_eControllerAxis_TrackPad);
		vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_ControllerRoleHint_Int32, m_handed ? vr::TrackedControllerRole_LeftHand : vr::TrackedControllerRole_RightHand);

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
			return static_cast<vr::IVRControllerComponent*>(this);
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

		if (m_Listener->HasValidTrackingInfo()) {
			TrackingInfo info;
			m_Listener->GetTrackingInfo(info);

			pose.qRotation.x = info.controller_Pose_Orientation.x;
			pose.qRotation.y = info.controller_Pose_Orientation.y;
			pose.qRotation.z = info.controller_Pose_Orientation.z;
			pose.qRotation.w = info.controller_Pose_Orientation.w;

			pose.vecPosition[0] = info.controller_Pose_Position.x;
			pose.vecPosition[1] = info.controller_Pose_Position.y;
			pose.vecPosition[2] = info.controller_Pose_Position.z;

			pose.poseTimeOffset = 0;
		}

		return pose;
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

	void ReportControllerState() {
		vr::VRServerDriverHost()->TrackedDevicePoseUpdated(m_unObjectId, GetPose(), sizeof(vr::DriverPose_t));
	}

	std::string GetSerialNumber() {
		return Settings::Instance().m_controllerSerialNumber;
	}

private:
	vr::TrackedDeviceIndex_t m_unObjectId;
	vr::PropertyContainerHandle_t m_ulPropertyContainer;

	uint64_t m_supportedButtons;
	bool m_handed;

	std::shared_ptr<Listener> m_Listener;
};

