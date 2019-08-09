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
	vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_RegisteredDeviceType_String, Settings::Instance().mControllerRegisteredDeviceType.c_str());

	uint64_t supportedButtons = 0xFFFFFFFFFFFFFFFFULL;
	vr::VRProperties()->SetUint64Property(m_ulPropertyContainer, vr::Prop_SupportedButtons_Uint64, supportedButtons);

	vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_DeviceProvidesBatteryStatus_Bool, true);


	vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_Axis0Type_Int32, vr::k_eControllerAxis_Joystick);

	vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_ControllerRoleHint_Int32, m_isLeftHand ? vr::TrackedControllerRole_LeftHand : vr::TrackedControllerRole_RightHand);

	vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_ControllerType_String, Settings::Instance().m_controllerType.c_str());
	//vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_LegacyInputProfile_String, Settings::Instance().m_controllerLegacyInputProfile.c_str());
	vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_InputProfilePath_String, Settings::Instance().m_controllerInputProfilePath.c_str());
	int i = 0;


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

bool OvrController::onPoseUpdate(int controllerIndex, const TrackingInfo &info) {

	if (m_unObjectId == vr::k_unTrackedDeviceIndexInvalid) {
		return false;
	}
	
	m_pose.qRotation = HmdQuaternion_Init(info.controller[controllerIndex].orientation.w,
		info.controller[controllerIndex].orientation.x,
		info.controller[controllerIndex].orientation.y,
		info.controller[controllerIndex].orientation.z);   //controllerRotation;
		

	m_pose.vecPosition[0] = info.controller[controllerIndex].position.x;
	m_pose.vecPosition[1] = info.controller[controllerIndex].position.y;
	m_pose.vecPosition[2] = info.controller[controllerIndex].position.z;

	

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


	return false;
}

std::string OvrController::GetSerialNumber() {
	char str[100];
	snprintf(str, sizeof(str), "_%s", m_index == 0 ? "Left" : "Right");
	return Settings::Instance().m_controllerSerialNumber + str;
}