#include "OvrController.h"


OvrController::OvrController(bool isLeftHand, int index)
	: m_isLeftHand(isLeftHand)
	, m_unObjectId(vr::k_unTrackedDeviceIndexInvalid)
	, m_index(index)
{
	double rightHandSignFlip = isLeftHand ? 1. : -1.;

	memset(&m_pose, 0, sizeof(m_pose));
	m_pose.poseIsValid = true;
	m_pose.result = vr::TrackingResult_Running_OK;
	m_pose.deviceIsConnected = true;

	//controller is rotated and translated, prepare pose
	double rotation[3] = {
		Settings::Instance().m_leftControllerRotationOffset[1] * DEG_TO_RAD * rightHandSignFlip,
		Settings::Instance().m_leftControllerRotationOffset[2] * DEG_TO_RAD * rightHandSignFlip,
		Settings::Instance().m_leftControllerRotationOffset[0] * DEG_TO_RAD,
	};
	m_pose.qDriverFromHeadRotation = EulerAngleToQuaternion(rotation);

	vr::HmdVector3d_t offset;
	offset.v[0] = Settings::Instance().m_leftControllerPositionOffset[0] * rightHandSignFlip;
	offset.v[1] = Settings::Instance().m_leftControllerPositionOffset[1];
	offset.v[2] = Settings::Instance().m_leftControllerPositionOffset[2];

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
	LogDriver("RemoteController::Activate. objectId=%d", unObjectId);

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
	case 1:	//Oculus no pinch

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

		vr::VRDriverInput()->CreateSkeletonComponent(m_ulPropertyContainer, "/input/skeleton/right", "/skeleton/hand/right", "/pose/raw", vr::EVRSkeletalTrackingLevel::VRSkeletalTracking_Partial, nullptr, SKELETON_BONE_COUNT, &m_compSkeleton);
	

		//icons
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceOff_String,"{oculus}/icons/rifts_right_controller_off.png" );
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceSearching_String,"{oculus}/icons/rifts_right_controller_searching.gif" );
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceSearchingAlert_String,"{oculus}/icons/rifts_right_controller_searching_alert.gif" );
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceReady_String,"{oculus}/icons/rifts_right_controller_ready.png" );
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceReadyAlert_String,"{oculus}/icons/rifts_right_controller_ready_alert.png" );
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceAlertLow_String,"{oculus}/icons/rifts_right_controller_ready_low.png" );
			   	
	
	}
	else {
		// X,Y for left hand.
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/x/click", &m_handles[ALVR_INPUT_X_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/x/touch", &m_handles[ALVR_INPUT_X_TOUCH]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/y/click", &m_handles[ALVR_INPUT_Y_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/y/touch", &m_handles[ALVR_INPUT_Y_TOUCH]);

		vr::VRDriverInput()->CreateSkeletonComponent(m_ulPropertyContainer, "/input/skeleton/left", "/skeleton/hand/left", "/pose/raw", vr::EVRSkeletalTrackingLevel::VRSkeletalTracking_Partial, nullptr, SKELETON_BONE_COUNT, &m_compSkeleton);
	
		//icons
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceOff_String, "{oculus}/icons/rifts_left_controller_off.png");
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceSearching_String, "{oculus}/icons/rifts_left_controller_searching.gif");
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceSearchingAlert_String, "{oculus}/icons/rifts_left_controller_searching_alert.gif");
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceReady_String, "{oculus}/icons/rifts_left_controller_ready.png");
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceReadyAlert_String, "{oculus}/icons/rifts_left_controller_ready_alert.png");
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceAlertLow_String, "{oculus}/icons/rifts_left_controller_ready_low.png");	
	
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

	case 2:	//Index
	case 3:	//Index no pinch
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
	LogDriver("RemoteController::Deactivate");
	m_unObjectId = vr::k_unTrackedDeviceIndexInvalid;
}

void OvrController::EnterStandby()
{
}

