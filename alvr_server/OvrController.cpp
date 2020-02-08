#include "OvrController.h"



OvrController::OvrController(bool isLeftHand, int index)
	: m_isLeftHand(isLeftHand)
	, m_unObjectId(vr::k_unTrackedDeviceIndexInvalid)
	, m_index(index)
{
	memset(&m_pose, 0, sizeof(m_pose));
	m_pose.poseIsValid = true;
	m_pose.result = vr::TrackingResult_Running_OK;
	m_pose.deviceIsConnected = true;

	//controller is rotated and translated, prepare pose
	double rotation[3] = { 0.0, 0.0, 36 * M_PI / 180 };
	m_pose.qDriverFromHeadRotation = EulerAngleToQuaternion(rotation);

	vr::HmdVector3d_t offset;
	offset.v[0] =	0;
	offset.v[1] =	0.009;
	offset.v[2] = -0.053;

	vr::HmdVector3d_t offetRes = vrmath::quaternionRotateVector(m_pose.qDriverFromHeadRotation, offset, false);

	m_pose.vecDriverFromHeadTranslation[0] = offetRes.v[0];
	m_pose.vecDriverFromHeadTranslation[1] = offetRes.v[1];
	m_pose.vecDriverFromHeadTranslation[2] = offetRes.v[2];

	m_pose.qWorldFromDriverRotation = HmdQuaternion_Init(1, 0, 0, 0);

	m_pose.qRotation = HmdQuaternion_Init(1, 0, 0, 0);

	//init handles
	for (int i = 0; i < ALVR_INPUT_COUNT; i++) {
		m_handles[i] = vr::k_ulInvalidInputComponentHandle;
	}

}


bool OvrController::GetHand() {
	return m_isLeftHand;
}

//
// ITrackedDeviceServerDriver
//

