#include "OpenVRFakeTrackingReference.h"

#include "Utils.h"

vr::EVRInitError OpenVRFakeTrackingReference::Activate(vr::TrackedDeviceIndex_t unObjectId)
{
	Log(L"OpenVRFakeTrackingReference::Activate. objectId=%d", unObjectId);

	mObjectId = unObjectId;
	mPropertyContainer = vr::VRProperties()->TrackedDeviceToPropertyContainer(mObjectId);

	vr::VRProperties()->SetStringProperty(mPropertyContainer, vr::Prop_ModelNumber_String, "OpenVRFakeTrackingReference-Model001");
	vr::VRProperties()->SetStringProperty(mPropertyContainer, vr::Prop_RenderModelName_String, "OpenVRFakeTrackingReference-Model001");

	vr::VRProperties()->SetStringProperty(mPropertyContainer, vr::Prop_AttachedDeviceId_String, GetSerialNumber().c_str());

	return vr::VRInitError_None;
}

void OpenVRFakeTrackingReference::Deactivate()
{
	Log(L"OpenVRFakeTrackingReference::Deactivate");
	mObjectId = vr::k_unTrackedDeviceIndexInvalid;
}

void OpenVRFakeTrackingReference::EnterStandby()
{
}

void * OpenVRFakeTrackingReference::GetComponent(const char * pchComponentNameAndVersion)
{
	Log(L"OpenVRFakeTrackingReference::GetComponent. Name=%hs", pchComponentNameAndVersion);

	return NULL;
}

void OpenVRFakeTrackingReference::PowerOff()
{
}

/** debug request from a client */

void OpenVRFakeTrackingReference::DebugRequest(const char * pchRequest, char * pchResponseBuffer, uint32_t unResponseBufferSize)
{
	if (unResponseBufferSize >= 1)
		pchResponseBuffer[0] = 0;
}

vr::DriverPose_t OpenVRFakeTrackingReference::GetPose()
{
	vr::DriverPose_t pose = { 0 };
	pose.poseIsValid = true;
	pose.result = vr::TrackingResult_Running_OK;
	pose.deviceIsConnected = true;

	pose.qWorldFromDriverRotation = HmdQuaternion_Init(1, 0, 0, 0);
	pose.qDriverFromHeadRotation = HmdQuaternion_Init(1, 0, 0, 0);
	pose.qRotation = HmdQuaternion_Init(1, 0, 0, 0);

	return pose;
}

std::string OpenVRFakeTrackingReference::GetSerialNumber() {
	return "ALVR-TrackingReference001";
}

void OpenVRFakeTrackingReference::OnPoseUpdated()
{
	vr::VRServerDriverHost()->TrackedDevicePoseUpdated(mObjectId, GetPose(), sizeof(vr::DriverPose_t));
}