void *OvrController::GetComponent(const char *pchComponentNameAndVersion)
{
	LogDriver("RemoteController::GetComponent. Name=%hs", pchComponentNameAndVersion);

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

	 Log("Controller%d getPose %lf %lf %lf", m_index, m_pose.vecPosition[0], m_pose.vecPosition[1], m_pose.vecPosition[2]);

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
	

	Log("CONTROLLER %d %f,%f,%f - %f,%f,%f", m_index, m_pose.vecVelocity[0], m_pose.vecVelocity[1], m_pose.vecVelocity[2], m_pose.vecAngularVelocity[0], m_pose.vecAngularVelocity[1], m_pose.vecAngularVelocity[2]);
	
	

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
	Log("Controller%d %d %lu: %08llX %08X %f:%f", m_index,controllerIndex, (unsigned long)m_unObjectId, c.buttons, c.flags, c.trackpadPosition.x, c.trackpadPosition.y);

	if (c.flags & TrackingInfo::Controller::FLAG_CONTROLLER_OCULUS_HAND) {

		float rotThumb = (c.boneRotations[alvrHandBone_Thumb0].z + c.boneRotations[alvrHandBone_Thumb0].y + c.boneRotations[alvrHandBone_Thumb1].z + c.boneRotations[alvrHandBone_Thumb1].y + c.boneRotations[alvrHandBone_Thumb2].z + c.boneRotations[alvrHandBone_Thumb2].y + c.boneRotations[alvrHandBone_Thumb3].z + c.boneRotations[alvrHandBone_Thumb3].y) * 0.67f;
		float rotIndex = (c.boneRotations[alvrHandBone_Index1].z + c.boneRotations[alvrHandBone_Index2].z + c.boneRotations[alvrHandBone_Index3].z) * 0.67f;
		float rotMiddle = (c.boneRotations[alvrHandBone_Middle1].z + c.boneRotations[alvrHandBone_Middle2].z + c.boneRotations[alvrHandBone_Middle3].z) * 0.67f;
		float rotRing = (c.boneRotations[alvrHandBone_Ring1].z + c.boneRotations[alvrHandBone_Ring2].z + c.boneRotations[alvrHandBone_Ring3].z) * 0.67f;
		float rotPinky = (c.boneRotations[alvrHandBone_Pinky1].z + c.boneRotations[alvrHandBone_Pinky2].z + c.boneRotations[alvrHandBone_Pinky3].z) * 0.67f;
		float grip = std::min({ rotMiddle,rotRing,rotPinky }) * 4.0f - 3.0f;

		switch(Settings::Instance().m_controllerMode){
		case 0:
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_SYSTEM_CLICK], (c.inputStateStatus & alvrInputStateHandStatus_RingPinching) != 0, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_APPLICATION_MENU_CLICK], false, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_GRIP_CLICK], grip > 0.9f, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_GRIP_VALUE], grip, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_GRIP_TOUCH], grip > 0.7f, 0.0);
			if (!m_isLeftHand) {
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_A_CLICK], (c.inputStateStatus & alvrInputStateHandStatus_MiddlePinching) != 0, 0.0);
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_A_TOUCH], (c.inputStateStatus & alvrInputStateHandStatus_MiddlePinching) != 0, 0.0);
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_B_CLICK], (c.inputStateStatus & alvrInputStateHandStatus_IndexPinching) != 0, 0.0);
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_B_TOUCH], (c.inputStateStatus & alvrInputStateHandStatus_IndexPinching) != 0, 0.0);
			}
			else {
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_X_CLICK], (c.inputStateStatus & alvrInputStateHandStatus_MiddlePinching) != 0, 0.0);
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_X_TOUCH], (c.inputStateStatus & alvrInputStateHandStatus_MiddlePinching) != 0, 0.0);
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_Y_CLICK], (c.inputStateStatus & alvrInputStateHandStatus_IndexPinching) != 0, 0.0);
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_Y_TOUCH], (c.inputStateStatus & alvrInputStateHandStatus_IndexPinching) != 0, 0.0);
			}
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_JOYSTICK_CLICK], rotThumb > 0.9f, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_JOYSTICK_X], 0.0, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_JOYSTICK_Y], 0.0, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_JOYSTICK_TOUCH], rotThumb > 0.7f, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_BACK_CLICK], false, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_GUIDE_CLICK], false, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_START_CLICK], false, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRIGGER_CLICK], rotIndex > 0.9f, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRIGGER_VALUE], rotIndex, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRIGGER_TOUCH], rotIndex > 0.7f, 0.0);
			break;
		case 1:
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_SYSTEM_CLICK], false, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_APPLICATION_MENU_CLICK], false, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_GRIP_CLICK], grip > 0.9f, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_GRIP_VALUE], grip, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_GRIP_TOUCH], grip > 0.7f, 0.0);
			if (!m_isLeftHand) {
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_A_CLICK], false, 0.0);
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_A_TOUCH], false, 0.0);
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_B_CLICK], false, 0.0);
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_B_TOUCH], false, 0.0);
			}
			else {
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_X_CLICK], false, 0.0);
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_X_TOUCH], false, 0.0);
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_Y_CLICK], false, 0.0);
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_Y_TOUCH], false, 0.0);
			}
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_JOYSTICK_CLICK], false, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_JOYSTICK_X], 0.0, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_JOYSTICK_Y], 0.0, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_JOYSTICK_TOUCH], rotThumb > 0.7f, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_BACK_CLICK], false, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_GUIDE_CLICK], false, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_START_CLICK], false, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRIGGER_CLICK], rotIndex > 0.9f, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRIGGER_VALUE], rotIndex, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRIGGER_TOUCH], rotIndex > 0.7f, 0.0);
			break;
		case 2:
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_SYSTEM_CLICK], (c.inputStateStatus & alvrInputStateHandStatus_RingPinching) != 0, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_GRIP_TOUCH], grip > 0.9f, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_GRIP_VALUE], grip, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRACKPAD_X], 0, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRACKPAD_Y], 0, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRACKPAD_TOUCH], false, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_JOYSTICK_X], 0, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_JOYSTICK_Y], 0, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_JOYSTICK_CLICK], rotThumb > 0.9f, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_JOYSTICK_TOUCH], rotThumb > 0.7f, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_A_CLICK], (c.inputStateStatus& alvrInputStateHandStatus_MiddlePinching) != 0, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_A_TOUCH], (c.inputStateStatus& alvrInputStateHandStatus_MiddlePinching) != 0, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_B_CLICK], (c.inputStateStatus& alvrInputStateHandStatus_IndexPinching) != 0, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_B_TOUCH], (c.inputStateStatus& alvrInputStateHandStatus_IndexPinching) != 0, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRIGGER_CLICK], rotIndex > 0.9f, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRIGGER_TOUCH], rotIndex > 0.7f, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRIGGER_VALUE], rotIndex, 0.0);
			break;
		case 3:
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_SYSTEM_CLICK], false, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_GRIP_TOUCH], grip > 0.9f, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_GRIP_VALUE], grip, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRACKPAD_X], 0, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRACKPAD_Y], 0, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRACKPAD_TOUCH], false, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_JOYSTICK_X], 0, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_JOYSTICK_Y], 0, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_JOYSTICK_CLICK], false, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_JOYSTICK_TOUCH], rotThumb > 0.7f, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_A_CLICK], false, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_A_TOUCH], false, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_B_CLICK], false, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_B_TOUCH], false, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRIGGER_CLICK], rotIndex > 0.9f, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRIGGER_TOUCH], rotIndex > 0.7f, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRIGGER_VALUE], rotIndex, 0.0);
			break;
		}
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

		vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_FINGER_INDEX], rotIndex, 0.0);
		vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_FINGER_MIDDLE], rotMiddle, 0.0);
		vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_FINGER_RING], rotRing, 0.0);
		vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_FINGER_PINKY], rotPinky, 0.0);

		vr::VRServerDriverHost()->TrackedDevicePoseUpdated(m_unObjectId, m_pose, sizeof(vr::DriverPose_t));
	}
	else {

		switch (Settings::Instance().m_controllerMode) {
		case 2:
		case 3:
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_SYSTEM_CLICK], (c.buttons& ALVR_BUTTON_FLAG(ALVR_INPUT_SYSTEM_CLICK)) != 0, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_GRIP_TOUCH], (c.buttons& ALVR_BUTTON_FLAG(ALVR_INPUT_GRIP_TOUCH)) != 0, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_GRIP_VALUE], c.gripValue, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRACKPAD_X], c.trackpadPosition.x, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRACKPAD_Y], 0, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRACKPAD_TOUCH], false, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_JOYSTICK_X], c.trackpadPosition.x, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_JOYSTICK_Y], c.trackpadPosition.y, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_JOYSTICK_CLICK], (c.buttons& ALVR_BUTTON_FLAG(ALVR_INPUT_JOYSTICK_CLICK)) != 0, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_JOYSTICK_TOUCH], (c.buttons& ALVR_BUTTON_FLAG(ALVR_INPUT_JOYSTICK_TOUCH)) != 0, 0.0);
			if (!m_isLeftHand) {
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_A_CLICK], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_A_CLICK)) != 0, 0.0);
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_A_TOUCH], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_A_TOUCH)) != 0, 0.0);
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_B_CLICK], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_B_CLICK)) != 0, 0.0);
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_B_TOUCH], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_B_TOUCH)) != 0, 0.0);
			}
			else {
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_A_CLICK], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_X_CLICK)) != 0, 0.0);
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_A_TOUCH], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_X_TOUCH)) != 0, 0.0);
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_B_CLICK], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_Y_CLICK)) != 0, 0.0);
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_B_TOUCH], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_Y_TOUCH)) != 0, 0.0);
			}
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRIGGER_CLICK], (c.buttons& ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_CLICK)) != 0, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRIGGER_TOUCH], (c.buttons& ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_TOUCH)) != 0, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRIGGER_VALUE], c.triggerValue, 0.0);
			{
				float trigger = 0;
				if ((c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_TOUCH)) != 0)trigger = 0.5f;
				if ((c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_CLICK)) != 0)trigger = 1.0f;
				float grip = 0;
				if ((c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_GRIP_TOUCH)) != 0)grip = 0.5f;
				if ((c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_GRIP_CLICK)) != 0)grip = 1.0f;
				vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_FINGER_INDEX], trigger, 0.0);
				vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_FINGER_MIDDLE], grip, 0.0);
				if ((c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_X_TOUCH)) != 0 ||
					(c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_Y_TOUCH)) != 0 ||
					(c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_A_TOUCH)) != 0 ||
					(c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_B_TOUCH)) != 0 ||
					(c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_JOYSTICK_TOUCH)) != 0) {
					vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_FINGER_RING], 1, 0.0);
					vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_FINGER_PINKY], 1, 0.0);
				}
				else {
					vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_FINGER_RING], grip, 0.0);
					vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_FINGER_PINKY], grip, 0.0);
				}
			}
			break;

		case 0:
		case 1:
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
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_START_CLICK], (c.buttons& ALVR_BUTTON_FLAG(ALVR_INPUT_START_CLICK)) != 0, 0.0);

			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRIGGER_CLICK], (c.buttons& ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_CLICK)) != 0, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRIGGER_VALUE], c.triggerValue, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRIGGER_TOUCH], (c.buttons& ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_TOUCH)) != 0, 0.0);

			uint64_t currentThumbTouch = c.buttons & (ALVR_BUTTON_FLAG(ALVR_INPUT_A_TOUCH) | ALVR_BUTTON_FLAG(ALVR_INPUT_B_TOUCH) |
				ALVR_BUTTON_FLAG(ALVR_INPUT_X_TOUCH) | ALVR_BUTTON_FLAG(ALVR_INPUT_Y_TOUCH) | ALVR_BUTTON_FLAG(ALVR_INPUT_JOYSTICK_TOUCH));
			if (m_lastThumbTouch != currentThumbTouch) {
				m_thumbAnimationProgress += 1. / ANIMATION_FRAME_COUNT;
				if (m_thumbAnimationProgress > 1.) {
					m_thumbAnimationProgress = 0;
					m_lastThumbTouch = currentThumbTouch;
				}
			}
			else {
				m_thumbAnimationProgress = 0;
			}

			uint64_t currentIndexTouch = c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_TOUCH);
			if (m_lastIndexTouch != currentIndexTouch) {
				m_indexAnimationProgress += 1. / ANIMATION_FRAME_COUNT;
				if (m_indexAnimationProgress > 1.) {
					m_indexAnimationProgress = 0;
					m_lastIndexTouch = currentIndexTouch;
				}
			}
			else {
				m_indexAnimationProgress = 0;
			}

			uint64_t lastPoseTouch = m_lastThumbTouch + m_lastIndexTouch;
			
			vr::VRBoneTransform_t boneTransforms[SKELETON_BONE_COUNT];

			// Perform whatever logic is necessary to convert your device's input into a skeletal pose,
			// first to create a pose "With Controller", that is as close to the pose of the user's real
			// hand as possible
			GetBoneTransform(true, m_isLeftHand, m_thumbAnimationProgress, m_indexAnimationProgress, lastPoseTouch, c, boneTransforms);

			// Then update the WithController pose on the component with those transforms
			vr::EVRInputError err = vr::VRDriverInput()->UpdateSkeletonComponent(m_compSkeleton, vr::VRSkeletalMotionRange_WithController, boneTransforms, SKELETON_BONE_COUNT);
			if (err != vr::VRInputError_None)
			{
				// Handle failure case
				Log("UpdateSkeletonComponentfailed.  Error: %i\n", err);
			}


			GetBoneTransform(false, m_isLeftHand, m_thumbAnimationProgress, m_indexAnimationProgress, lastPoseTouch, c, boneTransforms);

			// Then update the WithoutController pose on the component 
			err = vr::VRDriverInput()->UpdateSkeletonComponent(m_compSkeleton, vr::VRSkeletalMotionRange_WithoutController, boneTransforms, SKELETON_BONE_COUNT);
			if (err != vr::VRInputError_None)
			{
				// Handle failure case
				Log("UpdateSkeletonComponentfailed.  Error: %i\n", err);
			}
			break;
		}

		// Battery
		vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_DeviceBatteryPercentage_Float, c.batteryPercentRemaining / 100.0f);

		vr::VRServerDriverHost()->TrackedDevicePoseUpdated(m_unObjectId, m_pose, sizeof(vr::DriverPose_t));

	}

	return false;
}

