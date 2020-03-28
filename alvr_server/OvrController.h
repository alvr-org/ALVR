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

	void GetBoneTransformInterpolation(bool withController, bool isLeftHand, float animationProgress, const TrackingInfo::Controller &currentPoseInfo, const TrackingInfo::Controller &lastPoseInfo, vr::VRBoneTransform_t outBoneTransform[]);

	void GetBoneTransform(bool withController, bool isLeftHand, const TrackingInfo::Controller &c, vr::VRBoneTransform_t outBoneTransform[]);

private:
	static const int SKELETON_BONE_COUNT = 31;
	static const int ANIMATION_FRAMES_N = 15;

	vr::TrackedDeviceIndex_t m_unObjectId;
	vr::PropertyContainerHandle_t m_ulPropertyContainer;


	bool m_isLeftHand;
	int m_index;

	vr::VRInputComponentHandle_t m_handles[ALVR_INPUT_COUNT];
	vr::VRInputComponentHandle_t m_compHaptic;
	vr::VRInputComponentHandle_t m_compSkeleton = vr::k_ulInvalidInputComponentHandle;
	enum HandSkeletonBone : size_t
	{
		HSB_Root = 0,
		HSB_Wrist,
		HSB_Thumb0,
		HSB_Thumb1,
		HSB_Thumb2,
		HSB_Thumb3,
		HSB_IndexFinger0,
		HSB_IndexFinger1,
		HSB_IndexFinger2,
		HSB_IndexFinger3,
		HSB_IndexFinger4,
		HSB_MiddleFinger0,
		HSB_MiddleFinger1,
		HSB_MiddleFinger2,
		HSB_MiddleFinger3,
		HSB_MiddleFinger4,
		HSB_RingFinger0,
		HSB_RingFinger1,
		HSB_RingFinger2,
		HSB_RingFinger3,
		HSB_RingFinger4,
		HSB_PinkyFinger0,
		HSB_PinkyFinger1,
		HSB_PinkyFinger2,
		HSB_PinkyFinger3,
		HSB_PinkyFinger4,
		HSB_Aux_Thumb, // Not used yet
		HSB_Aux_IndexFinger, // Not used yet
		HSB_Aux_MiddleFinger, // Not used yet
		HSB_Aux_RingFinger, // Not used yet
		HSB_Aux_PinkyFinger, // Not used yet
		HSB_Count
	};
	vr::VRBoneTransform_t m_boneTransform[HSB_Count];

	vr::DriverPose_t m_pose;

	float m_leftControllerAnimationProgress = 0;
	float m_rightControllerAnimationProgress = 0;
	TrackingInfo::Controller m_lastLeftControllerPoseInfo = {};
	TrackingInfo::Controller m_lastRightControllerPoseInfo = {};
};