vr::EVRInitError OvrController::Activate(vr::TrackedDeviceIndex_t unObjectId)
{
	Log(L"RemoteController::Activate. objectId=%d", unObjectId);

	m_unObjectId = unObjectId;
	m_ulPropertyContainer = vr::VRProperties()->TrackedDeviceToPropertyContainer(m_unObjectId);

	vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_TrackingSystemName_String, Settings::Instance().m_controllerTrackingSystemName.c_str());
	vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_ManufacturerName_String, Settings::Instance().m_controllerManufacturerName.c_str());
	vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_ModelNumber_String, m_isLeftHand ? (Settings::Instance().m_controllerModelNumber + " (Left Controller)").c_str() : (Settings::Instance().m_controllerModelNumber + " (Right Controller)").c_str());

	vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_RenderModelName_String, m_isLeftHand ? Settings::Instance().m_controllerRenderModelNameLeft.c_str() : Settings::Instance().m_controllerRenderModelNameRight.c_str());

	vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_SerialNumber_String, GetSerialNumber().c_str());
	vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_AttachedDeviceId_String, GetSerialNumber().c_str());
	vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_RegisteredDeviceType_String, m_isLeftHand ? (Settings::Instance().mControllerRegisteredDeviceType + "_Left").c_str() : (Settings::Instance().mControllerRegisteredDeviceType + "_Right").c_str() );

	uint64_t supportedButtons = 0xFFFFFFFFFFFFFFFFULL;
	vr::VRProperties()->SetUint64Property(m_ulPropertyContainer, vr::Prop_SupportedButtons_Uint64, supportedButtons);

	vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_DeviceProvidesBatteryStatus_Bool, true);


	vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_Axis0Type_Int32, vr::k_eControllerAxis_Joystick);

	vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_ControllerRoleHint_Int32, m_isLeftHand ? vr::TrackedControllerRole_LeftHand : vr::TrackedControllerRole_RightHand);

	vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_ControllerType_String, Settings::Instance().m_controllerType.c_str());
	vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_InputProfilePath_String, Settings::Instance().m_controllerInputProfilePath.c_str());
	int i = 0;

	switch (Settings::Instance().m_controllerMode) {
	case 0:	//Oculus

	vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/system/click", &m_handles[ALVR_INPUT_SYSTEM_CLICK]);
	vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/application_menu/click", &m_handles[ALVR_INPUT_APPLICATION_MENU_CLICK]);
	vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/grip/click", &m_handles[ALVR_INPUT_GRIP_CLICK]);
	vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/grip/value", &m_handles[ALVR_INPUT_GRIP_VALUE], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedOneSided);
	vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/grip/touch", &m_handles[ALVR_INPUT_GRIP_TOUCH]);

	if (!m_isLeftHand) {
		// A,B for right hand.
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/a/click", &m_handles[ALVR_INPUT_A_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/a/touch", &m_handles[ALVR_INPUT_A_TOUCH]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/b/click", &m_handles[ALVR_INPUT_B_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/b/touch", &m_handles[ALVR_INPUT_B_TOUCH]);

		//vr::VRDriverInput()->CreateSkeletonComponent(m_ulPropertyContainer, "/input/skeleton/right", "/skeleton/hand/right", "/pose/raw", nullptr, SKELTON_BONE_COUNT, &m_compSkeleton);
	}
	else {
		// X,Y for left hand.
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/x/click", &m_handles[ALVR_INPUT_X_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/x/touch", &m_handles[ALVR_INPUT_X_TOUCH]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/y/click", &m_handles[ALVR_INPUT_Y_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/y/touch", &m_handles[ALVR_INPUT_Y_TOUCH]);

		//vr::VRDriverInput()->CreateSkeletonComponent(m_ulPropertyContainer, "/input/skeleton/left", "/skeleton/hand/left", "/pose/raw", nullptr, SKELTON_BONE_COUNT, &m_compSkeleton);
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

	vr::VRDriverInput()->CreateHapticComponent(m_ulPropertyContainer, "/output/haptic", &m_compHaptic);
	break;

	case 1:	//Index
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/system/click", &m_handles[ALVR_INPUT_SYSTEM_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/a/click", &m_handles[ALVR_INPUT_A_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/a/touch", &m_handles[ALVR_INPUT_A_TOUCH]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/b/click", &m_handles[ALVR_INPUT_B_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/b/touch", &m_handles[ALVR_INPUT_B_TOUCH]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/trigger/click", &m_handles[ALVR_INPUT_TRIGGER_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/trigger/touch", &m_handles[ALVR_INPUT_TRIGGER_TOUCH]);
		vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/trigger/value", &m_handles[ALVR_INPUT_TRIGGER_VALUE], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedOneSided);
		vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/trackpad/x", &m_handles[ALVR_INPUT_TRACKPAD_X], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);
		vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/trackpad/y", &m_handles[ALVR_INPUT_TRACKPAD_Y], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);
		vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/trackpad/force", &m_handles[ALVR_INPUT_TRACKPAD_FORCE], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedOneSided);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/trackpad/touch", &m_handles[ALVR_INPUT_TRACKPAD_TOUCH]);
		vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/grip/force", &m_handles[ALVR_INPUT_GRIP_FORCE], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedOneSided);
		vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/grip/value", &m_handles[ALVR_INPUT_GRIP_VALUE], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedOneSided);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/grip/touch", &m_handles[ALVR_INPUT_GRIP_TOUCH]);
		vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/thumbstick/x", &m_handles[ALVR_INPUT_JOYSTICK_X], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);
		vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/thumbstick/y", &m_handles[ALVR_INPUT_JOYSTICK_Y], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/thumbstick/click", &m_handles[ALVR_INPUT_JOYSTICK_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/thumbstick/touch", &m_handles[ALVR_INPUT_JOYSTICK_TOUCH]);
		vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/finger/index", &m_handles[ALVR_INPUT_FINGER_INDEX], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedOneSided);
		vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/finger/middle", &m_handles[ALVR_INPUT_FINGER_MIDDLE], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedOneSided);
		vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/finger/ring", &m_handles[ALVR_INPUT_FINGER_RING], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedOneSided);
		vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/finger/pinky", &m_handles[ALVR_INPUT_FINGER_PINKY], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedOneSided);
		if (m_isLeftHand) {
			vr::VRDriverInput()->CreateSkeletonComponent(m_ulPropertyContainer, "/input/skeleton/left", "/skeleton/hand/left", "/pose/raw", vr::EVRSkeletalTrackingLevel::VRSkeletalTracking_Partial, nullptr, 0U, &m_compSkeleton);
		}
		else {
			vr::VRDriverInput()->CreateSkeletonComponent(m_ulPropertyContainer, "/input/skeleton/right", "/skeleton/hand/right", "/pose/raw", vr::EVRSkeletalTrackingLevel::VRSkeletalTracking_Partial, nullptr, 0U, &m_compSkeleton);
		}
		vr::VRDriverInput()->CreateHapticComponent(m_ulPropertyContainer, "/output/haptic", &m_compHaptic);
		break;
	}

	return vr::VRInitError_None;
}