void GetThumbBoneTransform(bool withController, bool isLeftHand, uint64_t buttons, vr::VRBoneTransform_t outBoneTransform[]) {
	if (isLeftHand) {
		if ((buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_Y_TOUCH)) != 0) {
			//y touch
			if (withController) {
				outBoneTransform[2] = { {-0.017303, 0.032567, 0.025281, 1}, {0.317609, 0.528344 , 0.213134 , 0.757991} };
				outBoneTransform[3] = { {0.040406, 0.000000, -0.000000, 1}, {0.991742, 0.085317 , 0.019416 , 0.093765} };
				outBoneTransform[4] = { {0.032517, -0.000000, 0.000000, 1}, {0.959385, -0.012202 , -0.031055 , 0.280120} };
			}
			else {
				outBoneTransform[2] = { {-0.016426, 0.030866, 0.025118, 1}, {0.403850, 0.595704 , 0.082451 , 0.689380} };
				outBoneTransform[3] = { {0.040406, 0.000000, -0.000000, 1}, {0.989655, -0.090426 , 0.028457 , 0.107691} };
				outBoneTransform[4] = { {0.032517, 0.000000, 0.000000, 1}, {0.988590, 0.143978 , 0.041520 , 0.015363} };
			}
		}
		else if ((buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_X_TOUCH)) != 0) {
			//x touch
			if (withController) {
				outBoneTransform[2] = { {-0.017625, 0.031098, 0.022755, 1}, {0.388513, 0.527438 , 0.249444 , 0.713193} };
				outBoneTransform[3] = { {0.040406, 0.000000, -0.000000, 1}, {0.978341, 0.085924 , 0.037765 , 0.184501} };
				outBoneTransform[4] = { {0.032517, -0.000000, 0.000000, 1}, {0.894037, -0.043820 , -0.048328 , 0.443217} };
			}
			else {
				outBoneTransform[2] = { {-0.017288, 0.027151, 0.021465, 1}, {0.502777, 0.569978 , 0.147197 , 0.632988} };
				outBoneTransform[3] = { {0.040406, 0.000000, -0.000000, 1}, {0.970397, -0.048119 , 0.023261 , 0.235527} };
				outBoneTransform[4] = { {0.032517, 0.000000, 0.000000, 1}, {0.794064, 0.084451 , -0.037468 , 0.600772} };
			}
		}
		else if ((buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_JOYSTICK_TOUCH)) != 0) {
			//joy touch
			if (withController) {
				outBoneTransform[2] = { {-0.017914, 0.029178, 0.025298, 1}, {0.455126, 0.591760 , 0.168152 , 0.643743} };
				outBoneTransform[3] = { {0.040406, 0.000000, -0.000000, 1}, {0.969878, 0.084444 , 0.045679 , 0.223873} };
				outBoneTransform[4] = { {0.032517, -0.000000, 0.000000, 1}, {0.991257, 0.014384 , -0.005602 , 0.131040} };
			}
			else {
				outBoneTransform[2] = { {-0.017914, 0.029178, 0.025298, 1}, {0.455126, 0.591760 , 0.168152 , 0.643743} };
				outBoneTransform[3] = { {0.040406, 0.000000, -0.000000, 1}, {0.969878, 0.084444 , 0.045679 , 0.223873} };
				outBoneTransform[4] = { {0.032517, -0.000000, 0.000000, 1}, {0.991257, 0.014384 , -0.005602 , 0.131040} };
			}
		}
		else {
			// no touch
			outBoneTransform[2] = { {-0.012083, 0.028070, 0.025050, 1}, {0.464112, 0.567418 , 0.272106 , 0.623374} };
			outBoneTransform[3] = { {0.040406, 0.000000, -0.000000, 1}, {0.994838, 0.082939 , 0.019454 , 0.055130} };
			outBoneTransform[4] = { {0.032517, 0.000000, 0.000000, 1}, {0.974793, -0.003213 , 0.021867 , -0.222015} };
		}

		outBoneTransform[5] = { {0.030464, -0.000000, -0.000000, 1}, {1.000000, -0.000000 , 0.000000 , 0.000000} };
	}
	else {
		if ((buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_B_TOUCH)) != 0) {
			//b touch
			if (withController) {
				outBoneTransform[2] = { {0.017303, 0.032567, 0.025281, 1}, {0.528344, -0.317609 , 0.757991 , -0.213134} };
				outBoneTransform[3] = { {-0.040406, -0.000000, 0.000000, 1}, {0.991742, 0.085317 , 0.019416 , 0.093765} };
				outBoneTransform[4] = { {-0.032517, 0.000000, -0.000000, 1}, {0.959385, -0.012202 , -0.031055 , 0.280120} };
			}
			else {
				outBoneTransform[2] = { {0.016426, 0.030866, 0.025118, 1}, {0.595704, -0.403850 , 0.689380 , -0.082451} };
				outBoneTransform[3] = { {-0.040406, -0.000000, 0.000000, 1}, {0.989655, -0.090426 , 0.028457 , 0.107691} };
				outBoneTransform[4] = { {-0.032517, -0.000000, -0.000000, 1}, {0.988590, 0.143978 , 0.041520 , 0.015363} };
			}
		}
		else if ((buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_A_TOUCH)) != 0) {
			//a touch
			if (withController) {
				outBoneTransform[2] = { {0.017625, 0.031098, 0.022755, 1}, {0.527438, -0.388513 , 0.713193 , -0.249444} };
				outBoneTransform[3] = { {-0.040406, -0.000000, 0.000000, 1}, {0.978341, 0.085924 , 0.037765 , 0.184501} };
				outBoneTransform[4] = { {-0.032517, 0.000000, -0.000000, 1}, {0.894037, -0.043820 , -0.048328 , 0.443217} };
			}
			else {
				outBoneTransform[2] = { {0.017288, 0.027151, 0.021465, 1}, {0.569978, -0.502777 , 0.632988 , -0.147197} };
				outBoneTransform[3] = { {-0.040406, -0.000000, 0.000000, 1}, {0.970397, -0.048119 , 0.023261 , 0.235527} };
				outBoneTransform[4] = { {-0.032517, -0.000000, -0.000000, 1}, {0.794064, 0.084451 , -0.037468 , 0.600772} };
			}
		}
		else if ((buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_JOYSTICK_TOUCH)) != 0) {
			//joy touch
			if (withController) {
				outBoneTransform[2] = { {0.017914, 0.029178, 0.025298, 1}, {0.591760, -0.455126 , 0.643743 , -0.168152} };
				outBoneTransform[3] = { {-0.040406, -0.000000, 0.000000, 1}, {0.969878, 0.084444 , 0.045679 , 0.223873} };
				outBoneTransform[4] = { {-0.032517, 0.000000, -0.000000, 1}, {0.991257, 0.014384 , -0.005602 , 0.131040} };
			}
			else {
				outBoneTransform[2] = { {0.017914, 0.029178, 0.025298, 1}, {0.591760, -0.455126 , 0.643743 , -0.168152} };
				outBoneTransform[3] = { {-0.040406, -0.000000, 0.000000, 1}, {0.969878, 0.084444 , 0.045679 , 0.223873} };
				outBoneTransform[4] = { {-0.032517, 0.000000, -0.000000, 1}, {0.991257, 0.014384 , -0.005602 , 0.131040} };
			}
		}
		else {
			// no touch
			outBoneTransform[2] = { {0.012330, 0.028661, 0.025049, 1}, {0.571059, -0.451277 , 0.630056 , -0.270685} };
			outBoneTransform[3] = { {-0.040406, -0.000000, 0.000000, 1}, {0.994565, 0.078280 , 0.018282 , 0.066177} };
			outBoneTransform[4] = { {-0.032517, -0.000000, -0.000000, 1}, {0.977658, -0.003039 , 0.020722 , -0.209156} };
		}

		outBoneTransform[5] = { {-0.030464, 0.000000, 0.000000, 1}, {1.000000, -0.000000 , 0.000000 , 0.000000} };
	}
}

