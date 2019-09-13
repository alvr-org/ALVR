#pragma once
#pragma once
#include <openvr_driver.h>
#include <string>
#include "Logger.h"
#include "ClientConnection.h"
#include "packet_types.h"
//#include "FreePIE.h"
#include <openvr_math.h>

class OvrController : public vr::ITrackedDeviceServerDriver
{
public:
	OvrController(bool isLeftHand, int index);

	virtual ~OvrController() {};

	bool GetHand();

	//
	// ITrackedDeviceServerDriver
	//

	virtual vr::EVRInitError Activate(vr::TrackedDeviceIndex_t unObjectId);

	virtual void Deactivate();

	virtual void EnterStandby();

	void *GetComponent(const char *pchComponentNameAndVersion);

	virtual void PowerOff() {};

	/** debug request from a client */
	virtual void DebugRequest(const char *pchRequest, char *pchResponseBuffer, uint32_t unResponseBufferSize);

	virtual vr::DriverPose_t GetPose();

	vr::VRInputComponentHandle_t getHapticComponent();

	bool onPoseUpdate(int controllerIndex, const TrackingInfo &info);
	std::string GetSerialNumber();

	int getControllerIndex();



private:
	static const int SKELTON_BONE_COUNT = 31;

	vr::TrackedDeviceIndex_t m_unObjectId;
	vr::PropertyContainerHandle_t m_ulPropertyContainer;


	bool m_isLeftHand;
	int m_index;

	vr::VRInputComponentHandle_t m_handles[ALVR_INPUT_COUNT];
	vr::VRInputComponentHandle_t m_compHaptic;
	vr::VRInputComponentHandle_t m_compSkeleton;

	vr::DriverPose_t m_pose;
};