void OvrController::Deactivate()
{
	Log(L"RemoteController::Deactivate");
	m_unObjectId = vr::k_unTrackedDeviceIndexInvalid;
}

void OvrController::EnterStandby()
{
}

void *OvrController::GetComponent(const char *pchComponentNameAndVersion)
{
	Log(L"RemoteController::GetComponent. Name=%hs", pchComponentNameAndVersion);

	return NULL;
}

 void PowerOff()
{
}

/** debug request from a client */
 void OvrController::DebugRequest(const char *pchRequest, char *pchResponseBuffer, uint32_t unResponseBufferSize)
{
	if (unResponseBufferSize >= 1)
		pchResponseBuffer[0] = 0;
}

 vr::DriverPose_t OvrController::GetPose()
{

	 Log(L"Controller%d getPose %lf %lf %lf", m_index, m_pose.vecPosition[0], m_pose.vecPosition[1], m_pose.vecPosition[2]);

	return m_pose;
}

 int OvrController::getControllerIndex() {
	 return m_index;
 }

 vr::VRInputComponentHandle_t OvrController::getHapticComponent() {
	return m_compHaptic;
}

vr::HmdQuaternion_t QuatMultiply(const vr::HmdQuaternion_t *q1, const vr::HmdQuaternion_t *q2)
{
	vr::HmdQuaternion_t result;
	result.x = q1->w*q2->x + q1->x*q2->w + q1->y*q2->z - q1->z*q2->y;
	result.y = q1->w*q2->y - q1->x*q2->z + q1->y*q2->w + q1->z*q2->x;
	result.z = q1->w*q2->z + q1->x*q2->y - q1->y*q2->x + q1->z*q2->w;
	result.w = q1->w*q2->w - q1->x*q2->x - q1->y*q2->y - q1->z*q2->z;
	return result;
}
vr::HmdQuaternionf_t QuatMultiply(const vr::HmdQuaternion_t* q1, const vr::HmdQuaternionf_t* q2)
{
	vr::HmdQuaternionf_t result;
	result.x = q1->w * q2->x + q1->x * q2->w + q1->y * q2->z - q1->z * q2->y;
	result.y = q1->w * q2->y - q1->x * q2->z + q1->y * q2->w + q1->z * q2->x;
	result.z = q1->w * q2->z + q1->x * q2->y - q1->y * q2->x + q1->z * q2->w;
	result.w = q1->w * q2->w - q1->x * q2->x - q1->y * q2->y - q1->z * q2->z;
	return result;
}
vr::HmdQuaternionf_t QuatMultiply(const vr::HmdQuaternionf_t* q1, const vr::HmdQuaternion_t* q2)
{
	vr::HmdQuaternionf_t result;
	result.x = q1->w * q2->x + q1->x * q2->w + q1->y * q2->z - q1->z * q2->y;
	result.y = q1->w * q2->y - q1->x * q2->z + q1->y * q2->w + q1->z * q2->x;
	result.z = q1->w * q2->z + q1->x * q2->y - q1->y * q2->x + q1->z * q2->w;
	result.w = q1->w * q2->w - q1->x * q2->x - q1->y * q2->y - q1->z * q2->z;
	return result;
}
vr::HmdQuaternionf_t QuatMultiply(const vr::HmdQuaternionf_t* q1, const vr::HmdQuaternionf_t* q2)
{
	vr::HmdQuaternionf_t result;
	result.x = q1->w * q2->x + q1->x * q2->w + q1->y * q2->z - q1->z * q2->y;
	result.y = q1->w * q2->y - q1->x * q2->z + q1->y * q2->w + q1->z * q2->x;
	result.z = q1->w * q2->z + q1->x * q2->y - q1->y * q2->x + q1->z * q2->w;
	result.w = q1->w * q2->w - q1->x * q2->x - q1->y * q2->y - q1->z * q2->z;
	return result;
}