void GetTriggerBoneTransform(bool withController, bool isLeftHand, uint64_t buttons, vr::VRBoneTransform_t outBoneTransform[]) {
	if ((buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_CLICK)) != 0) {
		// click
		if (withController) {
			if (isLeftHand) {
				outBoneTransform[6] = { {-0.003925, 0.027171, 0.014640, 1}, {0.666448, 0.430031 , -0.455947 , 0.403772} };
				outBoneTransform[7] = { {0.076015, -0.005124, 0.000239, 1}, {-0.956011, -0.000025 , 0.158355 , -0.246913} };
				outBoneTransform[8] = { {0.043930, -0.000000, -0.000000, 1}, {-0.944138, -0.043351 , 0.014947 , -0.326345} };
				outBoneTransform[9] = { {0.028695, 0.000000, 0.000000, 1}, {-0.912149, 0.003626 , 0.039888 , -0.407898} };
				outBoneTransform[10] = { {0.022821, 0.000000, -0.000000, 1}, {1.000000, -0.000000 , -0.000000 , 0.000000} };
				outBoneTransform[11] = { {0.002177, 0.007120, 0.016319, 1}, {0.529359, 0.540512 , -0.463783 , 0.461011} };
				outBoneTransform[12] = { {0.070953, 0.000779, 0.000997, 1}, {0.847397, -0.257141 , -0.139135 , 0.443213} };
				outBoneTransform[13] = { {0.043108, 0.000000, 0.000000, 1}, {0.874907, 0.009875 , 0.026584 , 0.483460} };
				outBoneTransform[14] = { {0.033266, -0.000000, 0.000000, 1}, {0.894578, -0.036774 , -0.050597 , 0.442513} };
				outBoneTransform[15] = { {0.025892, -0.000000, 0.000000, 1}, {0.999195, -0.000000 , 0.000000 , 0.040126} };
				outBoneTransform[16] = { {0.000513, -0.006545, 0.016348, 1}, {0.500244, 0.530784 , -0.516215 , 0.448939} };
				outBoneTransform[17] = { {0.065876, 0.001786, 0.000693, 1}, {0.831617, -0.242931 , -0.139695 , 0.479461} };
				outBoneTransform[18] = { {0.040697, 0.000000, 0.000000, 1}, {0.769163, -0.001746 , 0.001363 , 0.639049} };
				outBoneTransform[19] = { {0.028747, -0.000000, -0.000000, 1}, {0.968615, -0.064538 , -0.046586 , 0.235477} };
				outBoneTransform[20] = { {0.022430, -0.000000, 0.000000, 1}, {1.000000, 0.000000 , -0.000000 , -0.000000} };
				outBoneTransform[21] = { {-0.002478, -0.018981, 0.015214, 1}, {0.474671, 0.434670 , -0.653212 , 0.398827} };
				outBoneTransform[22] = { {0.062878, 0.002844, 0.000332, 1}, {0.798788, -0.199577 , -0.094418 , 0.559636} };
				outBoneTransform[23] = { {0.030220, 0.000002, -0.000000, 1}, {0.853087, 0.001644 , -0.000913 , 0.521765} };
				outBoneTransform[24] = { {0.018187, -0.000002, 0.000000, 1}, {0.974249, 0.052491 , 0.003591 , 0.219249} };
				outBoneTransform[25] = { {0.018018, 0.000000, -0.000000, 1}, {1.000000, 0.000000 , 0.000000 , 0.000000} };
				outBoneTransform[26] = { {0.006629, 0.026690, 0.061870, 1}, {0.805084, -0.018369 , 0.584788 , -0.097597} };
				outBoneTransform[27] = { {-0.007882, -0.040478, 0.039337, 1}, {-0.322494, 0.932092 , 0.121861 , 0.111140} };
				outBoneTransform[28] = { {0.017136, -0.032633, 0.080682, 1}, {-0.169466, 0.800083 , 0.571006 , 0.071415} };
				outBoneTransform[29] = { {0.011144, -0.028727, 0.108366, 1}, {-0.076328, 0.788280 , 0.605097 , 0.081527} };
				outBoneTransform[30] = { {0.011333, -0.026044, 0.128585, 1}, {-0.144791, 0.737451 , 0.656958 , -0.060069} };
			}
			else {
				outBoneTransform[6] = { {-0.003925, 0.027171, 0.014640, 1}, {0.666448, 0.430031 , -0.455947 , 0.403772} };
				outBoneTransform[7] = { {0.076015, -0.005124, 0.000239, 1}, {-0.956011, -0.000025 , 0.158355 , -0.246913} };
				outBoneTransform[8] = { {0.043930, -0.000000, -0.000000, 1}, {-0.944138, -0.043351 , 0.014947 , -0.326345} };
				outBoneTransform[9] = { {0.028695, 0.000000, 0.000000, 1}, {-0.912149, 0.003626 , 0.039888 , -0.407898} };
				outBoneTransform[10] = { {0.022821, 0.000000, -0.000000, 1}, {1.000000, -0.000000 , -0.000000 , 0.000000} };
				outBoneTransform[11] = { {0.002177, 0.007120, 0.016319, 1}, {0.529359, 0.540512 , -0.463783 , 0.461011} };
				outBoneTransform[12] = { {0.070953, 0.000779, 0.000997, 1}, {0.847397, -0.257141 , -0.139135 , 0.443213} };
				outBoneTransform[13] = { {0.043108, 0.000000, 0.000000, 1}, {0.874907, 0.009875 , 0.026584 , 0.483460} };
				outBoneTransform[14] = { {0.033266, -0.000000, 0.000000, 1}, {0.894578, -0.036774 , -0.050597 , 0.442513} };
				outBoneTransform[15] = { {0.025892, -0.000000, 0.000000, 1}, {0.999195, -0.000000 , 0.000000 , 0.040126} };
				outBoneTransform[16] = { {0.000513, -0.006545, 0.016348, 1}, {0.500244, 0.530784 , -0.516215 , 0.448939} };
				outBoneTransform[17] = { {0.065876, 0.001786, 0.000693, 1}, {0.831617, -0.242931 , -0.139695 , 0.479461} };
				outBoneTransform[18] = { {0.040697, 0.000000, 0.000000, 1}, {0.769163, -0.001746 , 0.001363 , 0.639049} };
				outBoneTransform[19] = { {0.028747, -0.000000, -0.000000, 1}, {0.968615, -0.064538 , -0.046586 , 0.235477} };
				outBoneTransform[20] = { {0.022430, -0.000000, 0.000000, 1}, {1.000000, 0.000000 , -0.000000 , -0.000000} };
				outBoneTransform[21] = { {-0.002478, -0.018981, 0.015214, 1}, {0.474671, 0.434670 , -0.653212 , 0.398827} };
				outBoneTransform[22] = { {0.062878, 0.002844, 0.000332, 1}, {0.798788, -0.199577 , -0.094418 , 0.559636} };
				outBoneTransform[23] = { {0.030220, 0.000002, -0.000000, 1}, {0.853087, 0.001644 , -0.000913 , 0.521765} };
				outBoneTransform[24] = { {0.018187, -0.000002, 0.000000, 1}, {0.974249, 0.052491 , 0.003591 , 0.219249} };
				outBoneTransform[25] = { {0.018018, 0.000000, -0.000000, 1}, {1.000000, 0.000000 , 0.000000 , 0.000000} };
				outBoneTransform[26] = { {0.006629, 0.026690, 0.061870, 1}, {0.805084, -0.018369 , 0.584788 , -0.097597} };
				outBoneTransform[27] = { {-0.007882, -0.040478, 0.039337, 1}, {-0.322494, 0.932092 , 0.121861 , 0.111140} };
				outBoneTransform[28] = { {0.017136, -0.032633, 0.080682, 1}, {-0.169466, 0.800083 , 0.571006 , 0.071415} };
				outBoneTransform[29] = { {0.011144, -0.028727, 0.108366, 1}, {-0.076328, 0.788280 , 0.605097 , 0.081527} };
				outBoneTransform[30] = { {0.011333, -0.026044, 0.128585, 1}, {-0.144791, 0.737451 , 0.656958 , -0.060069} };
			}
		}
		else {
			if (isLeftHand) {
				outBoneTransform[6] = { {0.003802, 0.021514, 0.012803, 1}, {0.617314, 0.395175 , -0.510874 , 0.449185} };
				outBoneTransform[7] = { {0.074204, -0.005002, 0.000234, 1}, {0.737291, -0.032006 , -0.115013 , 0.664944} };
				outBoneTransform[8] = { {0.043287, -0.000000, -0.000000, 1}, {0.611381, 0.003287 , 0.003823 , 0.791320} };
				outBoneTransform[9] = { {0.028275, 0.000000, 0.000000, 1}, {0.745389, -0.000684 , -0.000945 , 0.666629} };
				outBoneTransform[10] = { {0.022821, 0.000000, -0.000000, 1}, {1.000000, 0.000000 , -0.000000 , 0.000000} };
				outBoneTransform[11] = { {0.004885, 0.006885, 0.016480, 1}, {0.522678, 0.527374 , -0.469333 , 0.477923} };
				outBoneTransform[12] = { {0.070953, 0.000779, 0.000997, 1}, {0.826071, -0.121321 , 0.017267 , 0.550082} };
				outBoneTransform[13] = { {0.043108, 0.000000, 0.000000, 1}, {0.956676, 0.013210 , 0.009330 , 0.290704} };
				outBoneTransform[14] = { {0.033266, 0.000000, 0.000000, 1}, {0.979740, -0.001605 , -0.019412 , 0.199323} };
				outBoneTransform[15] = { {0.025892, -0.000000, 0.000000, 1}, {0.999195, 0.000000 , 0.000000 , 0.040126} };
				outBoneTransform[16] = { {0.001696, -0.006648, 0.016418, 1}, {0.509620, 0.540794 , -0.504891 , 0.439220} };
				outBoneTransform[17] = { {0.065876, 0.001786, 0.000693, 1}, {0.955009, -0.065344 , -0.063228 , 0.282294} };
				outBoneTransform[18] = { {0.040577, 0.000000, 0.000000, 1}, {0.953823, -0.000972 , 0.000697 , 0.300366} };
				outBoneTransform[19] = { {0.028698, -0.000000, -0.000000, 1}, {0.977627, -0.001163 , -0.011433 , 0.210033} };
				outBoneTransform[20] = { {0.022430, -0.000000, 0.000000, 1}, {1.000000, 0.000000 , 0.000000 , 0.000000} };
				outBoneTransform[21] = { {-0.001792, -0.019041, 0.015254, 1}, {0.518602, 0.511152 , -0.596086 , 0.338315} };
				outBoneTransform[22] = { {0.062878, 0.002844, 0.000332, 1}, {0.978584, -0.045398 , -0.103083 , 0.172297} };
				outBoneTransform[23] = { {0.030154, 0.000000, 0.000000, 1}, {0.970479, -0.000068 , -0.002025 , 0.241175} };
				outBoneTransform[24] = { {0.018187, 0.000000, 0.000000, 1}, {0.997053, -0.000687 , -0.052009 , -0.056395} };
				outBoneTransform[25] = { {0.018018, 0.000000, -0.000000, 1}, {1.000000, -0.000000 , -0.000000 , -0.000000} };
				outBoneTransform[26] = { {-0.005193, 0.054191, 0.060030, 1}, {0.747374, 0.182388 , 0.599615 , 0.220518} };
				outBoneTransform[27] = { {0.000171, 0.016473, 0.096515, 1}, {-0.006456, 0.022747 , -0.932927 , -0.359287} };
				outBoneTransform[28] = { {-0.038019, -0.074839, 0.046941, 1}, {-0.199973, 0.698334 , -0.635627 , -0.261380} };
				outBoneTransform[29] = { {-0.036836, -0.089774, 0.081969, 1}, {-0.191006, 0.756582 , -0.607429 , -0.148761} };
				outBoneTransform[30] = { {-0.030241, -0.086049, 0.119881, 1}, {-0.019037, 0.779368 , -0.612017 , -0.132881} };
			}
			else {
				outBoneTransform[6] = { {-0.003802, 0.021514, 0.012803, 1}, {0.395174, -0.617314 , 0.449185 , 0.510874} };
				outBoneTransform[7] = { {-0.074204, 0.005002, -0.000234, 1}, {0.737291, -0.032006 , -0.115013 , 0.664944} };
				outBoneTransform[8] = { {-0.043287, 0.000000, 0.000000, 1}, {0.611381, 0.003287 , 0.003823 , 0.791320} };
				outBoneTransform[9] = { {-0.028275, -0.000000, -0.000000, 1}, {0.745389, -0.000684 , -0.000945 , 0.666629} };
				outBoneTransform[10] = { {-0.022821, -0.000000, 0.000000, 1}, {1.000000, 0.000000 , -0.000000 , 0.000000} };
				outBoneTransform[11] = { {-0.004885, 0.006885, 0.016480, 1}, {0.527233, -0.522513 , 0.478085 , 0.469510} };
				outBoneTransform[12] = { {-0.070953, -0.000779, -0.000997, 1}, {0.826317, -0.120120 , 0.019005 , 0.549918} };
				outBoneTransform[13] = { {-0.043108, -0.000000, -0.000000, 1}, {0.958363, 0.013484 , 0.007380 , 0.285138} };
				outBoneTransform[14] = { {-0.033266, -0.000000, -0.000000, 1}, {0.977901, -0.001431 , -0.018078 , 0.208279} };
				outBoneTransform[15] = { {-0.025892, 0.000000, -0.000000, 1}, {0.999195, 0.000000 , 0.000000 , 0.040126} };
				outBoneTransform[16] = { {-0.001696, -0.006648, 0.016418, 1}, {0.541481, -0.508179 , 0.441001 , 0.504054} };
				outBoneTransform[17] = { {-0.065876, -0.001786, -0.000693, 1}, {0.953780, -0.064506 , -0.058812 , 0.287548} };
				outBoneTransform[18] = { {-0.040577, -0.000000, -0.000000, 1}, {0.954761, -0.000983 , 0.000698 , 0.297372} };
				outBoneTransform[19] = { {-0.028698, 0.000000, 0.000000, 1}, {0.976924, -0.001344 , -0.010281 , 0.213335} };
				outBoneTransform[20] = { {-0.022430, 0.000000, -0.000000, 1}, {1.000000, 0.000000 , 0.000000 , 0.000000} };
				outBoneTransform[21] = { {0.001792, -0.019041, 0.015254, 1}, {0.510569, -0.514906 , 0.341115 , 0.598191} };
				outBoneTransform[22] = { {-0.062878, -0.002844, -0.000332, 1}, {0.979195, -0.043879 , -0.095103 , 0.173800} };
				outBoneTransform[23] = { {-0.030154, -0.000000, -0.000000, 1}, {0.971387, -0.000102 , -0.002019 , 0.237494} };
				outBoneTransform[24] = { {-0.018187, -0.000000, -0.000000, 1}, {0.997961, 0.000800 , -0.051911 , -0.037114} };
				outBoneTransform[25] = { {-0.018018, -0.000000, 0.000000, 1}, {1.000000, -0.000000 , -0.000000 , -0.000000} };
				outBoneTransform[26] = { {0.004392, 0.055515, 0.060253, 1}, {0.745924, 0.156756 , -0.597950 , -0.247953} };
				outBoneTransform[27] = { {-0.000171, 0.016473, 0.096515, 1}, {-0.006456, 0.022747 , 0.932927 , 0.359287} };
				outBoneTransform[28] = { {0.038119, -0.074730, 0.046338, 1}, {-0.207931, 0.699835 , 0.632631 , 0.258406} };
				outBoneTransform[29] = { {0.035492, -0.089519, 0.081636, 1}, {-0.197555, 0.760574 , 0.601098 , 0.145535} };
				outBoneTransform[30] = { {0.029073, -0.085957, 0.119561, 1}, {-0.031423, 0.791013 , 0.597190 , 0.129133} };
			}
		}
	}
	else if ((buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_TOUCH)) != 0) {
		// touch
		if (withController) {
			if (isLeftHand) {
				outBoneTransform[6] = { {-0.003925, 0.027171, 0.014640, 1}, {0.666448, 0.430031 , -0.455947 , 0.403772} };
				outBoneTransform[7] = { {0.074204, -0.005002, 0.000234, 1}, {-0.951843, 0.009717 , 0.158611 , -0.262188} };
				outBoneTransform[8] = { {0.043930, -0.000000, -0.000000, 1}, {-0.973045, -0.044676 , 0.010341 , -0.226012} };
				outBoneTransform[9] = { {0.028695, 0.000000, 0.000000, 1}, {-0.935253, -0.002881 , 0.023037 , -0.353217} };
				outBoneTransform[10] = { {0.022821, 0.000000, -0.000000, 1}, {1.000000, -0.000000 , -0.000000 , 0.000000} };
				outBoneTransform[11] = { {0.002177, 0.007120, 0.016319, 1}, {0.529359, 0.540512 , -0.463783 , 0.461011} };
				outBoneTransform[12] = { {0.070953, 0.000779, 0.000997, 1}, {0.847397, -0.257141 , -0.139135 , 0.443213} };
				outBoneTransform[13] = { {0.043108, 0.000000, 0.000000, 1}, {0.874907, 0.009875 , 0.026584 , 0.483460} };
				outBoneTransform[14] = { {0.033266, -0.000000, 0.000000, 1}, {0.894578, -0.036774 , -0.050597 , 0.442513} };
				outBoneTransform[15] = { {0.025892, -0.000000, 0.000000, 1}, {0.999195, -0.000000 , 0.000000 , 0.040126} };
				outBoneTransform[16] = { {0.000513, -0.006545, 0.016348, 1}, {0.500244, 0.530784 , -0.516215 , 0.448939} };
				outBoneTransform[17] = { {0.065876, 0.001786, 0.000693, 1}, {0.831617, -0.242931 , -0.139695 , 0.479461} };
				outBoneTransform[18] = { {0.040697, 0.000000, 0.000000, 1}, {0.769163, -0.001746 , 0.001363 , 0.639049} };
				outBoneTransform[19] = { {0.028747, -0.000000, -0.000000, 1}, {0.968615, -0.064538 , -0.046586 , 0.235477} };
				outBoneTransform[20] = { {0.022430, -0.000000, 0.000000, 1}, {1.000000, 0.000000 , -0.000000 , -0.000000} };
				outBoneTransform[21] = { {-0.002478, -0.018981, 0.015214, 1}, {0.474671, 0.434670 , -0.653212 , 0.398827} };
				outBoneTransform[22] = { {0.062878, 0.002844, 0.000332, 1}, {0.798788, -0.199577 , -0.094418 , 0.559636} };
				outBoneTransform[23] = { {0.030220, 0.000002, -0.000000, 1}, {0.853087, 0.001644 , -0.000913 , 0.521765} };
				outBoneTransform[24] = { {0.018187, -0.000002, 0.000000, 1}, {0.974249, 0.052491 , 0.003591 , 0.219249} };
				outBoneTransform[25] = { {0.018018, 0.000000, -0.000000, 1}, {1.000000, 0.000000 , 0.000000 , 0.000000} };
				outBoneTransform[26] = { {0.006629, 0.026690, 0.061870, 1}, {0.805084, -0.018369 , 0.584788 , -0.097597} };
				outBoneTransform[27] = { {-0.009005, -0.041708, 0.037992, 1}, {-0.338860, 0.939952 , -0.007564 , 0.040082} };
				outBoneTransform[28] = { {0.017136, -0.032633, 0.080682, 1}, {-0.169466, 0.800083 , 0.571006 , 0.071415} };
				outBoneTransform[29] = { {0.011144, -0.028727, 0.108366, 1}, {-0.076328, 0.788280 , 0.605097 , 0.081527} };
				outBoneTransform[30] = { {0.011333, -0.026044, 0.128585, 1}, {-0.144791, 0.737451 , 0.656958 , -0.060069} };
			}
			else {
				outBoneTransform[6] = { {-0.003925, 0.027171, 0.014640, 1}, {0.666448, 0.430031 , -0.455947 , 0.403772} };
				outBoneTransform[7] = { {0.074204, -0.005002, 0.000234, 1}, {-0.951843, 0.009717 , 0.158611 , -0.262188} };
				outBoneTransform[8] = { {0.043930, -0.000000, -0.000000, 1}, {-0.973045, -0.044676 , 0.010341 , -0.226012} };
				outBoneTransform[9] = { {0.028695, 0.000000, 0.000000, 1}, {-0.935253, -0.002881 , 0.023037 , -0.353217} };
				outBoneTransform[10] = { {0.022821, 0.000000, -0.000000, 1}, {1.000000, -0.000000 , -0.000000 , 0.000000} };
				outBoneTransform[11] = { {0.002177, 0.007120, 0.016319, 1}, {0.529359, 0.540512 , -0.463783 , 0.461011} };
				outBoneTransform[12] = { {0.070953, 0.000779, 0.000997, 1}, {0.847397, -0.257141 , -0.139135 , 0.443213} };
				outBoneTransform[13] = { {0.043108, 0.000000, 0.000000, 1}, {0.874907, 0.009875 , 0.026584 , 0.483460} };
				outBoneTransform[14] = { {0.033266, -0.000000, 0.000000, 1}, {0.894578, -0.036774 , -0.050597 , 0.442513} };
				outBoneTransform[15] = { {0.025892, -0.000000, 0.000000, 1}, {0.999195, -0.000000 , 0.000000 , 0.040126} };
				outBoneTransform[16] = { {0.000513, -0.006545, 0.016348, 1}, {0.500244, 0.530784 , -0.516215 , 0.448939} };
				outBoneTransform[17] = { {0.065876, 0.001786, 0.000693, 1}, {0.831617, -0.242931 , -0.139695 , 0.479461} };
				outBoneTransform[18] = { {0.040697, 0.000000, 0.000000, 1}, {0.769163, -0.001746 , 0.001363 , 0.639049} };
				outBoneTransform[19] = { {0.028747, -0.000000, -0.000000, 1}, {0.968615, -0.064538 , -0.046586 , 0.235477} };
				outBoneTransform[20] = { {0.022430, -0.000000, 0.000000, 1}, {1.000000, 0.000000 , -0.000000 , -0.000000} };
				outBoneTransform[21] = { {-0.002478, -0.018981, 0.015214, 1}, {0.474671, 0.434670 , -0.653212 , 0.398827} };
				outBoneTransform[22] = { {0.062878, 0.002844, 0.000332, 1}, {0.798788, -0.199577 , -0.094418 , 0.559636} };
				outBoneTransform[23] = { {0.030220, 0.000002, -0.000000, 1}, {0.853087, 0.001644 , -0.000913 , 0.521765} };
				outBoneTransform[24] = { {0.018187, -0.000002, 0.000000, 1}, {0.974249, 0.052491 , 0.003591 , 0.219249} };
				outBoneTransform[25] = { {0.018018, 0.000000, -0.000000, 1}, {1.000000, 0.000000 , 0.000000 , 0.000000} };
				outBoneTransform[26] = { {0.006629, 0.026690, 0.061870, 1}, {0.805084, -0.018369 , 0.584788 , -0.097597} };
				outBoneTransform[27] = { {-0.009005, -0.041708, 0.037992, 1}, {-0.338860, 0.939952 , -0.007564 , 0.040082} };
				outBoneTransform[28] = { {0.017136, -0.032633, 0.080682, 1}, {-0.169466, 0.800083 , 0.571006 , 0.071415} };
				outBoneTransform[29] = { {0.011144, -0.028727, 0.108366, 1}, {-0.076328, 0.788280 , 0.605097 , 0.081527} };
				outBoneTransform[30] = { {0.011333, -0.026044, 0.128585, 1}, {-0.144791, 0.737451 , 0.656958 , -0.060069} };
			}
		}
		else {
			if (isLeftHand) {
				outBoneTransform[6] = { {0.002693, 0.023387, 0.013573, 1}, {0.626743, 0.404630 , -0.499840 , 0.440032} };
				outBoneTransform[7] = { {0.074204, -0.005002, 0.000234, 1}, {0.869067, -0.019031 , -0.093524 , 0.485400} };
				outBoneTransform[8] = { {0.043512, -0.000000, -0.000000, 1}, {0.834068, 0.020722 , 0.003930 , 0.551259} };
				outBoneTransform[9] = { {0.028422, 0.000000, 0.000000, 1}, {0.890556, 0.000289 , -0.009290 , 0.454779} };
				outBoneTransform[10] = { {0.022821, 0.000000, -0.000000, 1}, {1.000000, 0.000000 , -0.000000 , 0.000000} };
				outBoneTransform[11] = { {0.003937, 0.006967, 0.016424, 1}, {0.531603, 0.532690 , -0.459598 , 0.471602} };
				outBoneTransform[12] = { {0.070953, 0.000779, 0.000997, 1}, {0.906933, -0.142169 , -0.015445 , 0.396261} };
				outBoneTransform[13] = { {0.043108, 0.000000, 0.000000, 1}, {0.975787, 0.014996 , 0.010867 , 0.217936} };
				outBoneTransform[14] = { {0.033266, 0.000000, 0.000000, 1}, {0.992777, -0.002096 , -0.021403 , 0.118029} };
				outBoneTransform[15] = { {0.025892, -0.000000, 0.000000, 1}, {0.999195, 0.000000 , 0.000000 , 0.040126} };
				outBoneTransform[16] = { {0.001282, -0.006612, 0.016394, 1}, {0.513688, 0.543325 , -0.502550 , 0.434011} };
				outBoneTransform[17] = { {0.065876, 0.001786, 0.000693, 1}, {0.971280, -0.068108 , -0.073480 , 0.215818} };
				outBoneTransform[18] = { {0.040619, 0.000000, 0.000000, 1}, {0.976566, -0.001379 , 0.000441 , 0.215216} };
				outBoneTransform[19] = { {0.028715, -0.000000, -0.000000, 1}, {0.987232, -0.000977 , -0.011919 , 0.158838} };
				outBoneTransform[20] = { {0.022430, -0.000000, 0.000000, 1}, {1.000000, 0.000000 , 0.000000 , 0.000000} };
				outBoneTransform[21] = { {-0.002032, -0.019020, 0.015240, 1}, {0.521784, 0.511917 , -0.594340 , 0.335325} };
				outBoneTransform[22] = { {0.062878, 0.002844, 0.000332, 1}, {0.982925, -0.053050 , -0.108004 , 0.139206} };
				outBoneTransform[23] = { {0.030177, 0.000000, 0.000000, 1}, {0.979798, 0.000394 , -0.001374 , 0.199982} };
				outBoneTransform[24] = { {0.018187, 0.000000, 0.000000, 1}, {0.997410, -0.000172 , -0.051977 , -0.049724} };
				outBoneTransform[25] = { {0.018018, 0.000000, -0.000000, 1}, {1.000000, -0.000000 , -0.000000 , -0.000000} };
				outBoneTransform[26] = { {-0.004857, 0.053377, 0.060017, 1}, {0.751040, 0.174397 , 0.601473 , 0.209178} };
				outBoneTransform[27] = { {-0.013234, -0.004327, 0.069740, 1}, {-0.119277, 0.262590 , -0.888979 , -0.355718} };
				outBoneTransform[28] = { {-0.037500, -0.074514, 0.046899, 1}, {-0.204942, 0.706005 , -0.626220 , -0.259623} };
				outBoneTransform[29] = { {-0.036251, -0.089302, 0.081732, 1}, {-0.194045, 0.764033 , -0.596592 , -0.150590} };
				outBoneTransform[30] = { {-0.029633, -0.085595, 0.119439, 1}, {-0.025015, 0.787219 , -0.601140 , -0.135243} };
			}
			else {
				outBoneTransform[6] = { {-0.002693, 0.023387, 0.013573, 1}, {0.404698, -0.626951 , 0.439894 , 0.499645} };
				outBoneTransform[7] = { {-0.074204, 0.005002, -0.000234, 1}, {0.870303, -0.017421 , -0.092515 , 0.483436} };
				outBoneTransform[8] = { {-0.043512, 0.000000, 0.000000, 1}, {0.835972, 0.018944 , 0.003312 , 0.548436} };
				outBoneTransform[9] = { {-0.028422, -0.000000, -0.000000, 1}, {0.890326, 0.000173 , -0.008504 , 0.455244} };
				outBoneTransform[10] = { {-0.022821, -0.000000, 0.000000, 1}, {1.000000, 0.000000 , -0.000000 , 0.000000} };
				outBoneTransform[11] = { {-0.003937, 0.006967, 0.016424, 1}, {0.532293, -0.531137 , 0.472074 , 0.460113} };
				outBoneTransform[12] = { {-0.070953, -0.000779, -0.000997, 1}, {0.908154, -0.139967 , -0.013210 , 0.394323} };
				outBoneTransform[13] = { {-0.043108, -0.000000, -0.000000, 1}, {0.977887, 0.015350 , 0.008912 , 0.208378} };
				outBoneTransform[14] = { {-0.033266, -0.000000, -0.000000, 1}, {0.992487, -0.002006 , -0.020888 , 0.120540} };
				outBoneTransform[15] = { {-0.025892, 0.000000, -0.000000, 1}, {0.999195, 0.000000 , 0.000000 , 0.040126} };
				outBoneTransform[16] = { {-0.001282, -0.006612, 0.016394, 1}, {0.544460, -0.511334 , 0.436935 , 0.501187} };
				outBoneTransform[17] = { {-0.065876, -0.001786, -0.000693, 1}, {0.971233, -0.064561 , -0.071188 , 0.217877} };
				outBoneTransform[18] = { {-0.040619, -0.000000, -0.000000, 1}, {0.978211, -0.001419 , 0.000451 , 0.207607} };
				outBoneTransform[19] = { {-0.028715, 0.000000, 0.000000, 1}, {0.987488, -0.001166 , -0.010852 , 0.157314} };
				outBoneTransform[20] = { {-0.022430, 0.000000, -0.000000, 1}, {1.000000, 0.000000 , 0.000000 , 0.000000} };
				outBoneTransform[21] = { {0.002032, -0.019020, 0.015240, 1}, {0.513640, -0.518192 , 0.337332 , 0.594860} };
				outBoneTransform[22] = { {-0.062878, -0.002844, -0.000332, 1}, {0.983501, -0.050059 , -0.104491 , 0.138930} };
				outBoneTransform[23] = { {-0.030177, -0.000000, -0.000000, 1}, {0.981170, 0.000501 , -0.001363 , 0.193138} };
				outBoneTransform[24] = { {-0.018187, -0.000000, -0.000000, 1}, {0.997801, 0.000487 , -0.051933 , -0.041173} };
				outBoneTransform[25] = { {-0.018018, -0.000000, 0.000000, 1}, {1.000000, -0.000000 , -0.000000 , -0.000000} };
				outBoneTransform[26] = { {0.004574, 0.055518, 0.060226, 1}, {0.745334, 0.161961 , -0.597782 , -0.246784} };
				outBoneTransform[27] = { {0.013831, -0.004360, 0.069547, 1}, {-0.117443, 0.257604 , 0.890065 , 0.357255} };
				outBoneTransform[28] = { {0.038220, -0.074817, 0.046428, 1}, {-0.205767, 0.697939 , 0.635107 , 0.259191} };
				outBoneTransform[29] = { {0.035802, -0.089658, 0.081733, 1}, {-0.196007, 0.758396 , 0.604341 , 0.145564} };
				outBoneTransform[30] = { {0.029364, -0.086069, 0.119701, 1}, {-0.028444, 0.787767 , 0.601616 , 0.129123} };
			}
		}
	}
	else {
		// no touch
		if (isLeftHand) {
			outBoneTransform[6] = { {0.000632, 0.026866, 0.015002, 1}, {0.644251, 0.421979 , -0.478202 , 0.422133} };
			outBoneTransform[7] = { {0.074204, -0.005002, 0.000234, 1}, {0.995332, 0.007007 , -0.039124 , 0.087949} };
			outBoneTransform[8] = { {0.043930, -0.000000, -0.000000, 1}, {0.997891, 0.045808 , 0.002142 , -0.045943} };
			outBoneTransform[9] = { {0.028695, 0.000000, 0.000000, 1}, {0.999649, 0.001850 , -0.022782 , -0.013409} };
			outBoneTransform[10] = { {0.022821, 0.000000, -0.000000, 1}, {1.000000, 0.000000 , -0.000000 , 0.000000} };
			outBoneTransform[11] = { {0.002177, 0.007120, 0.016319, 1}, {0.546723, 0.541277 , -0.442520 , 0.460749} };
			outBoneTransform[12] = { {0.070953, 0.000779, 0.000997, 1}, {0.980294, -0.167261 , -0.078959 , 0.069368} };
			outBoneTransform[13] = { {0.043108, 0.000000, 0.000000, 1}, {0.997947, 0.018493 , 0.013192 , 0.059886} };
			outBoneTransform[14] = { {0.033266, 0.000000, 0.000000, 1}, {0.997394, -0.003328 , -0.028225 , -0.066315} };
			outBoneTransform[15] = { {0.025892, -0.000000, 0.000000, 1}, {0.999195, 0.000000 , 0.000000 , 0.040126} };
			outBoneTransform[16] = { {0.000513, -0.006545, 0.016348, 1}, {0.516692, 0.550144 , -0.495548 , 0.429888} };
			outBoneTransform[17] = { {0.065876, 0.001786, 0.000693, 1}, {0.990420, -0.058696 , -0.101820 , 0.072495} };
			outBoneTransform[18] = { {0.040697, 0.000000, 0.000000, 1}, {0.999545, -0.002240 , 0.000004 , 0.030081} };
			outBoneTransform[19] = { {0.028747, -0.000000, -0.000000, 1}, {0.999102, -0.000721 , -0.012693 , 0.040420} };
			outBoneTransform[20] = { {0.022430, -0.000000, 0.000000, 1}, {1.000000, 0.000000 , 0.000000 , 0.000000} };
			outBoneTransform[21] = { {-0.002478, -0.018981, 0.015214, 1}, {0.526918, 0.523940 , -0.584025 , 0.326740} };
			outBoneTransform[22] = { {0.062878, 0.002844, 0.000332, 1}, {0.986609, -0.059615 , -0.135163 , 0.069132} };
			outBoneTransform[23] = { {0.030220, 0.000000, 0.000000, 1}, {0.994317, 0.001896 , -0.000132 , 0.106446} };
			outBoneTransform[24] = { {0.018187, 0.000000, 0.000000, 1}, {0.995931, -0.002010 , -0.052079 , -0.073526} };
			outBoneTransform[25] = { {0.018018, 0.000000, -0.000000, 1}, {1.000000, -0.000000 , -0.000000 , -0.000000} };
			outBoneTransform[26] = { {-0.006059, 0.056285, 0.060064, 1}, {0.737238, 0.202745 , 0.594267 , 0.249441} };
			outBoneTransform[27] = { {-0.040416, -0.043018, 0.019345, 1}, {-0.290330, 0.623527 , -0.663809 , -0.293734} };
			outBoneTransform[28] = { {-0.039354, -0.075674, 0.047048, 1}, {-0.187047, 0.678062 , -0.659285 , -0.265683} };
			outBoneTransform[29] = { {-0.038340, -0.090987, 0.082579, 1}, {-0.183037, 0.736793 , -0.634757 , -0.143936} };
			outBoneTransform[30] = { {-0.031806, -0.087214, 0.121015, 1}, {-0.003659, 0.758407 , -0.639342 , -0.126678} };
		}
		else {
			outBoneTransform[6] = { {-0.000632, 0.026866, 0.015002, 1}, {0.421833, -0.643793 , 0.422458 , 0.478661} };
			outBoneTransform[7] = { {-0.074204, 0.005002, -0.000234, 1}, {0.994784, 0.007053 , -0.041286 , 0.093009} };
			outBoneTransform[8] = { {-0.043930, 0.000000, 0.000000, 1}, {0.998404, 0.045905 , 0.002780 , -0.032767} };
			outBoneTransform[9] = { {-0.028695, -0.000000, -0.000000, 1}, {0.999704, 0.001955 , -0.022774 , -0.008282} };
			outBoneTransform[10] = { {-0.022821, -0.000000, 0.000000, 1}, {1.000000, 0.000000 , -0.000000 , 0.000000} };
			outBoneTransform[11] = { {-0.002177, 0.007120, 0.016319, 1}, {0.541874, -0.547427 , 0.459996 , 0.441701} };
			outBoneTransform[12] = { {-0.070953, -0.000779, -0.000997, 1}, {0.979837, -0.168061 , -0.075910 , 0.076899} };
			outBoneTransform[13] = { {-0.043108, -0.000000, -0.000000, 1}, {0.997271, 0.018278 , 0.013375 , 0.070266} };
			outBoneTransform[14] = { {-0.033266, -0.000000, -0.000000, 1}, {0.998402, -0.003143 , -0.026423 , -0.049849} };
			outBoneTransform[15] = { {-0.025892, 0.000000, -0.000000, 1}, {0.999195, 0.000000 , 0.000000 , 0.040126} };
			outBoneTransform[16] = { {-0.000513, -0.006545, 0.016348, 1}, {0.548983, -0.519068 , 0.426914 , 0.496920} };
			outBoneTransform[17] = { {-0.065876, -0.001786, -0.000693, 1}, {0.989791, -0.065882 , -0.096417 , 0.081716} };
			outBoneTransform[18] = { {-0.040697, -0.000000, -0.000000, 1}, {0.999102, -0.002168 , -0.000020 , 0.042317} };
			outBoneTransform[19] = { {-0.028747, 0.000000, 0.000000, 1}, {0.998584, -0.000674 , -0.012714 , 0.051653} };
			outBoneTransform[20] = { {-0.022430, 0.000000, -0.000000, 1}, {1.000000, 0.000000 , 0.000000 , 0.000000} };
			outBoneTransform[21] = { {0.002478, -0.018981, 0.015214, 1}, {0.518597, -0.527304 , 0.328264 , 0.587580} };
			outBoneTransform[22] = { {-0.062878, -0.002844, -0.000332, 1}, {0.987294, -0.063356 , -0.125964 , 0.073274} };
			outBoneTransform[23] = { {-0.030220, -0.000000, -0.000000, 1}, {0.993413, 0.001573 , -0.000147 , 0.114578} };
			outBoneTransform[24] = { {-0.018187, -0.000000, -0.000000, 1}, {0.997047, -0.000695 , -0.052009 , -0.056495} };
			outBoneTransform[25] = { {-0.018018, -0.000000, 0.000000, 1}, {1.000000, -0.000000 , -0.000000 , -0.000000} };
			outBoneTransform[26] = { {0.005198, 0.054204, 0.060030, 1}, {0.747318, 0.182508 , -0.599586 , -0.220688} };
			outBoneTransform[27] = { {0.038779, -0.042973, 0.019824, 1}, {-0.297445, 0.639373 , 0.648910 , 0.285734} };
			outBoneTransform[28] = { {0.038027, -0.074844, 0.046941, 1}, {-0.199898, 0.698218 , 0.635767 , 0.261406} };
			outBoneTransform[29] = { {0.036845, -0.089781, 0.081973, 1}, {-0.190960, 0.756469 , 0.607591 , 0.148733} };
			outBoneTransform[30] = { {0.030251, -0.086056, 0.119887, 1}, {-0.018948, 0.779249 , 0.612180 , 0.132846} };
		}
	}
}

