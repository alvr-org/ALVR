#pragma once
#include <openvr_driver.h>
#include <string>
#include "Logger.h"
#include "Listener.h"
#include "packet_types.h"
#include "FreePIE.h"

class OpenVRController : public vr::ITrackedDeviceServerDriver
{
public:
	OpenVRController(bool hand, int index);

	virtual ~OpenVRController();

	bool GetHand();

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

	bool IsMyHapticComponent(uint64_t handle);
	bool ReportControllerState(int controllerIndex, const TrackingInfo &info
		, const vr::HmdQuaternion_t controllerRotation, const TrackingVector3 &controllerPosition
		, bool enableControllerButton, const FreePIE::FreePIEFileMapping &freePIEData);

	std::string GetSerialNumber();

private:
	static const int SKELTON_BONE_COUNT = 31;

	vr::TrackedDeviceIndex_t mObjectId;
	vr::PropertyContainerHandle_t mPropertyContainer;

	uint64_t mPreviousButtons;
	uint32_t mPreviousFlags;

	bool mHand;
	int mIndex;
	bool mIsTouch;

	vr::VRInputComponentHandle_t mHandles[ALVR_INPUT_COUNT];
	vr::VRInputComponentHandle_t mHapticHandle;
	vr::VRInputComponentHandle_t mSkeletonHandle;

	vr::DriverPose_t mPose;
};