bool OvrController::onPoseUpdate(int controllerIndex, const TrackingInfo &info) {

	if (m_unObjectId == vr::k_unTrackedDeviceIndexInvalid) {
		return false;
	}
	
	if (info.controller[controllerIndex].flags & TrackingInfo::Controller::FLAG_CONTROLLER_OCULUS_HAND) {

		vr::HmdQuaternion_t rootBoneRot = HmdQuaternion_Init(
			info.controller[controllerIndex].boneRootOrientation.w,
			info.controller[controllerIndex].boneRootOrientation.x,
			info.controller[controllerIndex].boneRootOrientation.y,
			info.controller[controllerIndex].boneRootOrientation.z);
		vr::HmdQuaternion_t boneFixer = info.controller[controllerIndex].flags & TrackingInfo::Controller::FLAG_CONTROLLER_LEFTHAND ?
			HmdQuaternion_Init(-0.5, 0.5, 0.5, -0.5) :
			HmdQuaternion_Init(0.5, 0.5, 0.5, 0.5);
		m_pose.qRotation = QuatMultiply(&rootBoneRot, &boneFixer);
		if (info.controller[controllerIndex].flags & TrackingInfo::Controller::FLAG_CONTROLLER_LEFTHAND) {
			double bonePosFixer[3] = { 0.0,0.05,-0.05 };
			m_pose.vecPosition[0] = info.controller[controllerIndex].boneRootPosition.x + bonePosFixer[0];
			m_pose.vecPosition[1] = info.controller[controllerIndex].boneRootPosition.y + bonePosFixer[1];
			m_pose.vecPosition[2] = info.controller[controllerIndex].boneRootPosition.z + bonePosFixer[2];
		}
		else {
			double bonePosFixer[3] = { 0.0,0.05,-0.05 };
			m_pose.vecPosition[0] = info.controller[controllerIndex].boneRootPosition.x + bonePosFixer[0];
			m_pose.vecPosition[1] = info.controller[controllerIndex].boneRootPosition.y + bonePosFixer[1];
			m_pose.vecPosition[2] = info.controller[controllerIndex].boneRootPosition.z + bonePosFixer[2];
		}
	}
	else {

	m_pose.qRotation = HmdQuaternion_Init(info.controller[controllerIndex].orientation.w,
		info.controller[controllerIndex].orientation.x,
		info.controller[controllerIndex].orientation.y,
		info.controller[controllerIndex].orientation.z);   //controllerRotation;
		

	m_pose.vecPosition[0] = info.controller[controllerIndex].position.x;
	m_pose.vecPosition[1] = info.controller[controllerIndex].position.y;
	m_pose.vecPosition[2] = info.controller[controllerIndex].position.z;

	}

	m_pose.vecVelocity[0] = info.controller[controllerIndex].linearVelocity.x;
	m_pose.vecVelocity[1] = info.controller[controllerIndex].linearVelocity.y;
	m_pose.vecVelocity[2] = info.controller[controllerIndex].linearVelocity.z;
	//m_pose.vecAcceleration[0] = info.controller[controllerIndex].linearAcceleration.x;
	//m_pose.vecAcceleration[1] = info.controller[controllerIndex].linearAcceleration.y;
	//m_pose.vecAcceleration[2] = info.controller[controllerIndex].linearAcceleration.z;
	m_pose.vecAngularVelocity[0] = info.controller[controllerIndex].angularVelocity.x;
	m_pose.vecAngularVelocity[1] = info.controller[controllerIndex].angularVelocity.y;
	m_pose.vecAngularVelocity[2] = info.controller[controllerIndex].angularVelocity.z;
	//m_pose.vecAngularAcceleration[0] = info.controller[controllerIndex].angularAcceleration.x;
	//m_pose.vecAngularAcceleration[1] = info.controller[controllerIndex].angularAcceleration.y;
	//m_pose.vecAngularAcceleration[2] = info.controller[controllerIndex].angularAcceleration.z;
	
	
	
	//correct direction of velocities
	vr::HmdVector3d_t angVel;
	angVel.v[0] = m_pose.vecAngularVelocity[0];
	angVel.v[1] = m_pose.vecAngularVelocity[1];
	angVel.v[2] = m_pose.vecAngularVelocity[2];
	vr::HmdVector3d_t angVelRes = vrmath::quaternionRotateVector(m_pose.qRotation, angVel, true);
	m_pose.vecAngularVelocity[0] = angVelRes.v[0];
	m_pose.vecAngularVelocity[1] = angVelRes.v[1];
	m_pose.vecAngularVelocity[2] = angVelRes.v[2];
	



	/*
	vr::HmdVector3d_t vel;
	vel.v[0] = m_pose.vecVelocity[0];
	vel.v[1] = m_pose.vecVelocity[1];
	vel.v[2] = m_pose.vecVelocity[2];
	vr::HmdVector3d_t velRes = vrmath::quaternionRotateVector(m_pose.qRotation, vel, true);
	m_pose.vecVelocity[0] = velRes.v[0];
	m_pose.vecVelocity[1] = velRes.v[1];
	m_pose.vecVelocity[2] = velRes.v[2];
	*/
	

	Log(L"CONTROLLER %d %f,%f,%f - %f,%f,%f", m_index, m_pose.vecVelocity[0], m_pose.vecVelocity[1], m_pose.vecVelocity[2], m_pose.vecAngularVelocity[0], m_pose.vecAngularVelocity[1], m_pose.vecAngularVelocity[2]);
	
	

	/*
	double rotation[3] = { 0.0, 0.0, 36 * M_PI / 180 };
	m_pose.qDriverFromHeadRotation = EulerAngleToQuaternion(rotation);
	m_pose.vecDriverFromHeadTranslation[1] = 0.031153;
	m_pose.vecDriverFromHeadTranslation[2] = -0.042878;

	

	//double r[3] = { 0, -0.031153 ,0.042878 };
	double r[3] = { 0, 0 ,-0.053 };
	double v1[3] = { m_pose.vecVelocity[0], m_pose.vecVelocity[1], m_pose.vecVelocity[2] };
	double w[3] = { m_pose.vecAngularVelocity[0], m_pose.vecAngularVelocity[1], m_pose.vecAngularVelocity[2] };

	double tmp[3] = { 0, 0 ,0 };
	tmp[0] = (w[1] * r[2]) - (w[2] * r[1]);
	tmp[1] = (w[2] * r[0]) - (w[0] * r[2]);
	tmp[2] = (w[0] * r[1]) - (w[1] * r[0]);



	m_pose.vecVelocity[0] = m_pose.vecVelocity[0] + tmp[0];
	m_pose.vecVelocity[1] = m_pose.vecVelocity[1] + tmp[1];
	m_pose.vecVelocity[2] = m_pose.vecVelocity[2] + tmp[2];
	*/
	

	m_pose.poseTimeOffset = Settings::Instance().m_controllerPoseOffset;

	   

	auto& c = info.controller[controllerIndex];
	Log(L"Controller%d %d %lu: %08llX %08X %f:%f", m_index,controllerIndex, (unsigned long)m_unObjectId, c.buttons, c.flags, c.trackpadPosition.x, c.trackpadPosition.y);

	if (c.flags & TrackingInfo::Controller::FLAG_CONTROLLER_OCULUS_HAND) {

		vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_SYSTEM_CLICK], (c.inputStateStatus & alvrInputStateHandStatus_RingPinching) != 0, 0.0);
		vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_GRIP_TOUCH], false, 0.0);
		vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_GRIP_VALUE], 0.0, 0.0);
		vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRACKPAD_X], 0, 0.0);
		vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRACKPAD_Y], 0, 0.0);
		vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRACKPAD_TOUCH], false, 0.0);
		vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_JOYSTICK_X], 0, 0.0);
		vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_JOYSTICK_Y], 0, 0.0);
		vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_JOYSTICK_CLICK], false, 0.0);
		vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_JOYSTICK_TOUCH], false, 0.0);
		vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_A_CLICK], false, 0.0);
		vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_A_TOUCH], false, 0.0);
		vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_B_CLICK], (c.inputStateStatus & alvrInputStateHandStatus_MiddlePinching) != 0, 0.0);
		vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_B_TOUCH], (c.inputStateStatus & alvrInputStateHandStatus_MiddlePinching) != 0, 0.0);
		vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRIGGER_CLICK], (c.inputStateStatus & alvrInputStateHandStatus_IndexPinching) != 0, 0.0);
		vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRIGGER_TOUCH], (c.inputStateStatus& alvrInputStateHandStatus_IndexPinching) != 0, 0.0);
		vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRIGGER_VALUE], (c.inputStateStatus & alvrInputStateHandStatus_IndexPinching) ? 1:0, 0.0);
		//Hand
		const vr::VRBoneTransform_t handRestPose = { { 0, 0, 0, 1 }, { 1, 0, 0, 0 } };
		for (size_t i = 0U; i < HSB_Count; i++) m_boneTransform[i] = handRestPose;