void GetGripClickBoneTransform(bool withController, bool isLeftHand, vr::VRBoneTransform_t outBoneTransform[]) {
	if (withController) {
		if (isLeftHand) {
			outBoneTransform[11] = { {0.002177, 0.007120, 0.016319, 1}, {0.529359, 0.540512 , -0.463783 , 0.461011} };
			outBoneTransform[12] = { {0.070953, 0.000779, 0.000997, 1}, {-0.831727, 0.270927 , 0.175647 , -0.451638} };
			outBoneTransform[13] = { {0.043108, 0.000000, 0.000000, 1}, {-0.854886, -0.008231 , -0.028107 , -0.517990} };
			outBoneTransform[14] = { {0.033266, -0.000000, 0.000000, 1}, {-0.825759, 0.085208 , 0.086456 , -0.550805} };
			outBoneTransform[15] = { {0.025892, -0.000000, 0.000000, 1}, {0.999195, -0.000000 , 0.000000 , 0.040126} };
			outBoneTransform[16] = { {0.000513, -0.006545, 0.016348, 1}, {0.500244, 0.530784 , -0.516215 , 0.448939} };
			outBoneTransform[17] = { {0.065876, 0.001786, 0.000693, 1}, {0.831617, -0.242931 , -0.139695 , 0.479461} };
			outBoneTransform[18] = { {0.040697, 0.000000, 0.000000, 1}, {0.769163, -0.001746 , 0.001363 , 0.639049} };
			outBoneTransform[19] = { {0.028747, -0.000000, -0.000000, 1}, {0.968615, -0.064537 , -0.046586 , 0.235477} };
			outBoneTransform[20] = { {0.022430, -0.000000, 0.000000, 1}, {1.000000, 0.000000 , -0.000000 , -0.000000} };
			outBoneTransform[21] = { {-0.002478, -0.018981, 0.015214, 1}, {0.474671, 0.434670 , -0.653212 , 0.398827} };
			outBoneTransform[22] = { {0.062878, 0.002844, 0.000332, 1}, {0.798788, -0.199577 , -0.094418 , 0.559636} };
			outBoneTransform[23] = { {0.030220, 0.000002, -0.000000, 1}, {0.853087, 0.001644 , -0.000913 , 0.521765} };
			outBoneTransform[24] = { {0.018187, -0.000002, 0.000000, 1}, {0.974249, 0.052491 , 0.003591 , 0.219249} };
			outBoneTransform[25] = { {0.018018, 0.000000, -0.000000, 1}, {1.000000, 0.000000 , 0.000000 , 0.000000} };

			outBoneTransform[28] = { {0.016642, -0.029992, 0.083200, 1}, {-0.094577, 0.694550 , 0.702845 , 0.121100} };
			outBoneTransform[29] = { {0.011144, -0.028727, 0.108366, 1}, {-0.076328, 0.788280 , 0.605097 , 0.081527} };
			outBoneTransform[30] = { {0.011333, -0.026044, 0.128585, 1}, {-0.144791, 0.737451 , 0.656958 , -0.060069} };
		}
		else {
			outBoneTransform[11] = { {0.002177, 0.007120, 0.016319, 1}, {0.529359, 0.540512 , -0.463783 , 0.461011} };
			outBoneTransform[12] = { {0.070953, 0.000779, 0.000997, 1}, {-0.831727, 0.270927 , 0.175647 , -0.451638} };
			outBoneTransform[13] = { {0.043108, 0.000000, 0.000000, 1}, {-0.854886, -0.008231 , -0.028107 , -0.517990} };
			outBoneTransform[14] = { {0.033266, -0.000000, 0.000000, 1}, {-0.825759, 0.085208 , 0.086456 , -0.550805} };
			outBoneTransform[15] = { {0.025892, -0.000000, 0.000000, 1}, {0.999195, -0.000000 , 0.000000 , 0.040126} };
			outBoneTransform[16] = { {0.000513, -0.006545, 0.016348, 1}, {0.500244, 0.530784 , -0.516215 , 0.448939} };
			outBoneTransform[17] = { {0.065876, 0.001786, 0.000693, 1}, {0.831617, -0.242931 , -0.139695 , 0.479461} };
			outBoneTransform[18] = { {0.040697, 0.000000, 0.000000, 1}, {0.769163, -0.001746 , 0.001363 , 0.639049} };
			outBoneTransform[19] = { {0.028747, -0.000000, -0.000000, 1}, {0.968615, -0.064537 , -0.046586 , 0.235477} };
			outBoneTransform[20] = { {0.022430, -0.000000, 0.000000, 1}, {1.000000, 0.000000 , -0.000000 , -0.000000} };
			outBoneTransform[21] = { {-0.002478, -0.018981, 0.015214, 1}, {0.474671, 0.434670 , -0.653212 , 0.398827} };
			outBoneTransform[22] = { {0.062878, 0.002844, 0.000332, 1}, {0.798788, -0.199577 , -0.094418 , 0.559636} };
			outBoneTransform[23] = { {0.030220, 0.000002, -0.000000, 1}, {0.853087, 0.001644 , -0.000913 , 0.521765} };
			outBoneTransform[24] = { {0.018187, -0.000002, 0.000000, 1}, {0.974249, 0.052491 , 0.003591 , 0.219249} };
			outBoneTransform[25] = { {0.018018, 0.000000, -0.000000, 1}, {1.000000, 0.000000 , 0.000000 , 0.000000} };

			outBoneTransform[28] = { {0.016642, -0.029992, 0.083200, 1}, {-0.094577, 0.694550 , 0.702845 , 0.121100} };
			outBoneTransform[29] = { {0.011144, -0.028727, 0.108366, 1}, {-0.076328, 0.788280 , 0.605097 , 0.081527} };
			outBoneTransform[30] = { {0.011333, -0.026044, 0.128585, 1}, {-0.144791, 0.737451 , 0.656958 , -0.060069} };
		}

	}
	else {
		if (isLeftHand) {
			outBoneTransform[11] = { {0.005787, 0.006806, 0.016534, 1}, {0.514203, 0.522315 , -0.478348 , 0.483700} };
			outBoneTransform[12] = { {0.070953, 0.000779, 0.000997, 1}, {0.723653, -0.097901 , 0.048546 , 0.681458} };
			outBoneTransform[13] = { {0.043108, 0.000000, 0.000000, 1}, {0.637464, -0.002366 , -0.002831 , 0.770472} };
			outBoneTransform[14] = { {0.033266, 0.000000, 0.000000, 1}, {0.658008, 0.002610 , 0.003196 , 0.753000} };
			outBoneTransform[15] = { {0.025892, -0.000000, 0.000000, 1}, {0.999195, 0.000000 , 0.000000 , 0.040126} };
			outBoneTransform[16] = { {0.004123, -0.006858, 0.016563, 1}, {0.489609, 0.523374 , -0.520644 , 0.463997} };
			outBoneTransform[17] = { {0.065876, 0.001786, 0.000693, 1}, {0.759970, -0.055609 , 0.011571 , 0.647471} };
			outBoneTransform[18] = { {0.040331, 0.000000, 0.000000, 1}, {0.664315, 0.001595 , 0.001967 , 0.747449} };
			outBoneTransform[19] = { {0.028489, -0.000000, -0.000000, 1}, {0.626957, -0.002784 , -0.003234 , 0.779042} };
			outBoneTransform[20] = { {0.022430, -0.000000, 0.000000, 1}, {1.000000, 0.000000 , 0.000000 , 0.000000} };
			outBoneTransform[21] = { {0.001131, -0.019295, 0.015429, 1}, {0.479766, 0.477833 , -0.630198 , 0.379934} };
			outBoneTransform[22] = { {0.062878, 0.002844, 0.000332, 1}, {0.827001, 0.034282 , 0.003440 , 0.561144} };
			outBoneTransform[23] = { {0.029874, 0.000000, 0.000000, 1}, {0.702185, -0.006716 , -0.009289 , 0.711903} };
			outBoneTransform[24] = { {0.017979, 0.000000, 0.000000, 1}, {0.676853, 0.007956 , 0.009917 , 0.736009} };
			outBoneTransform[25] = { {0.018018, 0.000000, -0.000000, 1}, {1.000000, -0.000000 , -0.000000 , -0.000000} };

			outBoneTransform[28] = { {0.000448, 0.001536, 0.116543, 1}, {-0.039357, 0.105143 , -0.928833 , -0.353079} };
			outBoneTransform[29] = { {0.003949, -0.014869, 0.130608, 1}, {-0.055071, 0.068695 , -0.944016 , -0.317933} };
			outBoneTransform[30] = { {0.003263, -0.034685, 0.139926, 1}, {0.019690, -0.100741 , -0.957331 , -0.270149} };
		}
		else {
			outBoneTransform[11] = { {-0.005787, 0.006806, 0.016534, 1}, {0.522315, -0.514203 , 0.483700 , 0.478348} };
			outBoneTransform[12] = { {-0.070953, -0.000779, -0.000997, 1}, {0.723653, -0.097901 , 0.048546 , 0.681458} };
			outBoneTransform[13] = { {-0.043108, -0.000000, -0.000000, 1}, {0.637464, -0.002366 , -0.002831 , 0.770472} };
			outBoneTransform[14] = { {-0.033266, -0.000000, -0.000000, 1}, {0.658008, 0.002610 , 0.003196 , 0.753000} };
			outBoneTransform[15] = { {-0.025892, 0.000000, -0.000000, 1}, {0.999195, 0.000000 , 0.000000 , 0.040126} };
			outBoneTransform[16] = { {-0.004123, -0.006858, 0.016563, 1}, {0.523374, -0.489609 , 0.463997 , 0.520644} };
			outBoneTransform[17] = { {-0.065876, -0.001786, -0.000693, 1}, {0.759970, -0.055609 , 0.011571 , 0.647471} };
			outBoneTransform[18] = { {-0.040331, -0.000000, -0.000000, 1}, {0.664315, 0.001595 , 0.001967 , 0.747449} };
			outBoneTransform[19] = { {-0.028489, 0.000000, 0.000000, 1}, {0.626957, -0.002784 , -0.003234 , 0.779042} };
			outBoneTransform[20] = { {-0.022430, 0.000000, -0.000000, 1}, {1.000000, 0.000000 , 0.000000 , 0.000000} };
			outBoneTransform[21] = { {-0.001131, -0.019295, 0.015429, 1}, {0.477833, -0.479766 , 0.379935 , 0.630198} };
			outBoneTransform[22] = { {-0.062878, -0.002844, -0.000332, 1}, {0.827001, 0.034282 , 0.003440 , 0.561144} };
			outBoneTransform[23] = { {-0.029874, -0.000000, -0.000000, 1}, {0.702185, -0.006716 , -0.009289 , 0.711903} };
			outBoneTransform[24] = { {-0.017979, -0.000000, -0.000000, 1}, {0.676853, 0.007956 , 0.009917 , 0.736009} };
			outBoneTransform[25] = { {-0.018018, -0.000000, 0.000000, 1}, {1.000000, -0.000000 , -0.000000 , -0.000000} };

			outBoneTransform[28] = { {-0.000448, 0.001536, 0.116543, 1}, {-0.039357, 0.105143 , 0.928833 , 0.353079} };
			outBoneTransform[29] = { {-0.003949, -0.014869, 0.130608, 1}, {-0.055071, 0.068695 , 0.944016 , 0.317933} };
			outBoneTransform[30] = { {-0.003263, -0.034685, 0.139926, 1}, {0.019690, -0.100741 , 0.957331 , 0.270149} };
		}
	}
}

