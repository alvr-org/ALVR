#pragma once

#include <openvr_driver.h>
#include "Logger.h"

class OpenVRFakeTrackingReference : public vr::ITrackedDeviceServerDriver
{
public:
	//
	// ITrackedDeviceServerDriver
	//

	virtual vr::EVRInitError Activate(vr::TrackedDeviceIndex_t unObjectId);
	virtual void Deactivate();
	virtual void EnterStandby();
	void *GetComponent(const char *pchComponentNameAndVersion);
	virtual void PowerOff();
	virtual void DebugRequest(const char *pchRequest, char *pchResponseBuffer, uint32_t unResponseBufferSize);
	virtual vr::DriverPose_t GetPose();
	std::string GetSerialNumber();
	void OnPoseUpdated();

private:
	vr::TrackedDeviceIndex_t mObjectId;
	vr::PropertyContainerHandle_t mPropertyContainer;
};