#define COPY4(a,b) do{b.w=a.w;b.x=a.x;b.y=a.y;b.z=a.z;}while(0)
#define COPY4M(a,b,c) do{b.w=a.w*c;b.x=a.x*c;b.y=a.y*c;b.z=a.z*c;}while(0)
#define ADD4(a,b) do{b.w+=a.w;b.x+=a.x;b.y+=a.y;b.z+=a.z;}while(0)
#define COPY3(a,b) do{b.v[0]=a.x;b.v[1]=a.y;b.v[2]=a.z;}while(0)
#define COPY3M(a,b,c) do{b.v[0]=a.x*c;b.v[1]=a.y*c;b.v[2]=a.z*c;}while(0)
#define SIZE4(b) (sqrt(b.v[0]*b.v[0]+b.v[1]*b.v[1]+b.v[2]*b.v[2]))
#define APPSIZE4(a,b,c) do{a.v[0]*=b/c;a.v[1]*=b/c;a.v[2]*=b/c;}while(0)

		vr::HmdQuaternion_t inv = vrmath::quaternionConjugate(m_pose.qRotation);
		COPY4(c.boneRootOrientation, m_boneTransform[HSB_Wrist].orientation);
		vr::HmdQuaternionf_t hoge = QuatMultiply(&inv, &m_boneTransform[HSB_Wrist].orientation);
		COPY4(c.boneRotations[alvrHandBone_WristRoot], m_boneTransform[HSB_Wrist].orientation);
		m_boneTransform[HSB_Wrist].orientation = QuatMultiply(&hoge, &m_boneTransform[HSB_Wrist].orientation);

		//COPY4(c.boneRotations[alvrHandBone_WristRoot], m_boneTransform[HSB_Wrist].orientation);
		COPY4(c.boneRotations[alvrHandBone_Thumb0], m_boneTransform[HSB_Thumb0].orientation);
		COPY4(c.boneRotations[alvrHandBone_Thumb1], m_boneTransform[HSB_Thumb1].orientation);
		COPY4(c.boneRotations[alvrHandBone_Thumb2], m_boneTransform[HSB_Thumb2].orientation);
		COPY4(c.boneRotations[alvrHandBone_Thumb3], m_boneTransform[HSB_Thumb3].orientation);
		COPY4(c.boneRotations[alvrHandBone_Index1], m_boneTransform[HSB_IndexFinger1].orientation);
		COPY4(c.boneRotations[alvrHandBone_Index2], m_boneTransform[HSB_IndexFinger2].orientation);
		COPY4(c.boneRotations[alvrHandBone_Index3], m_boneTransform[HSB_IndexFinger3].orientation);
		COPY4(c.boneRotations[alvrHandBone_Middle1], m_boneTransform[HSB_MiddleFinger1].orientation);
		COPY4(c.boneRotations[alvrHandBone_Middle2], m_boneTransform[HSB_MiddleFinger2].orientation);
		COPY4(c.boneRotations[alvrHandBone_Middle3], m_boneTransform[HSB_MiddleFinger3].orientation);
		COPY4(c.boneRotations[alvrHandBone_Ring1], m_boneTransform[HSB_RingFinger1].orientation);
		COPY4(c.boneRotations[alvrHandBone_Ring2], m_boneTransform[HSB_RingFinger2].orientation);
		COPY4(c.boneRotations[alvrHandBone_Ring3], m_boneTransform[HSB_RingFinger3].orientation);
		COPY4(c.boneRotations[alvrHandBone_Pinky0], m_boneTransform[HSB_PinkyFinger0].orientation);
		COPY4(c.boneRotations[alvrHandBone_Pinky1], m_boneTransform[HSB_PinkyFinger1].orientation);
		COPY4(c.boneRotations[alvrHandBone_Pinky2], m_boneTransform[HSB_PinkyFinger2].orientation);
		COPY4(c.boneRotations[alvrHandBone_Pinky3], m_boneTransform[HSB_PinkyFinger3].orientation);
		
		//COPY3(c.boneRootPosition, m_boneTransform[HSB_Root].position);
		COPY3(c.bonePositionsBase[alvrHandBone_WristRoot], m_boneTransform[HSB_Wrist].position);
		COPY3(c.bonePositionsBase[alvrHandBone_Thumb0], m_boneTransform[HSB_Thumb0].position);
		COPY3(c.bonePositionsBase[alvrHandBone_Thumb1], m_boneTransform[HSB_Thumb1].position);
		COPY3(c.bonePositionsBase[alvrHandBone_Thumb2], m_boneTransform[HSB_Thumb2].position);
		COPY3(c.bonePositionsBase[alvrHandBone_Thumb3], m_boneTransform[HSB_Thumb3].position);
		COPY3(c.bonePositionsBase[alvrHandBone_Index1], m_boneTransform[HSB_IndexFinger1].position);
		COPY3(c.bonePositionsBase[alvrHandBone_Index2], m_boneTransform[HSB_IndexFinger2].position);
		COPY3(c.bonePositionsBase[alvrHandBone_Index3], m_boneTransform[HSB_IndexFinger3].position);
		COPY3(c.bonePositionsBase[alvrHandBone_Middle1], m_boneTransform[HSB_MiddleFinger1].position);
		COPY3(c.bonePositionsBase[alvrHandBone_Middle2], m_boneTransform[HSB_MiddleFinger2].position);
		COPY3(c.bonePositionsBase[alvrHandBone_Middle3], m_boneTransform[HSB_MiddleFinger3].position);
		COPY3(c.bonePositionsBase[alvrHandBone_Ring1], m_boneTransform[HSB_RingFinger1].position);
		COPY3(c.bonePositionsBase[alvrHandBone_Ring2], m_boneTransform[HSB_RingFinger2].position);
		COPY3(c.bonePositionsBase[alvrHandBone_Ring3], m_boneTransform[HSB_RingFinger3].position);
		COPY3(c.bonePositionsBase[alvrHandBone_Pinky0], m_boneTransform[HSB_PinkyFinger0].position);
		COPY3(c.bonePositionsBase[alvrHandBone_Pinky1], m_boneTransform[HSB_PinkyFinger1].position);
		COPY3(c.bonePositionsBase[alvrHandBone_Pinky2], m_boneTransform[HSB_PinkyFinger2].position);
		COPY3(c.bonePositionsBase[alvrHandBone_Pinky3], m_boneTransform[HSB_PinkyFinger3].position);

		vr::VRDriverInput()->UpdateSkeletonComponent(m_compSkeleton, vr::VRSkeletalMotionRange_WithController, m_boneTransform, HSB_Count);
		vr::VRDriverInput()->UpdateSkeletonComponent(m_compSkeleton, vr::VRSkeletalMotionRange_WithoutController, m_boneTransform, HSB_Count);

		vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_FINGER_INDEX], c.boneRotations[alvrHandBone_Index1].z + c.boneRotations[alvrHandBone_Index2].z + c.boneRotations[alvrHandBone_Index3].z, 0.0);
		vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_FINGER_MIDDLE], c.boneRotations[alvrHandBone_Middle1].z + c.boneRotations[alvrHandBone_Middle2].z + c.boneRotations[alvrHandBone_Middle3].z, 0.0);
		vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_FINGER_RING], c.boneRotations[alvrHandBone_Ring1].z + c.boneRotations[alvrHandBone_Ring2].z + c.boneRotations[alvrHandBone_Ring3].z, 0.0);
		vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_FINGER_PINKY], c.boneRotations[alvrHandBone_Pinky1].z + c.boneRotations[alvrHandBone_Pinky2].z + c.boneRotations[alvrHandBone_Pinky3].z, 0.0);

		vr::VRServerDriverHost()->TrackedDevicePoseUpdated(m_unObjectId, m_pose, sizeof(vr::DriverPose_t));
	}
	else {

	vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_SYSTEM_CLICK], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_SYSTEM_CLICK)) != 0, 0.0);
	vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_APPLICATION_MENU_CLICK], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_APPLICATION_MENU_CLICK)) != 0, 0.0);
	vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_GRIP_CLICK], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_GRIP_CLICK)) != 0, 0.0);
	vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_GRIP_VALUE], c.gripValue, 0.0);
	vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_GRIP_TOUCH], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_GRIP_TOUCH)) != 0, 0.0);


	if (!m_isLeftHand) {
		// A,B for right hand.
		vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_A_CLICK], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_A_CLICK)) != 0, 0.0);
		vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_A_TOUCH], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_A_TOUCH)) != 0, 0.0);
		vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_B_CLICK], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_B_CLICK)) != 0, 0.0);
		vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_B_TOUCH], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_B_TOUCH)) != 0, 0.0);

	}
	else {
		// X,Y for left hand.
		vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_X_CLICK], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_X_CLICK)) != 0, 0.0);
		vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_X_TOUCH], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_X_TOUCH)) != 0, 0.0);
		vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_Y_CLICK], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_Y_CLICK)) != 0, 0.0);
		vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_Y_TOUCH], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_Y_TOUCH)) != 0, 0.0);
	}

	vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_JOYSTICK_CLICK], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_JOYSTICK_CLICK)) != 0, 0.0);
	vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_JOYSTICK_X], c.trackpadPosition.x, 0.0);
	vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_JOYSTICK_Y], c.trackpadPosition.y, 0.0);
	vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_JOYSTICK_TOUCH], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_JOYSTICK_TOUCH)) != 0, 0.0);


	vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_BACK_CLICK], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_BACK_CLICK)) != 0, 0.0);
	vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_GUIDE_CLICK], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_GUIDE_CLICK)) != 0, 0.0);
	vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_START_CLICK], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_START_CLICK)) != 0, 0.0);

	vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRIGGER_CLICK], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_CLICK)) != 0, 0.0);
	vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRIGGER_VALUE], c.triggerValue, 0.0);
	vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRIGGER_TOUCH], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_TOUCH)) != 0, 0.0);


	

	// Battery
	vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_DeviceBatteryPercentage_Float, c.batteryPercentRemaining / 100.0f);
	
	vr::VRServerDriverHost()->TrackedDevicePoseUpdated(m_unObjectId, m_pose, sizeof(vr::DriverPose_t));

	}

	return false;
}

std::string OvrController::GetSerialNumber() {
	char str[100];
	snprintf(str, sizeof(str), "_%s", m_index == 0 ? "Left" : "Right");
	return Settings::Instance().m_controllerSerialNumber + str;
}