void OvrController::GetBoneTransform(bool withController, bool isLeftHand, float thumbAnimationProgress, float indexAnimationProgress, uint64_t lastPoseButtons, const TrackingInfo::Controller& c, vr::VRBoneTransform_t outBoneTransform[]) {

	vr::VRBoneTransform_t boneTransform1[SKELETON_BONE_COUNT];
	vr::VRBoneTransform_t boneTransform2[SKELETON_BONE_COUNT];

	// root and wrist
	outBoneTransform[0] = { {0.000000, 0.000000, 0.000000, 1}, {1.000000, -0.000000 , -0.000000 , 0.000000} };
	if (isLeftHand) {
		outBoneTransform[1] = { {-0.034038, 0.036503, 0.164722, 1}, {-0.055147, -0.078608 , -0.920279 , 0.379296} };
	}
	else {
		outBoneTransform[1] = { {0.034038, 0.036503, 0.164722, 1}, {-0.055147, -0.078608 , 0.920279 , -0.379296} };
	}

	//thumb
	GetThumbBoneTransform(withController, isLeftHand, lastPoseButtons, boneTransform1);
	GetThumbBoneTransform(withController, isLeftHand, c.buttons, boneTransform2);
	for (int boneIdx = 2; boneIdx < 6; boneIdx++) {
		outBoneTransform[boneIdx].position = Lerp(boneTransform1[boneIdx].position, boneTransform2[boneIdx].position, thumbAnimationProgress);
		outBoneTransform[boneIdx].orientation = Slerp(boneTransform1[boneIdx].orientation, boneTransform2[boneIdx].orientation, thumbAnimationProgress);
	}

	//trigger (index to pinky)
	if (c.triggerValue > 0) {
		GetTriggerBoneTransform(withController, isLeftHand, ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_TOUCH), boneTransform1);
		GetTriggerBoneTransform(withController, isLeftHand, ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_CLICK), boneTransform2);
		for (int boneIdx = 6; boneIdx < SKELETON_BONE_COUNT; boneIdx++) {
			outBoneTransform[boneIdx].position = Lerp(boneTransform1[boneIdx].position, boneTransform2[boneIdx].position, c.triggerValue);
			outBoneTransform[boneIdx].orientation = Slerp(boneTransform1[boneIdx].orientation, boneTransform2[boneIdx].orientation, c.triggerValue);
		}
	}
	else {
		GetTriggerBoneTransform(withController, isLeftHand, lastPoseButtons, boneTransform1);
		GetTriggerBoneTransform(withController, isLeftHand, c.buttons, boneTransform2);
		for (int boneIdx = 6; boneIdx < SKELETON_BONE_COUNT; boneIdx++) {
			outBoneTransform[boneIdx].position = Lerp(boneTransform1[boneIdx].position, boneTransform2[boneIdx].position, indexAnimationProgress);
			outBoneTransform[boneIdx].orientation = Slerp(boneTransform1[boneIdx].orientation, boneTransform2[boneIdx].orientation, indexAnimationProgress);
		}
	}

	// grip (middle to pinky)
	if (c.gripValue > 0) {
		GetGripClickBoneTransform(withController, isLeftHand, boneTransform2);
		for (int boneIdx = 11; boneIdx < 26; boneIdx++) {
			outBoneTransform[boneIdx].position = Lerp(outBoneTransform[boneIdx].position, boneTransform2[boneIdx].position, c.gripValue);
			outBoneTransform[boneIdx].orientation = Slerp(outBoneTransform[boneIdx].orientation, boneTransform2[boneIdx].orientation, c.gripValue);
		}
		for (int boneIdx = 28; boneIdx < SKELETON_BONE_COUNT; boneIdx++) {
			outBoneTransform[boneIdx].position = Lerp(outBoneTransform[boneIdx].position, boneTransform2[boneIdx].position, c.gripValue);
			outBoneTransform[boneIdx].orientation = Slerp(outBoneTransform[boneIdx].orientation, boneTransform2[boneIdx].orientation, c.gripValue);
		}
	}
}


std::string OvrController::GetSerialNumber() {
	char str[100];
	snprintf(str, sizeof(str), "_%s", m_index == 0 ? "Left" : "Right");
	return Settings::Instance().m_controllerSerialNumber + str;
}