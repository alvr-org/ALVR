#include "OvrController.h"

#include <algorithm>
#include <cstring>
#include <string_view>

#include "Settings.h"
#include "Utils.h"
#include "include/openvr_math.h"
#include "Logger.h"

OvrController::OvrController(bool isLeftHand, int index, float* poseTimeOffset)
	: m_unObjectId(vr::k_unTrackedDeviceIndexInvalid)
	, m_isLeftHand(isLeftHand)
	, m_index(index)
	, m_poseTimeOffset(poseTimeOffset)
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
	Debug("RemoteController::Activate. objectId=%d\n", unObjectId);

	const bool isViveTracker = Settings::Instance().m_controllerMode == 8 ||
							   Settings::Instance().m_controllerMode == 9;
	m_unObjectId = unObjectId;
	m_ulPropertyContainer = vr::VRProperties()->TrackedDeviceToPropertyContainer(m_unObjectId);

	vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_TrackingSystemName_String,
		Settings::Instance().m_useHeadsetTrackingSystem ?
			Settings::Instance().mTrackingSystemName.c_str() :
		 	Settings::Instance().m_controllerTrackingSystemName.c_str());
	vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_ManufacturerName_String, Settings::Instance().m_controllerManufacturerName.c_str());
	vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_ModelNumber_String, m_isLeftHand ? (Settings::Instance().m_controllerModelNumber + " (Left Controller)").c_str() : (Settings::Instance().m_controllerModelNumber + " (Right Controller)").c_str());

	vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_RenderModelName_String, m_isLeftHand ? Settings::Instance().m_controllerRenderModelNameLeft.c_str() : Settings::Instance().m_controllerRenderModelNameRight.c_str());

	vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_SerialNumber_String, GetSerialNumber().c_str());
	vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_AttachedDeviceId_String, GetSerialNumber().c_str());

	const std::string regDeviceTypeString = [this, isViveTracker]()
	{
		const auto& settings = Settings::Instance();
		if (isViveTracker)
		{
			static constexpr const std::string_view vive_prefix = "vive_tracker_";
			const auto& ctrlType = m_isLeftHand ? settings.m_controllerTypeLeft : settings.m_controllerTypeRight;
			std::string ret = settings.mControllerRegisteredDeviceType;
			if (ret.length() > 0 && ret[ret.length()-1] != '/')
				ret += '/';
			ret += ctrlType.length() <= vive_prefix.length() ? ctrlType : ctrlType.substr(vive_prefix.length());
			return ret;
		}
		return m_isLeftHand ?
			(Settings::Instance().mControllerRegisteredDeviceType + "_Left") :
			(Settings::Instance().mControllerRegisteredDeviceType + "_Right");
	}();
	vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_RegisteredDeviceType_String, regDeviceTypeString.c_str());

	uint64_t supportedButtons = 0xFFFFFFFFFFFFFFFFULL;
	vr::VRProperties()->SetUint64Property(m_ulPropertyContainer, vr::Prop_SupportedButtons_Uint64, supportedButtons);

	vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_DeviceProvidesBatteryStatus_Bool, true);


	vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_Axis0Type_Int32, vr::k_eControllerAxis_Joystick);

	vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_ControllerRoleHint_Int32, isViveTracker ? 
																										vr::TrackedControllerRole_Invalid :
																										(m_isLeftHand ? vr::TrackedControllerRole_LeftHand : vr::TrackedControllerRole_RightHand));

	vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_ControllerType_String, m_isLeftHand ? Settings::Instance().m_controllerTypeLeft.c_str() : Settings::Instance().m_controllerTypeRight.c_str());
	vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_InputProfilePath_String, Settings::Instance().m_controllerInputProfilePath.c_str());

	switch (Settings::Instance().m_controllerMode) {
	case 0:	//Oculus Rift
	case 1:	//Oculus Rift no pinch
	case 6:	//Oculus Quest
	case 7:	//Oculus Quest no pinch

	vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/system/click", &m_handles[ALVR_INPUT_SYSTEM_CLICK]);
	vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/system/touch", &m_handles[ALVR_INPUT_THUMB_REST_TOUCH]);
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

	case 8:
	case 9: { // Vive Tracker
		// All of these property values were dumped from real a vive tracker via https://github.com/SDraw/openvr_dumper
		// and were copied from https://github.com/SDraw/driver_kinectV2
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_ResourceRoot_String, "htc");
		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_WillDriftInYaw_Bool, false);
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_TrackingFirmwareVersion_String, "1541800000 RUNNER-WATCHMAN$runner-watchman@runner-watchman 2018-01-01 FPGA 512(2.56/0/0) BL 0 VRC 1541800000 Radio 1518800000"); // Changed
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_HardwareRevision_String, "product 128 rev 2.5.6 lot 2000/0/0 0");
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_ConnectedWirelessDongle_String, "D0000BE000");
		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_DeviceIsWireless_Bool, true);
		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_DeviceIsCharging_Bool, false);
		vr::VRProperties()->SetInt32Property(m_ulPropertyContainer, vr::Prop_ControllerHandSelectionPriority_Int32, -1);
		vr::HmdMatrix34_t l_transform = { -1.f, 0.f, 0.f, 0.f, 0.f, 0.f, -1.f, 0.f, 0.f, -1.f, 0.f, 0.f };
		vr::VRProperties()->SetProperty(m_ulPropertyContainer, vr::Prop_StatusDisplayTransform_Matrix34, &l_transform, sizeof(vr::HmdMatrix34_t), vr::k_unHmdMatrix34PropertyTag);
		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_Firmware_UpdateAvailable_Bool, false);
		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_Firmware_ManualUpdate_Bool, false);
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_Firmware_ManualUpdateURL_String, "https://developer.valvesoftware.com/wiki/SteamVR/HowTo_Update_Firmware");
		vr::VRProperties()->SetUint64Property(m_ulPropertyContainer, vr::Prop_HardwareRevision_Uint64, 2214720000);
		vr::VRProperties()->SetUint64Property(m_ulPropertyContainer, vr::Prop_FirmwareVersion_Uint64, 1541800000);
		vr::VRProperties()->SetUint64Property(m_ulPropertyContainer, vr::Prop_FPGAVersion_Uint64, 512);
		vr::VRProperties()->SetUint64Property(m_ulPropertyContainer, vr::Prop_VRCVersion_Uint64, 1514800000);
		vr::VRProperties()->SetUint64Property(m_ulPropertyContainer, vr::Prop_RadioVersion_Uint64, 1518800000);
		vr::VRProperties()->SetUint64Property(m_ulPropertyContainer, vr::Prop_DongleVersion_Uint64, 8933539758);
		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_DeviceCanPowerOff_Bool, true);
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_Firmware_ProgrammingTarget_String, GetSerialNumber().c_str());
		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_Firmware_ForceUpdateRequired_Bool, false);
		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_Identifiable_Bool, false);
		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_Firmware_RemindUpdate_Bool, false);
		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_HasDisplayComponent_Bool, false);
		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_HasCameraComponent_Bool, false);
		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_HasDriverDirectModeComponent_Bool, false);
		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_HasVirtualDisplayComponent_Bool, false);

		//icons
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceOff_String, "{htc}/icons/tracker_status_off.png");
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceSearching_String, "{htc}/icons/tracker_status_searching.gif");
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceSearchingAlert_String, "{htc}/icons/tracker_status_searching_alert.gif");
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceReady_String, "{htc}/icons/tracker_status_ready.png");
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceReadyAlert_String, "{htc}/icons/tracker_status_ready_alert.png");
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceNotReady_String, "{htc}/icons/tracker_status_error.png");
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceStandby_String, "{htc}/icons/tracker_status_standby.png");
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceAlertLow_String, "{htc}/icons/tracker_status_ready_low.png");
		// yes we want to explicitly fallthrough to vive case!, vive trackers can have input when POGO pins are connected to a peripheral.
		// the input bindings are only active when the tracker role is set to "vive_tracker_handed"/held_in_hand roles.
		[[fallthrough]];
	}
	case 4: //Vive
	case 5: //Vive no pinch
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/trackpad/touch", &m_handles[ALVR_INPUT_TRACKPAD_TOUCH]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/trackpad/click", &m_handles[ALVR_INPUT_TRACKPAD_CLICK]);
		vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/trackpad/x", &m_handles[ALVR_INPUT_TRACKPAD_X], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);
		vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/trackpad/y", &m_handles[ALVR_INPUT_TRACKPAD_Y], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedTwoSided);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/trigger/click", &m_handles[ALVR_INPUT_TRIGGER_CLICK]);
		vr::VRDriverInput()->CreateScalarComponent(m_ulPropertyContainer, "/input/trigger/value", &m_handles[ALVR_INPUT_TRIGGER_VALUE], vr::VRScalarType_Absolute, vr::VRScalarUnits_NormalizedOneSided);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/grip/click", &m_handles[ALVR_INPUT_GRIP_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/application_menu/click", &m_handles[ALVR_INPUT_APPLICATION_MENU_CLICK]);
		vr::VRDriverInput()->CreateBooleanComponent(m_ulPropertyContainer, "/input/system/click", &m_handles[ALVR_INPUT_SYSTEM_CLICK]);
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
	Debug("RemoteController::Deactivate\n");
	m_unObjectId = vr::k_unTrackedDeviceIndexInvalid;
}

void OvrController::EnterStandby()
{
}

void *OvrController::GetComponent(const char *pchComponentNameAndVersion)
{
	Debug("RemoteController::GetComponent. Name=%hs\n", pchComponentNameAndVersion);

	return NULL;
}

 void PowerOff()
{
}

/** debug request from a client */
 void OvrController::DebugRequest(const char * /*pchRequest*/, char *pchResponseBuffer, uint32_t unResponseBufferSize)
{
	if (unResponseBufferSize >= 1)
		pchResponseBuffer[0] = 0;
}

 vr::DriverPose_t OvrController::GetPose()
{

	 Debug("Controller%d getPose %lf %lf %lf\n", m_index, m_pose.vecPosition[0], m_pose.vecPosition[1], m_pose.vecPosition[2]);

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
	result.x = (float)(q1->w * q2->x + q1->x * q2->w + q1->y * q2->z - q1->z * q2->y);
	result.y = (float)(q1->w * q2->y - q1->x * q2->z + q1->y * q2->w + q1->z * q2->x);
	result.z = (float)(q1->w * q2->z + q1->x * q2->y - q1->y * q2->x + q1->z * q2->w);
	result.w = (float)(q1->w * q2->w - q1->x * q2->x - q1->y * q2->y - q1->z * q2->z);
	return result;
}
vr::HmdQuaternionf_t QuatMultiply(const vr::HmdQuaternionf_t* q1, const vr::HmdQuaternion_t* q2)
{
	vr::HmdQuaternionf_t result;
	result.x = (float)(q1->w * q2->x + q1->x * q2->w + q1->y * q2->z - q1->z * q2->y);
	result.y = (float)(q1->w * q2->y - q1->x * q2->z + q1->y * q2->w + q1->z * q2->x);
	result.z = (float)(q1->w * q2->z + q1->x * q2->y - q1->y * q2->x + q1->z * q2->w);
	result.w = (float)(q1->w * q2->w - q1->x * q2->x - q1->y * q2->y - q1->z * q2->z);
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

	if (!m_pose.deviceIsConnected) {
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
		m_pose.vecPosition[0] = info.controller[controllerIndex].boneRootPosition.x;
		m_pose.vecPosition[1] = info.controller[controllerIndex].boneRootPosition.y;
		m_pose.vecPosition[2] = info.controller[controllerIndex].boneRootPosition.z;
		
		if (info.controller[controllerIndex].flags & TrackingInfo::Controller::FLAG_CONTROLLER_LEFTHAND) {
			double bonePosFixer[3] = { 0.0,0.05,-0.05 };
			vr::HmdVector3d_t posFix = vrmath::quaternionRotateVector(m_pose.qRotation, bonePosFixer);
			m_pose.vecPosition[0] = info.controller[controllerIndex].boneRootPosition.x + posFix.v[0];
			m_pose.vecPosition[1] = info.controller[controllerIndex].boneRootPosition.y + posFix.v[1];
			m_pose.vecPosition[2] = info.controller[controllerIndex].boneRootPosition.z + posFix.v[2];
		}
		else {
			double bonePosFixer[3] = { 0.0,0.05,-0.05 };
			vr::HmdVector3d_t posFix = vrmath::quaternionRotateVector(m_pose.qRotation, bonePosFixer);
			m_pose.vecPosition[0] = info.controller[controllerIndex].boneRootPosition.x + posFix.v[0];
			m_pose.vecPosition[1] = info.controller[controllerIndex].boneRootPosition.y + posFix.v[1];
			m_pose.vecPosition[2] = info.controller[controllerIndex].boneRootPosition.z + posFix.v[2];
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

    // use cutoffs for velocity and acceleration to stop jitter when there is not a lot of movement
	float LinearVelocityMultiplier = Shape(Magnitude(info.controller[controllerIndex].linearVelocity), Settings::Instance().m_linearVelocityCutoff);
	float LinearAccelerationMultiplier = Shape(Magnitude(info.controller[controllerIndex].linearAcceleration), Settings::Instance().m_linearAccelerationCutoff);
	float AngularVelocityMultiplier = Shape(Magnitude(info.controller[controllerIndex].angularVelocity), Settings::Instance().m_angularVelocityCutoff * DEG_TO_RAD);
	float AngularAccelerationMultiplier = Shape(Magnitude(info.controller[controllerIndex].angularAcceleration), Settings::Instance().m_angularAccelerationCutoff * DEG_TO_RAD);

	m_pose.vecVelocity[0] = info.controller[controllerIndex].linearVelocity.x * LinearVelocityMultiplier;
	m_pose.vecVelocity[1] = info.controller[controllerIndex].linearVelocity.y * LinearVelocityMultiplier;
	m_pose.vecVelocity[2] = info.controller[controllerIndex].linearVelocity.z * LinearVelocityMultiplier;
	m_pose.vecAcceleration[0] = info.controller[controllerIndex].linearAcceleration.x * LinearAccelerationMultiplier;
	m_pose.vecAcceleration[1] = info.controller[controllerIndex].linearAcceleration.y * LinearAccelerationMultiplier;
	m_pose.vecAcceleration[2] = info.controller[controllerIndex].linearAcceleration.z * LinearAccelerationMultiplier;
	m_pose.vecAngularVelocity[0] = info.controller[controllerIndex].angularVelocity.x * AngularVelocityMultiplier;
	m_pose.vecAngularVelocity[1] = info.controller[controllerIndex].angularVelocity.y * AngularVelocityMultiplier;
	m_pose.vecAngularVelocity[2] = info.controller[controllerIndex].angularVelocity.z * AngularVelocityMultiplier;
	m_pose.vecAngularAcceleration[0] = info.controller[controllerIndex].angularAcceleration.x * AngularAccelerationMultiplier;
	m_pose.vecAngularAcceleration[1] = info.controller[controllerIndex].angularAcceleration.y * AngularAccelerationMultiplier;
	m_pose.vecAngularAcceleration[2] = info.controller[controllerIndex].angularAcceleration.z * AngularAccelerationMultiplier;
	
	
	
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
	

	Debug("CONTROLLER %d %f,%f,%f - %f,%f,%f\n", m_index, m_pose.vecVelocity[0], m_pose.vecVelocity[1], m_pose.vecVelocity[2], m_pose.vecAngularVelocity[0], m_pose.vecAngularVelocity[1], m_pose.vecAngularVelocity[2]);
	
	

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
	

	m_pose.poseTimeOffset = *m_poseTimeOffset;

	   

	auto& c = info.controller[controllerIndex];
	Debug("Controller%d %d %lu: %08llX %08X %f:%f\n", m_index,controllerIndex, (unsigned long)m_unObjectId, c.buttons, c.flags, c.trackpadPosition.x, c.trackpadPosition.y);

	if (c.flags & TrackingInfo::Controller::FLAG_CONTROLLER_OCULUS_HAND) {
		//m_pose.poseTimeOffset = 0.;
		float rotThumb = (c.boneRotations[alvrHandBone_Thumb0].z + c.boneRotations[alvrHandBone_Thumb0].y + c.boneRotations[alvrHandBone_Thumb1].z + c.boneRotations[alvrHandBone_Thumb1].y + c.boneRotations[alvrHandBone_Thumb2].z + c.boneRotations[alvrHandBone_Thumb2].y + c.boneRotations[alvrHandBone_Thumb3].z + c.boneRotations[alvrHandBone_Thumb3].y) * 0.67f;
		float rotIndex = (c.boneRotations[alvrHandBone_Index1].z + c.boneRotations[alvrHandBone_Index2].z + c.boneRotations[alvrHandBone_Index3].z) * 0.67f;
		float rotMiddle = (c.boneRotations[alvrHandBone_Middle1].z + c.boneRotations[alvrHandBone_Middle2].z + c.boneRotations[alvrHandBone_Middle3].z) * 0.67f;
		float rotRing = (c.boneRotations[alvrHandBone_Ring1].z + c.boneRotations[alvrHandBone_Ring2].z + c.boneRotations[alvrHandBone_Ring3].z) * 0.67f;
		float rotPinky = (c.boneRotations[alvrHandBone_Pinky1].z + c.boneRotations[alvrHandBone_Pinky2].z + c.boneRotations[alvrHandBone_Pinky3].z) * 0.67f;
		float grip = std::min({ rotMiddle,rotRing,rotPinky }) * 4.0f - 3.0f;

		// Currently only the index pinch seems to have a system gesture. Otherwise just make sure the pinching fingers have high confidence.
		bool registerIndexPinch = (c.handFingerConfidences & alvrIndexConfidence_High) && (c.handFingerConfidences & alvrThumbConfidence_High) && !(c.inputStateStatus & alvrInputStateHandStatus_SystemGestureProcessing);
		bool registerMiddlePinch = (c.handFingerConfidences & alvrMiddleConfidence_High) && (c.handFingerConfidences & alvrThumbConfidence_High);
		bool registerRingPinch = (c.handFingerConfidences & alvrRingConfidence_High) && (c.handFingerConfidences & alvrThumbConfidence_High);
		bool registerPinkyPinch = (c.handFingerConfidences & alvrPinkyConfidence_High) && (c.handFingerConfidences & alvrThumbConfidence_High);

		switch(Settings::Instance().m_controllerMode){
		case 0: //Oculus Rift
		case 6: //Oculus Quest
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_SYSTEM_CLICK], registerPinkyPinch && (c.inputStateStatus & alvrInputStateHandStatus_PinkyPinching) != 0, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_APPLICATION_MENU_CLICK], false, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_GRIP_CLICK], grip > 0.9f, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_GRIP_VALUE], grip, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_GRIP_TOUCH], grip > 0.7f, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_THUMB_REST_TOUCH], false, 0.0);
			if (!m_isLeftHand) {
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_A_CLICK], registerRingPinch && (c.inputStateStatus & alvrInputStateHandStatus_RingPinching) != 0, 0.0);
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_A_TOUCH], registerRingPinch && (c.inputStateStatus & alvrInputStateHandStatus_RingPinching) != 0, 0.0);
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_B_CLICK], registerMiddlePinch && (c.inputStateStatus & alvrInputStateHandStatus_MiddlePinching) != 0, 0.0);
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_B_TOUCH], registerMiddlePinch && (c.inputStateStatus & alvrInputStateHandStatus_MiddlePinching) != 0, 0.0);
			}
			else {
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_X_CLICK], registerRingPinch && (c.inputStateStatus & alvrInputStateHandStatus_RingPinching) != 0, 0.0);
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_X_TOUCH], registerRingPinch && (c.inputStateStatus & alvrInputStateHandStatus_RingPinching) != 0, 0.0);
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_Y_CLICK], registerMiddlePinch && (c.inputStateStatus & alvrInputStateHandStatus_MiddlePinching) != 0, 0.0);
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_Y_TOUCH], registerMiddlePinch && (c.inputStateStatus & alvrInputStateHandStatus_MiddlePinching) != 0, 0.0);
			}
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_JOYSTICK_CLICK], rotThumb > 0.9f, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_JOYSTICK_X], 0.0f, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_JOYSTICK_Y], 0.0f, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_JOYSTICK_TOUCH], rotThumb > 0.7f, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_BACK_CLICK], false, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_GUIDE_CLICK], false, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_START_CLICK], false, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRIGGER_CLICK], registerIndexPinch && (c.inputStateStatus & alvrInputStateHandStatus_IndexPinching) != 0, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRIGGER_VALUE], registerIndexPinch ? c.fingerPinchStrengths[alvrFingerPinch_Index] : 0.0, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRIGGER_TOUCH], registerIndexPinch && (c.fingerPinchStrengths[alvrFingerPinch_Index] > 0.7f), 0.0);
			break;
		case 1: //Oculus Rift no pinch
		case 7: //Oculus Quest no pinch
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_SYSTEM_CLICK], false, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_APPLICATION_MENU_CLICK], false, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_GRIP_CLICK], grip > 0.9f, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_GRIP_VALUE], grip, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_GRIP_TOUCH], grip > 0.7f, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_THUMB_REST_TOUCH], false, 0.0);
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
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_JOYSTICK_X], 0.0f, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_JOYSTICK_Y], 0.0f, 0.0);
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
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_GRIP_TOUCH], grip > 0.7f, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_GRIP_FORCE], grip - 1.0, 0.0);
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
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_GRIP_TOUCH], grip > 0.7f, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_GRIP_FORCE], grip - 1.0, 0.0);
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
		case 4:
		case 8: // vive tracker
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRACKPAD_TOUCH], rotThumb > 0.7f, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRACKPAD_CLICK], rotThumb > 0.9f, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRACKPAD_X], 0, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRACKPAD_Y], 0, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRIGGER_CLICK], rotIndex > 0.9f, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRIGGER_VALUE], rotIndex, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_GRIP_CLICK], grip > 0.9f, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_APPLICATION_MENU_CLICK], (c.inputStateStatus & alvrInputStateHandStatus_MiddlePinching) != 0, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_SYSTEM_CLICK], (c.inputStateStatus & alvrInputStateHandStatus_RingPinching) != 0, 0.0);
			break;
		case 5:
		case 9: // vive tracker
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRACKPAD_TOUCH], rotThumb > 0.7f, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRACKPAD_CLICK], rotThumb > 0.9f, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRACKPAD_X], 0, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRACKPAD_Y], 0, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRIGGER_CLICK], rotIndex > 0.9f, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRIGGER_VALUE], rotIndex, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_GRIP_CLICK], grip > 0.9f, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_APPLICATION_MENU_CLICK], false, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_SYSTEM_CLICK], false, 0.0);
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

		vr::HmdQuaternion_t boneFixer = HmdQuaternion_Init(0, 0, 0.924, -0.383);
		COPY4(c.boneRotations[alvrHandBone_WristRoot], m_boneTransform[HSB_Wrist].orientation);
		m_boneTransform[HSB_Wrist].orientation = QuatMultiply(&m_boneTransform[HSB_Wrist].orientation, &boneFixer);

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

		// Will use one of the existing poses from the implementation below instead for position data.
		//COPY3(c.boneRootPosition, m_boneTransform[HSB_Root].position);
		//COPY3(c.bonePositionsBase[alvrHandBone_WristRoot], m_boneTransform[HSB_Wrist].position);
		//COPY3(c.bonePositionsBase[alvrHandBone_Thumb0], m_boneTransform[HSB_Thumb0].position);
		//COPY3(c.bonePositionsBase[alvrHandBone_Thumb1], m_boneTransform[HSB_Thumb1].position);
		//COPY3(c.bonePositionsBase[alvrHandBone_Thumb2], m_boneTransform[HSB_Thumb2].position);
		//COPY3(c.bonePositionsBase[alvrHandBone_Thumb3], m_boneTransform[HSB_Thumb3].position);
		//COPY3(c.bonePositionsBase[alvrHandBone_Index1], m_boneTransform[HSB_IndexFinger1].position);
		//COPY3(c.bonePositionsBase[alvrHandBone_Index2], m_boneTransform[HSB_IndexFinger2].position);
		//COPY3(c.bonePositionsBase[alvrHandBone_Index3], m_boneTransform[HSB_IndexFinger3].position);
		//COPY3(c.bonePositionsBase[alvrHandBone_Middle1], m_boneTransform[HSB_MiddleFinger1].position);
		//COPY3(c.bonePositionsBase[alvrHandBone_Middle2], m_boneTransform[HSB_MiddleFinger2].position);
		//COPY3(c.bonePositionsBase[alvrHandBone_Middle3], m_boneTransform[HSB_MiddleFinger3].position);
		//COPY3(c.bonePositionsBase[alvrHandBone_Ring1], m_boneTransform[HSB_RingFinger1].position);
		//COPY3(c.bonePositionsBase[alvrHandBone_Ring2], m_boneTransform[HSB_RingFinger2].position);
		//COPY3(c.bonePositionsBase[alvrHandBone_Ring3], m_boneTransform[HSB_RingFinger3].position);
		//COPY3(c.bonePositionsBase[alvrHandBone_Pinky0], m_boneTransform[HSB_PinkyFinger0].position);
		//COPY3(c.bonePositionsBase[alvrHandBone_Pinky1], m_boneTransform[HSB_PinkyFinger1].position);
		//COPY3(c.bonePositionsBase[alvrHandBone_Pinky2], m_boneTransform[HSB_PinkyFinger2].position);
		//COPY3(c.bonePositionsBase[alvrHandBone_Pinky3], m_boneTransform[HSB_PinkyFinger3].position);

		// Use position data (and orientation for missing bones - index, middle and ring finger bone 0)
		// from the functions below.
		if (m_isLeftHand) {
			m_boneTransform[2].position = { -0.012083f, 0.028070f, 0.025050f, 1.f };
			m_boneTransform[3].position = { 0.040406f, 0.000000f, -0.000000f, 1.f };
			m_boneTransform[4].position = { 0.032517f, 0.000000f, 0.000000f, 1.f };

			m_boneTransform[6].position = { 0.000632f, 0.026866f, 0.015002f, 1.f };
			m_boneTransform[7].position = { 0.074204f, -0.005002f, 0.000234f, 1.f };
			m_boneTransform[8].position = { 0.043930f, -0.000000f, -0.000000f, 1.f };
			m_boneTransform[9].position = { 0.028695f, 0.000000f, 0.000000f, 1.f };

			m_boneTransform[11].position = { 0.002177f, 0.007120f, 0.016319f, 1.f };
			m_boneTransform[12].position = { 0.070953f, 0.000779f, 0.000997f, 1.f };
			m_boneTransform[13].position = { 0.043108f, 0.000000f, 0.000000f, 1.f };
			m_boneTransform[14].position = { 0.033266f, 0.000000f, 0.000000f, 1.f };

			m_boneTransform[16].position = { 0.000513f, -0.006545f, 0.016348f, 1.f };
			m_boneTransform[17].position = { 0.065876f, 0.001786f, 0.000693f, 1.f };
			m_boneTransform[18].position = { 0.040697f, 0.000000f, 0.000000f, 1.f };
			m_boneTransform[19].position = { 0.028747f, -0.000000f, -0.000000f, 1.f };

			m_boneTransform[21].position = { -0.002478f, -0.018981f, 0.015214f, 1.f };
			m_boneTransform[22].position = { 0.062878f, 0.002844f, 0.000332f, 1.f };
			m_boneTransform[23].position = { 0.030220f, 0.000000f, 0.000000f, 1.f };
			m_boneTransform[24].position = { 0.018187f, 0.000000f, 0.000000f, 1.f };

			m_boneTransform[6].orientation =  {0.644251f, 0.421979f , -0.478202f , 0.422133f};
			m_boneTransform[11].orientation = {0.546723f, 0.541277f , -0.442520f , 0.460749f};
			m_boneTransform[16].orientation = {0.516692f, 0.550144f , -0.495548f , 0.429888f};
		}
		else {
			m_boneTransform[2].position = { 0.012330f, 0.028661f, 0.025049f, 1.f };
			m_boneTransform[3].position = { -0.040406f, -0.000000f, 0.000000f, 1.f };
			m_boneTransform[4].position = { -0.032517f, -0.000000f, -0.000000f, 1.f };

			m_boneTransform[6].position = { -0.000632f, 0.026866f, 0.015002f, 1.f };
			m_boneTransform[7].position = { -0.074204f, 0.005002f, -0.000234f, 1.f };
			m_boneTransform[8].position = { -0.043930f, 0.000000f, 0.000000f, 1.f };
			m_boneTransform[9].position = { -0.028695f, -0.000000f, -0.000000f, 1.f };

			m_boneTransform[11].position = { -0.002177f, 0.007120f, 0.016319f, 1.f };
			m_boneTransform[12].position = { -0.070953f, -0.000779f, -0.000997f, 1.f };
			m_boneTransform[13].position = { -0.043108f, -0.000000f, -0.000000f, 1.f };
			m_boneTransform[14].position = { -0.033266f, -0.000000f, -0.000000f, 1.f };

			m_boneTransform[16].position = { -0.000513f, -0.006545f, 0.016348f, 1.f };
			m_boneTransform[17].position = { -0.065876f, -0.001786f, -0.000693f, 1.f };
			m_boneTransform[18].position = { -0.040697f, -0.000000f, -0.000000f, 1.f };
			m_boneTransform[19].position = { -0.028747f, 0.000000f, 0.000000f, 1.f };

			m_boneTransform[21].position = { 0.002478f, -0.018981f, 0.015214f, 1.f };
			m_boneTransform[22].position = { -0.062878f, -0.002844f, -0.000332f, 1.f };
			m_boneTransform[23].position = { -0.030220f, -0.000000f, -0.000000f, 1.f };
			m_boneTransform[24].position = { -0.018187f, -0.000000f, -0.000000f, 1.f };

			m_boneTransform[6].orientation =  {0.421833f, -0.643793f , 0.422458f , 0.478661f};
			m_boneTransform[11].orientation = {0.541874f, -0.547427f , 0.459996f , 0.441701f};
			m_boneTransform[16].orientation = {0.548983f, -0.519068f , 0.426914f , 0.496920f};
		}

		// Move the hand itself back to counteract the translation applied to the controller position. (more or less)
		float bonePosFixer[3] = { 0.025f, 0.f, 0.1f };
		if (!m_isLeftHand)
			bonePosFixer[0] = -bonePosFixer[0];
		m_boneTransform[HSB_Wrist].position.v[0] = m_boneTransform[HSB_Wrist].position.v[0] + bonePosFixer[0];
		m_boneTransform[HSB_Wrist].position.v[1] = m_boneTransform[HSB_Wrist].position.v[1] + bonePosFixer[1];
		m_boneTransform[HSB_Wrist].position.v[2] = m_boneTransform[HSB_Wrist].position.v[2] + bonePosFixer[2];

		// Rotate thumb0 and pinky0 properly.
		if (m_isLeftHand)
		{
			vr::HmdQuaternion_t fixer = HmdQuaternion_Init(0.5, 0.5, -0.5, 0.5);
			m_boneTransform[HSB_Thumb0].orientation = QuatMultiply(&fixer, &m_boneTransform[HSB_Thumb0].orientation);
			m_boneTransform[HSB_PinkyFinger0].orientation = QuatMultiply(&fixer, &m_boneTransform[HSB_PinkyFinger0].orientation);
		}
		else
		{
			vr::HmdQuaternion_t fixer = HmdQuaternion_Init(0.5, -0.5, 0.5, 0.5);
			m_boneTransform[HSB_Thumb0].orientation = QuatMultiply(&fixer, &m_boneTransform[HSB_Thumb0].orientation);
			m_boneTransform[HSB_PinkyFinger0].orientation = QuatMultiply(&fixer, &m_boneTransform[HSB_PinkyFinger0].orientation);
		}
		

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
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_GRIP_FORCE], (c.gripValue - 0.8) * 5.0, 0.0);
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

		case 4:
		case 5:
		case 8: // Vive Tracker
		case 9: // Vive Tracker (No Pinch)
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRACKPAD_TOUCH], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_JOYSTICK_TOUCH)) != 0, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRACKPAD_CLICK], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_JOYSTICK_CLICK)) != 0, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRACKPAD_X], c.trackpadPosition.x, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRACKPAD_Y], c.trackpadPosition.y, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_TRIGGER_CLICK], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_CLICK)) != 0, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_TRIGGER_VALUE], c.triggerValue, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_GRIP_CLICK], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_GRIP_CLICK)) != 0, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_SYSTEM_CLICK], (c.buttons& ALVR_BUTTON_FLAG(ALVR_INPUT_SYSTEM_CLICK)) != 0, 0.0);

			if (!m_isLeftHand) {
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_APPLICATION_MENU_CLICK], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_A_CLICK)) != 0, 0.0);
			}
			else {
				vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_APPLICATION_MENU_CLICK], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_X_CLICK)) != 0, 0.0);
			}
			break;

		case 0: //Oculus Rift
		case 1: //Oculus Rift no pinch
		case 6:	//Oculus Quest
		case 7:	//Oculus Quest no pinch
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_SYSTEM_CLICK], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_SYSTEM_CLICK)) != 0, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_APPLICATION_MENU_CLICK], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_APPLICATION_MENU_CLICK)) != 0, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_GRIP_CLICK], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_GRIP_CLICK)) != 0, 0.0);
			vr::VRDriverInput()->UpdateScalarComponent(m_handles[ALVR_INPUT_GRIP_VALUE], c.gripValue, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_GRIP_TOUCH], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_GRIP_TOUCH)) != 0, 0.0);
			vr::VRDriverInput()->UpdateBooleanComponent(m_handles[ALVR_INPUT_THUMB_REST_TOUCH], (c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_THUMB_REST_TOUCH)) != 0, 0.0);


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
				m_thumbAnimationProgress += 1.f / ANIMATION_FRAME_COUNT;
				if (m_thumbAnimationProgress > 1.f) {
					m_thumbAnimationProgress = 0;
					m_lastThumbTouch = currentThumbTouch;
				}
			}
			else {
				m_thumbAnimationProgress = 0;
			}

			uint64_t currentIndexTouch = c.buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_TOUCH);
			if (m_lastIndexTouch != currentIndexTouch) {
				m_indexAnimationProgress += 1.f / ANIMATION_FRAME_COUNT;
				if (m_indexAnimationProgress > 1.f) {
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
				Debug("UpdateSkeletonComponentfailed.  Error: %i\n", err);
			}


			GetBoneTransform(false, m_isLeftHand, m_thumbAnimationProgress, m_indexAnimationProgress, lastPoseTouch, c, boneTransforms);

			// Then update the WithoutController pose on the component 
			err = vr::VRDriverInput()->UpdateSkeletonComponent(m_compSkeleton, vr::VRSkeletalMotionRange_WithoutController, boneTransforms, SKELETON_BONE_COUNT);
			if (err != vr::VRInputError_None)
			{
				// Handle failure case
				Debug("UpdateSkeletonComponentfailed.  Error: %i\n", err);
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
				outBoneTransform[2] = { {-0.017303f, 0.032567f, 0.025281f, 1.f}, {0.317609f, 0.528344f , 0.213134f , 0.757991f} };
				outBoneTransform[3] = { {0.040406f, 0.000000f, -0.000000f, 1.f}, {0.991742f, 0.085317f , 0.019416f , 0.093765f} };
				outBoneTransform[4] = { {0.032517f, -0.000000f, 0.000000f, 1.f}, {0.959385f, -0.012202f , -0.031055f , 0.280120f} };
			}
			else {
				outBoneTransform[2] = { {-0.016426f, 0.030866f, 0.025118f, 1.f}, {0.403850f, 0.595704f , 0.082451f , 0.689380f} };
				outBoneTransform[3] = { {0.040406f, 0.000000f, -0.000000f, 1.f}, {0.989655f, -0.090426f , 0.028457f , 0.107691f} };
				outBoneTransform[4] = { {0.032517f, 0.000000f, 0.000000f, 1.f}, {0.988590f, 0.143978f , 0.041520f , 0.015363f} };
			}
		}
		else if ((buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_X_TOUCH)) != 0) {
			//x touch
			if (withController) {
				outBoneTransform[2] = { {-0.017625f, 0.031098f, 0.022755f, 1}, {0.388513f, 0.527438f , 0.249444f , 0.713193f} };
				outBoneTransform[3] = { {0.040406f, 0.000000f, -0.000000f, 1}, {0.978341f, 0.085924f , 0.037765f , 0.184501f} };
				outBoneTransform[4] = { {0.032517f, -0.000000f, 0.000000f, 1}, {0.894037f, -0.043820f , -0.048328f , 0.443217f} };
			}
			else {
				outBoneTransform[2] = { {-0.017288f, 0.027151f, 0.021465f, 1}, {0.502777f, 0.569978f , 0.147197f , 0.632988f} };
				outBoneTransform[3] = { {0.040406f, 0.000000f, -0.000000f, 1}, {0.970397f, -0.048119f , 0.023261f , 0.235527f} };
				outBoneTransform[4] = { {0.032517f, 0.000000f, 0.000000f, 1}, {0.794064f, 0.084451f , -0.037468f , 0.600772f} };
			}
		}
		else if ((buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_JOYSTICK_TOUCH)) != 0) {
			//joy touch
			if (withController) {
				outBoneTransform[2] = { {-0.017914f, 0.029178f, 0.025298f, 1}, {0.455126f, 0.591760f , 0.168152f , 0.643743f} };
				outBoneTransform[3] = { {0.040406f, 0.000000f, -0.000000f, 1}, {0.969878f, 0.084444f , 0.045679f , 0.223873f} };
				outBoneTransform[4] = { {0.032517f, -0.000000f, 0.000000f, 1}, {0.991257f, 0.014384f , -0.005602f , 0.131040f} };
			}
			else {
				outBoneTransform[2] = { {-0.017914f, 0.029178f, 0.025298f, 1}, {0.455126f, 0.591760f , 0.168152f , 0.643743f} };
				outBoneTransform[3] = { {0.040406f, 0.000000f, -0.000000f, 1}, {0.969878f, 0.084444f , 0.045679f , 0.223873f} };
				outBoneTransform[4] = { {0.032517f, -0.000000f, 0.000000f, 1}, {0.991257f, 0.014384f , -0.005602f , 0.131040f} };
			}
		}
		else {
			// no touch
			outBoneTransform[2] = { {-0.012083f, 0.028070f, 0.025050f, 1}, {0.464112f, 0.567418f , 0.272106f , 0.623374f} };
			outBoneTransform[3] = { {0.040406f, 0.000000f, -0.000000f, 1}, {0.994838f, 0.082939f , 0.019454f , 0.055130f} };
			outBoneTransform[4] = { {0.032517f, 0.000000f, 0.000000f, 1}, {0.974793f, -0.003213f , 0.021867f , -0.222015f} };
		}

		outBoneTransform[5] = { {0.030464f, -0.000000f, -0.000000f, 1}, {1.000000f, -0.000000f , 0.000000f , 0.000000f} };
	}
	else {
		if ((buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_B_TOUCH)) != 0) {
			//b touch
			if (withController) {
				outBoneTransform[2] = { {0.017303f, 0.032567f, 0.025281f, 1}, {0.528344f, -0.317609f , 0.757991f , -0.213134f} };
				outBoneTransform[3] = { {-0.040406f, -0.000000f, 0.000000f, 1}, {0.991742f, 0.085317f , 0.019416f , 0.093765f} };
				outBoneTransform[4] = { {-0.032517f, 0.000000f, -0.000000f, 1}, {0.959385f, -0.012202f , -0.031055f , 0.280120f} };
			}
			else {
				outBoneTransform[2] = { {0.016426f, 0.030866f, 0.025118f, 1}, {0.595704f, -0.403850f , 0.689380f , -0.082451f} };
				outBoneTransform[3] = { {-0.040406f, -0.000000f, 0.000000f, 1}, {0.989655f, -0.090426f , 0.028457f , 0.107691f} };
				outBoneTransform[4] = { {-0.032517f, -0.000000f, -0.000000f, 1}, {0.988590f, 0.143978f , 0.041520f , 0.015363f} };
			}
		}
		else if ((buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_A_TOUCH)) != 0) {
			//a touch
			if (withController) {
				outBoneTransform[2] = { {0.017625f, 0.031098f, 0.022755f, 1}, {0.527438f, -0.388513f , 0.713193f , -0.249444f} };
				outBoneTransform[3] = { {-0.040406f, -0.000000f, 0.000000f, 1}, {0.978341f, 0.085924f , 0.037765f , 0.184501f} };
				outBoneTransform[4] = { {-0.032517f, 0.000000f, -0.000000f, 1}, {0.894037f, -0.043820f , -0.048328f , 0.443217f} };
			}
			else {
				outBoneTransform[2] = { {0.017288f, 0.027151f, 0.021465f, 1}, {0.569978f, -0.502777f , 0.632988f , -0.147197f} };
				outBoneTransform[3] = { {-0.040406f, -0.000000f, 0.000000f, 1}, {0.970397f, -0.048119f , 0.023261f , 0.235527f} };
				outBoneTransform[4] = { {-0.032517f, -0.000000f, -0.000000f, 1}, {0.794064f, 0.084451f , -0.037468f , 0.600772f} };
			}
		}
		else if ((buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_JOYSTICK_TOUCH)) != 0) {
			//joy touch
			if (withController) {
				outBoneTransform[2] = { {0.017914f, 0.029178f, 0.025298f, 1}, {0.591760f, -0.455126f , 0.643743f , -0.168152f} };
				outBoneTransform[3] = { {-0.040406f, -0.000000f, 0.000000f, 1}, {0.969878f, 0.084444f , 0.045679f , 0.223873f} };
				outBoneTransform[4] = { {-0.032517f, 0.000000f, -0.000000f, 1}, {0.991257f, 0.014384f , -0.005602f , 0.131040f} };
			}
			else {
				outBoneTransform[2] = { {0.017914f, 0.029178f, 0.025298f, 1}, {0.591760f, -0.455126f , 0.643743f , -0.168152f} };
				outBoneTransform[3] = { {-0.040406f, -0.000000f, 0.000000f, 1}, {0.969878f, 0.084444f , 0.045679f , 0.223873f} };
				outBoneTransform[4] = { {-0.032517f, 0.000000f, -0.000000f, 1}, {0.991257f, 0.014384f , -0.005602f , 0.131040f} };
			}
		}
		else {
			// no touch
			outBoneTransform[2] = { {0.012330f, 0.028661f, 0.025049f, 1}, {0.571059f, -0.451277f , 0.630056f , -0.270685f} };
			outBoneTransform[3] = { {-0.040406f, -0.000000f, 0.000000f, 1}, {0.994565f, 0.078280f , 0.018282f , 0.066177f} };
			outBoneTransform[4] = { {-0.032517f, -0.000000f, -0.000000f, 1}, {0.977658f, -0.003039f , 0.020722f , -0.209156f} };
		}

		outBoneTransform[5] = { {-0.030464f, 0.000000f, 0.000000f, 1}, {1.000000f, -0.000000f , 0.000000f , 0.000000f} };
	}
}

void GetTriggerBoneTransform(bool withController, bool isLeftHand, uint64_t buttons, vr::VRBoneTransform_t outBoneTransform[]) {
	if ((buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_CLICK)) != 0) {
		// click
		if (withController) {
			if (isLeftHand) {
				outBoneTransform[6] = { {-0.003925f, 0.027171f, 0.014640f, 1}, {0.666448f, 0.430031f , -0.455947f , 0.403772f} };
				outBoneTransform[7] = { {0.076015f, -0.005124f, 0.000239f, 1}, {-0.956011f, -0.000025f , 0.158355f , -0.246913f} };
				outBoneTransform[8] = { {0.043930f, -0.000000f, -0.000000f, 1}, {-0.944138f, -0.043351f , 0.014947f , -0.326345f} };
				outBoneTransform[9] = { {0.028695f, 0.000000f, 0.000000f, 1}, {-0.912149f, 0.003626f , 0.039888f , -0.407898f} };
				outBoneTransform[10] = { {0.022821f, 0.000000f, -0.000000f, 1}, {1.000000f, -0.000000f , -0.000000f , 0.000000f} };
				outBoneTransform[11] = { {0.002177f, 0.007120f, 0.016319f, 1}, {0.529359f, 0.540512f , -0.463783f , 0.461011f} };
				outBoneTransform[12] = { {0.070953f, 0.000779f, 0.000997f, 1}, {0.847397f, -0.257141f , -0.139135f , 0.443213f} };
				outBoneTransform[13] = { {0.043108f, 0.000000f, 0.000000f, 1}, {0.874907f, 0.009875f , 0.026584f , 0.483460f} };
				outBoneTransform[14] = { {0.033266f, -0.000000f, 0.000000f, 1}, {0.894578f, -0.036774f , -0.050597f , 0.442513f} };
				outBoneTransform[15] = { {0.025892f, -0.000000f, 0.000000f, 1}, {0.999195f, -0.000000f , 0.000000f , 0.040126f} };
				outBoneTransform[16] = { {0.000513f, -0.006545f, 0.016348f, 1}, {0.500244f, 0.530784f , -0.516215f , 0.448939f} };
				outBoneTransform[17] = { {0.065876f, 0.001786f, 0.000693f, 1}, {0.831617f, -0.242931f , -0.139695f , 0.479461f} };
				outBoneTransform[18] = { {0.040697f, 0.000000f, 0.000000f, 1}, {0.769163f, -0.001746f , 0.001363f , 0.639049f} };
				outBoneTransform[19] = { {0.028747f, -0.000000f, -0.000000f, 1}, {0.968615f, -0.064538f , -0.046586f , 0.235477f} };
				outBoneTransform[20] = { {0.022430f, -0.000000f, 0.000000f, 1}, {1.000000f, 0.000000f , -0.000000f , -0.000000f} };
				outBoneTransform[21] = { {-0.002478f, -0.018981f, 0.015214f, 1}, {0.474671f, 0.434670f , -0.653212f , 0.398827f} };
				outBoneTransform[22] = { {0.062878f, 0.002844f, 0.000332f, 1}, {0.798788f, -0.199577f , -0.094418f , 0.559636f} };
				outBoneTransform[23] = { {0.030220f, 0.000002f, -0.000000f, 1}, {0.853087f, 0.001644f , -0.000913f , 0.521765f} };
				outBoneTransform[24] = { {0.018187f, -0.000002f, 0.000000f, 1}, {0.974249f, 0.052491f , 0.003591f , 0.219249f} };
				outBoneTransform[25] = { {0.018018f, 0.000000f, -0.000000f, 1}, {1.000000f, 0.000000f , 0.000000f , 0.000000f} };
				outBoneTransform[26] = { {0.006629f, 0.026690f, 0.061870f, 1}, {0.805084f, -0.018369f , 0.584788f , -0.097597f} };
				outBoneTransform[27] = { {-0.007882f, -0.040478f, 0.039337f, 1}, {-0.322494f, 0.932092f , 0.121861f , 0.111140f} };
				outBoneTransform[28] = { {0.017136f, -0.032633f, 0.080682f, 1}, {-0.169466f, 0.800083f , 0.571006f , 0.071415f} };
				outBoneTransform[29] = { {0.011144f, -0.028727f, 0.108366f, 1}, {-0.076328f, 0.788280f , 0.605097f , 0.081527f} };
				outBoneTransform[30] = { {0.011333f, -0.026044f, 0.128585f, 1}, {-0.144791f, 0.737451f , 0.656958f , -0.060069f} };
			}
			else {
				outBoneTransform[6] = { {-0.003925f, 0.027171f, 0.014640f, 1}, {0.666448f, 0.430031f , -0.455947f , 0.403772f} };
				outBoneTransform[7] = { {0.076015f, -0.005124f, 0.000239f, 1}, {-0.956011f, -0.000025f , 0.158355f , -0.246913f} };
				outBoneTransform[8] = { {0.043930f, -0.000000f, -0.000000f, 1}, {-0.944138f, -0.043351f , 0.014947f , -0.326345f} };
				outBoneTransform[9] = { {0.028695f, 0.000000f, 0.000000f, 1}, {-0.912149f, 0.003626f , 0.039888f , -0.407898f} };
				outBoneTransform[10] = { {0.022821f, 0.000000f, -0.000000f, 1}, {1.000000f, -0.000000f , -0.000000f , 0.000000f} };
				outBoneTransform[11] = { {0.002177f, 0.007120f, 0.016319f, 1}, {0.529359f, 0.540512f , -0.463783f , 0.461011f} };
				outBoneTransform[12] = { {0.070953f, 0.000779f, 0.000997f, 1}, {0.847397f, -0.257141f , -0.139135f , 0.443213f} };
				outBoneTransform[13] = { {0.043108f, 0.000000f, 0.000000f, 1}, {0.874907f, 0.009875f , 0.026584f , 0.483460f} };
				outBoneTransform[14] = { {0.033266f, -0.000000f, 0.000000f, 1}, {0.894578f, -0.036774f , -0.050597f , 0.442513f} };
				outBoneTransform[15] = { {0.025892f, -0.000000f, 0.000000f, 1}, {0.999195f, -0.000000f , 0.000000f , 0.040126f} };
				outBoneTransform[16] = { {0.000513f, -0.006545f, 0.016348f, 1}, {0.500244f, 0.530784f , -0.516215f , 0.448939f} };
				outBoneTransform[17] = { {0.065876f, 0.001786f, 0.000693f, 1}, {0.831617f, -0.242931f , -0.139695f , 0.479461f} };
				outBoneTransform[18] = { {0.040697f, 0.000000f, 0.000000f, 1}, {0.769163f, -0.001746f , 0.001363f , 0.639049f} };
				outBoneTransform[19] = { {0.028747f, -0.000000f, -0.000000f, 1}, {0.968615f, -0.064538f , -0.046586f , 0.235477f} };
				outBoneTransform[20] = { {0.022430f, -0.000000f, 0.000000f, 1}, {1.000000f, 0.000000f , -0.000000f , -0.000000f} };
				outBoneTransform[21] = { {-0.002478f, -0.018981f, 0.015214f, 1}, {0.474671f, 0.434670f , -0.653212f , 0.398827f} };
				outBoneTransform[22] = { {0.062878f, 0.002844f, 0.000332f, 1}, {0.798788f, -0.199577f , -0.094418f , 0.559636f} };
				outBoneTransform[23] = { {0.030220f, 0.000002f, -0.000000f, 1}, {0.853087f, 0.001644f , -0.000913f , 0.521765f} };
				outBoneTransform[24] = { {0.018187f, -0.000002f, 0.000000f, 1}, {0.974249f, 0.052491f , 0.003591f , 0.219249f} };
				outBoneTransform[25] = { {0.018018f, 0.000000f, -0.000000f, 1}, {1.000000f, 0.000000f , 0.000000f , 0.000000f} };
				outBoneTransform[26] = { {0.006629f, 0.026690f, 0.061870f, 1}, {0.805084f, -0.018369f , 0.584788f , -0.097597f} };
				outBoneTransform[27] = { {-0.007882f, -0.040478f, 0.039337f, 1}, {-0.322494f, 0.932092f , 0.121861f , 0.111140f} };
				outBoneTransform[28] = { {0.017136f, -0.032633f, 0.080682f, 1}, {-0.169466f, 0.800083f , 0.571006f , 0.071415f} };
				outBoneTransform[29] = { {0.011144f, -0.028727f, 0.108366f, 1}, {-0.076328f, 0.788280f , 0.605097f , 0.081527f} };
				outBoneTransform[30] = { {0.011333f, -0.026044f, 0.128585f, 1}, {-0.144791f, 0.737451f , 0.656958f , -0.060069f} };
			}
		}
		else {
			if (isLeftHand) {
				outBoneTransform[6] = { {0.003802f, 0.021514f, 0.012803f, 1}, {0.617314f, 0.395175f , -0.510874f , 0.449185f} };
				outBoneTransform[7] = { {0.074204f, -0.005002f, 0.000234f, 1}, {0.737291f, -0.032006f , -0.115013f , 0.664944f} };
				outBoneTransform[8] = { {0.043287f, -0.000000f, -0.000000f, 1}, {0.611381f, 0.003287f , 0.003823f , 0.791320f} };
				outBoneTransform[9] = { {0.028275f, 0.000000f, 0.000000f, 1}, {0.745389f, -0.000684f , -0.000945f , 0.666629f} };
				outBoneTransform[10] = { {0.022821f, 0.000000f, -0.000000f, 1}, {1.000000f, 0.000000f , -0.000000f , 0.000000f} };
				outBoneTransform[11] = { {0.004885f, 0.006885f, 0.016480f, 1}, {0.522678f, 0.527374f , -0.469333f , 0.477923f} };
				outBoneTransform[12] = { {0.070953f, 0.000779f, 0.000997f, 1}, {0.826071f, -0.121321f , 0.017267f , 0.550082f} };
				outBoneTransform[13] = { {0.043108f, 0.000000f, 0.000000f, 1}, {0.956676f, 0.013210f , 0.009330f , 0.290704f} };
				outBoneTransform[14] = { {0.033266f, 0.000000f, 0.000000f, 1}, {0.979740f, -0.001605f , -0.019412f , 0.199323f} };
				outBoneTransform[15] = { {0.025892f, -0.000000f, 0.000000f, 1}, {0.999195f, 0.000000f , 0.000000f , 0.040126f} };
				outBoneTransform[16] = { {0.001696f, -0.006648f, 0.016418f, 1}, {0.509620f, 0.540794f , -0.504891f , 0.439220f} };
				outBoneTransform[17] = { {0.065876f, 0.001786f, 0.000693f, 1}, {0.955009f, -0.065344f , -0.063228f , 0.282294f} };
				outBoneTransform[18] = { {0.040577f, 0.000000f, 0.000000f, 1}, {0.953823f, -0.000972f , 0.000697f , 0.300366f} };
				outBoneTransform[19] = { {0.028698f, -0.000000f, -0.000000f, 1}, {0.977627f, -0.001163f , -0.011433f , 0.210033f} };
				outBoneTransform[20] = { {0.022430f, -0.000000f, 0.000000f, 1}, {1.000000f, 0.000000f , 0.000000f , 0.000000f} };
				outBoneTransform[21] = { {-0.001792f, -0.019041f, 0.015254f, 1}, {0.518602f, 0.511152f , -0.596086f , 0.338315f} };
				outBoneTransform[22] = { {0.062878f, 0.002844f, 0.000332f, 1}, {0.978584f, -0.045398f , -0.103083f , 0.172297f} };
				outBoneTransform[23] = { {0.030154f, 0.000000f, 0.000000f, 1}, {0.970479f, -0.000068f , -0.002025f , 0.241175f} };
				outBoneTransform[24] = { {0.018187f, 0.000000f, 0.000000f, 1}, {0.997053f, -0.000687f , -0.052009f , -0.056395f} };
				outBoneTransform[25] = { {0.018018f, 0.000000f, -0.000000f, 1}, {1.000000f, -0.000000f , -0.000000f , -0.000000f} };
				outBoneTransform[26] = { {-0.005193f, 0.054191f, 0.060030f, 1}, {0.747374f, 0.182388f , 0.599615f , 0.220518f} };
				outBoneTransform[27] = { {0.000171f, 0.016473f, 0.096515f, 1}, {-0.006456f, 0.022747f , -0.932927f , -0.359287f} };
				outBoneTransform[28] = { {-0.038019f, -0.074839f, 0.046941f, 1}, {-0.199973f, 0.698334f , -0.635627f , -0.261380f} };
				outBoneTransform[29] = { {-0.036836f, -0.089774f, 0.081969f, 1}, {-0.191006f, 0.756582f , -0.607429f , -0.148761f} };
				outBoneTransform[30] = { {-0.030241f, -0.086049f, 0.119881f, 1}, {-0.019037f, 0.779368f , -0.612017f , -0.132881f} };
			}
			else {
				outBoneTransform[6] = { {-0.003802f, 0.021514f, 0.012803f, 1}, {0.395174f, -0.617314f , 0.449185f , 0.510874f} };
				outBoneTransform[7] = { {-0.074204f, 0.005002f, -0.000234f, 1}, {0.737291f, -0.032006f , -0.115013f , 0.664944f} };
				outBoneTransform[8] = { {-0.043287f, 0.000000f, 0.000000f, 1}, {0.611381f, 0.003287f , 0.003823f , 0.791320f} };
				outBoneTransform[9] = { {-0.028275f, -0.000000f, -0.000000f, 1}, {0.745389f, -0.000684f , -0.000945f , 0.666629f} };
				outBoneTransform[10] = { {-0.022821f, -0.000000f, 0.000000f, 1}, {1.000000f, 0.000000f , -0.000000f , 0.000000f} };
				outBoneTransform[11] = { {-0.004885f, 0.006885f, 0.016480f, 1}, {0.527233f, -0.522513f , 0.478085f , 0.469510f} };
				outBoneTransform[12] = { {-0.070953f, -0.000779f, -0.000997f, 1}, {0.826317f, -0.120120f , 0.019005f , 0.549918f} };
				outBoneTransform[13] = { {-0.043108f, -0.000000f, -0.000000f, 1}, {0.958363f, 0.013484f , 0.007380f , 0.285138f} };
				outBoneTransform[14] = { {-0.033266f, -0.000000f, -0.000000f, 1}, {0.977901f, -0.001431f , -0.018078f , 0.208279f} };
				outBoneTransform[15] = { {-0.025892f, 0.000000f, -0.000000f, 1}, {0.999195f, 0.000000f , 0.000000f , 0.040126f} };
				outBoneTransform[16] = { {-0.001696f, -0.006648f, 0.016418f, 1}, {0.541481f, -0.508179f , 0.441001f , 0.504054f} };
				outBoneTransform[17] = { {-0.065876f, -0.001786f, -0.000693f, 1}, {0.953780f, -0.064506f , -0.058812f , 0.287548f} };
				outBoneTransform[18] = { {-0.040577f, -0.000000f, -0.000000f, 1}, {0.954761f, -0.000983f , 0.000698f , 0.297372f} };
				outBoneTransform[19] = { {-0.028698f, 0.000000f, 0.000000f, 1}, {0.976924f, -0.001344f , -0.010281f , 0.213335f} };
				outBoneTransform[20] = { {-0.022430f, 0.000000f, -0.000000f, 1}, {1.000000f, 0.000000f , 0.000000f , 0.000000f} };
				outBoneTransform[21] = { {0.001792f, -0.019041f, 0.015254f, 1}, {0.510569f, -0.514906f , 0.341115f , 0.598191f} };
				outBoneTransform[22] = { {-0.062878f, -0.002844f, -0.000332f, 1}, {0.979195f, -0.043879f , -0.095103f , 0.173800f} };
				outBoneTransform[23] = { {-0.030154f, -0.000000f, -0.000000f, 1}, {0.971387f, -0.000102f , -0.002019f , 0.237494f} };
				outBoneTransform[24] = { {-0.018187f, -0.000000f, -0.000000f, 1}, {0.997961f, 0.000800f , -0.051911f , -0.037114f} };
				outBoneTransform[25] = { {-0.018018f, -0.000000f, 0.000000f, 1}, {1.000000f, -0.000000f , -0.000000f , -0.000000f} };
				outBoneTransform[26] = { {0.004392f, 0.055515f, 0.060253f, 1}, {0.745924f, 0.156756f , -0.597950f , -0.247953f} };
				outBoneTransform[27] = { {-0.000171f, 0.016473f, 0.096515f, 1}, {-0.006456f, 0.022747f , 0.932927f , 0.359287f} };
				outBoneTransform[28] = { {0.038119f, -0.074730f, 0.046338f, 1}, {-0.207931f, 0.699835f , 0.632631f , 0.258406f} };
				outBoneTransform[29] = { {0.035492f, -0.089519f, 0.081636f, 1}, {-0.197555f, 0.760574f , 0.601098f , 0.145535f} };
				outBoneTransform[30] = { {0.029073f, -0.085957f, 0.119561f, 1}, {-0.031423f, 0.791013f , 0.597190f , 0.129133f} };
			}
		}
	}
	else if ((buttons & ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_TOUCH)) != 0) {
		// touch
		if (withController) {
			if (isLeftHand) {
				outBoneTransform[6] = { {-0.003925f, 0.027171f, 0.014640f, 1}, {0.666448f, 0.430031f , -0.455947f , 0.403772f} };
				outBoneTransform[7] = { {0.074204f, -0.005002f, 0.000234f, 1}, {-0.951843f, 0.009717f , 0.158611f , -0.262188f} };
				outBoneTransform[8] = { {0.043930f, -0.000000f, -0.000000f, 1}, {-0.973045f, -0.044676f , 0.010341f , -0.226012f} };
				outBoneTransform[9] = { {0.028695f, 0.000000f, 0.000000f, 1}, {-0.935253f, -0.002881f , 0.023037f , -0.353217f} };
				outBoneTransform[10] = { {0.022821f, 0.000000f, -0.000000f, 1}, {1.000000f, -0.000000f , -0.000000f , 0.000000f} };
				outBoneTransform[11] = { {0.002177f, 0.007120f, 0.016319f, 1}, {0.529359f, 0.540512f , -0.463783f , 0.461011f} };
				outBoneTransform[12] = { {0.070953f, 0.000779f, 0.000997f, 1}, {0.847397f, -0.257141f , -0.139135f , 0.443213f} };
				outBoneTransform[13] = { {0.043108f, 0.000000f, 0.000000f, 1}, {0.874907f, 0.009875f , 0.026584f , 0.483460f} };
				outBoneTransform[14] = { {0.033266f, -0.000000f, 0.000000f, 1}, {0.894578f, -0.036774f , -0.050597f , 0.442513f} };
				outBoneTransform[15] = { {0.025892f, -0.000000f, 0.000000f, 1}, {0.999195f, -0.000000f , 0.000000f , 0.040126f} };
				outBoneTransform[16] = { {0.000513f, -0.006545f, 0.016348f, 1}, {0.500244f, 0.530784f , -0.516215f , 0.448939f} };
				outBoneTransform[17] = { {0.065876f, 0.001786f, 0.000693f, 1}, {0.831617f, -0.242931f , -0.139695f , 0.479461f} };
				outBoneTransform[18] = { {0.040697f, 0.000000f, 0.000000f, 1}, {0.769163f, -0.001746f , 0.001363f , 0.639049f} };
				outBoneTransform[19] = { {0.028747f, -0.000000f, -0.000000f, 1}, {0.968615f, -0.064538f , -0.046586f , 0.235477f} };
				outBoneTransform[20] = { {0.022430f, -0.000000f, 0.000000f, 1}, {1.000000f, 0.000000f , -0.000000f , -0.000000f} };
				outBoneTransform[21] = { {-0.002478f, -0.018981f, 0.015214f, 1}, {0.474671f, 0.434670f , -0.653212f , 0.398827f} };
				outBoneTransform[22] = { {0.062878f, 0.002844f, 0.000332f, 1}, {0.798788f, -0.199577f , -0.094418f , 0.559636f} };
				outBoneTransform[23] = { {0.030220f, 0.000002f, -0.000000f, 1}, {0.853087f, 0.001644f , -0.000913f , 0.521765f} };
				outBoneTransform[24] = { {0.018187f, -0.000002f, 0.000000f, 1}, {0.974249f, 0.052491f , 0.003591f , 0.219249f} };
				outBoneTransform[25] = { {0.018018f, 0.000000f, -0.000000f, 1}, {1.000000f, 0.000000f , 0.000000f , 0.000000f} };
				outBoneTransform[26] = { {0.006629f, 0.026690f, 0.061870f, 1}, {0.805084f, -0.018369f , 0.584788f , -0.097597f} };
				outBoneTransform[27] = { {-0.009005f, -0.041708f, 0.037992f, 1}, {-0.338860f, 0.939952f , -0.007564f , 0.040082f} };
				outBoneTransform[28] = { {0.017136f, -0.032633f, 0.080682f, 1}, {-0.169466f, 0.800083f , 0.571006f , 0.071415f} };
				outBoneTransform[29] = { {0.011144f, -0.028727f, 0.108366f, 1}, {-0.076328f, 0.788280f , 0.605097f , 0.081527f} };
				outBoneTransform[30] = { {0.011333f, -0.026044f, 0.128585f, 1}, {-0.144791f, 0.737451f , 0.656958f , -0.060069f} };
			}
			else {
				outBoneTransform[6] = { {-0.003925f, 0.027171f, 0.014640f, 1}, {0.666448f, 0.430031f , -0.455947f , 0.403772f} };
				outBoneTransform[7] = { {0.074204f, -0.005002f, 0.000234f, 1}, {-0.951843f, 0.009717f , 0.158611f , -0.262188f} };
				outBoneTransform[8] = { {0.043930f, -0.000000f, -0.000000f, 1}, {-0.973045f, -0.044676f , 0.010341f , -0.226012f} };
				outBoneTransform[9] = { {0.028695f, 0.000000f, 0.000000f, 1}, {-0.935253f, -0.002881f , 0.023037f , -0.353217f} };
				outBoneTransform[10] = { {0.022821f, 0.000000f, -0.000000f, 1}, {1.000000f, -0.000000f , -0.000000f , 0.000000f} };
				outBoneTransform[11] = { {0.002177f, 0.007120f, 0.016319f, 1}, {0.529359f, 0.540512f , -0.463783f , 0.461011f} };
				outBoneTransform[12] = { {0.070953f, 0.000779f, 0.000997f, 1}, {0.847397f, -0.257141f , -0.139135f , 0.443213f} };
				outBoneTransform[13] = { {0.043108f, 0.000000f, 0.000000f, 1}, {0.874907f, 0.009875f , 0.026584f , 0.483460f} };
				outBoneTransform[14] = { {0.033266f, -0.000000f, 0.000000f, 1}, {0.894578f, -0.036774f , -0.050597f , 0.442513f} };
				outBoneTransform[15] = { {0.025892f, -0.000000f, 0.000000f, 1}, {0.999195f, -0.000000f , 0.000000f , 0.040126f} };
				outBoneTransform[16] = { {0.000513f, -0.006545f, 0.016348f, 1}, {0.500244f, 0.530784f , -0.516215f , 0.448939f} };
				outBoneTransform[17] = { {0.065876f, 0.001786f, 0.000693f, 1}, {0.831617f, -0.242931f , -0.139695f , 0.479461f} };
				outBoneTransform[18] = { {0.040697f, 0.000000f, 0.000000f, 1}, {0.769163f, -0.001746f , 0.001363f , 0.639049f} };
				outBoneTransform[19] = { {0.028747f, -0.000000f, -0.000000f, 1}, {0.968615f, -0.064538f , -0.046586f , 0.235477f} };
				outBoneTransform[20] = { {0.022430f, -0.000000f, 0.000000f, 1}, {1.000000f, 0.000000f , -0.000000f , -0.000000f} };
				outBoneTransform[21] = { {-0.002478f, -0.018981f, 0.015214f, 1}, {0.474671f, 0.434670f , -0.653212f , 0.398827f} };
				outBoneTransform[22] = { {0.062878f, 0.002844f, 0.000332f, 1}, {0.798788f, -0.199577f , -0.094418f , 0.559636f} };
				outBoneTransform[23] = { {0.030220f, 0.000002f, -0.000000f, 1}, {0.853087f, 0.001644f , -0.000913f , 0.521765f} };
				outBoneTransform[24] = { {0.018187f, -0.000002f, 0.000000f, 1}, {0.974249f, 0.052491f , 0.003591f , 0.219249f} };
				outBoneTransform[25] = { {0.018018f, 0.000000f, -0.000000f, 1}, {1.000000f, 0.000000f , 0.000000f , 0.000000f} };
				outBoneTransform[26] = { {0.006629f, 0.026690f, 0.061870f, 1}, {0.805084f, -0.018369f , 0.584788f , -0.097597f} };
				outBoneTransform[27] = { {-0.009005f, -0.041708f, 0.037992f, 1}, {-0.338860f, 0.939952f , -0.007564f , 0.040082f} };
				outBoneTransform[28] = { {0.017136f, -0.032633f, 0.080682f, 1}, {-0.169466f, 0.800083f , 0.571006f , 0.071415f} };
				outBoneTransform[29] = { {0.011144f, -0.028727f, 0.108366f, 1}, {-0.076328f, 0.788280f , 0.605097f , 0.081527f} };
				outBoneTransform[30] = { {0.011333f, -0.026044f, 0.128585f, 1}, {-0.144791f, 0.737451f , 0.656958f , -0.060069f} };
			}
		}
		else {
			if (isLeftHand) {
				outBoneTransform[6] = { {0.002693f, 0.023387f, 0.013573f, 1}, {0.626743f, 0.404630f , -0.499840f , 0.440032f} };
				outBoneTransform[7] = { {0.074204f, -0.005002f, 0.000234f, 1}, {0.869067f, -0.019031f , -0.093524f , 0.485400f} };
				outBoneTransform[8] = { {0.043512f, -0.000000f, -0.000000f, 1}, {0.834068f, 0.020722f , 0.003930f , 0.551259f} };
				outBoneTransform[9] = { {0.028422f, 0.000000f, 0.000000f, 1}, {0.890556f, 0.000289f , -0.009290f , 0.454779f} };
				outBoneTransform[10] = { {0.022821f, 0.000000f, -0.000000f, 1}, {1.000000f, 0.000000f , -0.000000f , 0.000000f} };
				outBoneTransform[11] = { {0.003937f, 0.006967f, 0.016424f, 1}, {0.531603f, 0.532690f , -0.459598f , 0.471602f} };
				outBoneTransform[12] = { {0.070953f, 0.000779f, 0.000997f, 1}, {0.906933f, -0.142169f , -0.015445f , 0.396261f} };
				outBoneTransform[13] = { {0.043108f, 0.000000f, 0.000000f, 1}, {0.975787f, 0.014996f , 0.010867f , 0.217936f} };
				outBoneTransform[14] = { {0.033266f, 0.000000f, 0.000000f, 1}, {0.992777f, -0.002096f , -0.021403f , 0.118029f} };
				outBoneTransform[15] = { {0.025892f, -0.000000f, 0.000000f, 1}, {0.999195f, 0.000000f , 0.000000f , 0.040126f} };
				outBoneTransform[16] = { {0.001282f, -0.006612f, 0.016394f, 1}, {0.513688f, 0.543325f , -0.502550f , 0.434011f} };
				outBoneTransform[17] = { {0.065876f, 0.001786f, 0.000693f, 1}, {0.971280f, -0.068108f , -0.073480f , 0.215818f} };
				outBoneTransform[18] = { {0.040619f, 0.000000f, 0.000000f, 1}, {0.976566f, -0.001379f , 0.000441f , 0.215216f} };
				outBoneTransform[19] = { {0.028715f, -0.000000f, -0.000000f, 1}, {0.987232f, -0.000977f , -0.011919f , 0.158838f} };
				outBoneTransform[20] = { {0.022430f, -0.000000f, 0.000000f, 1}, {1.000000f, 0.000000f , 0.000000f , 0.000000f} };
				outBoneTransform[21] = { {-0.002032f, -0.019020f, 0.015240f, 1}, {0.521784f, 0.511917f , -0.594340f , 0.335325f} };
				outBoneTransform[22] = { {0.062878f, 0.002844f, 0.000332f, 1}, {0.982925f, -0.053050f , -0.108004f , 0.139206f} };
				outBoneTransform[23] = { {0.030177f, 0.000000f, 0.000000f, 1}, {0.979798f, 0.000394f , -0.001374f , 0.199982f} };
				outBoneTransform[24] = { {0.018187f, 0.000000f, 0.000000f, 1}, {0.997410f, -0.000172f , -0.051977f , -0.049724f} };
				outBoneTransform[25] = { {0.018018f, 0.000000f, -0.000000f, 1}, {1.000000f, -0.000000f , -0.000000f , -0.000000f} };
				outBoneTransform[26] = { {-0.004857f, 0.053377f, 0.060017f, 1}, {0.751040f, 0.174397f , 0.601473f , 0.209178f} };
				outBoneTransform[27] = { {-0.013234f, -0.004327f, 0.069740f, 1}, {-0.119277f, 0.262590f , -0.888979f , -0.355718f} };
				outBoneTransform[28] = { {-0.037500f, -0.074514f, 0.046899f, 1}, {-0.204942f, 0.706005f , -0.626220f , -0.259623f} };
				outBoneTransform[29] = { {-0.036251f, -0.089302f, 0.081732f, 1}, {-0.194045f, 0.764033f , -0.596592f , -0.150590f} };
				outBoneTransform[30] = { {-0.029633f, -0.085595f, 0.119439f, 1}, {-0.025015f, 0.787219f , -0.601140f , -0.135243f} };
			}
			else {
				outBoneTransform[6] = { {-0.002693f, 0.023387f, 0.013573f, 1}, {0.404698f, -0.626951f , 0.439894f , 0.499645f} };
				outBoneTransform[7] = { {-0.074204f, 0.005002f, -0.000234f, 1}, {0.870303f, -0.017421f , -0.092515f , 0.483436f} };
				outBoneTransform[8] = { {-0.043512f, 0.000000f, 0.000000f, 1}, {0.835972f, 0.018944f , 0.003312f , 0.548436f} };
				outBoneTransform[9] = { {-0.028422f, -0.000000f, -0.000000f, 1}, {0.890326f, 0.000173f , -0.008504f , 0.455244f} };
				outBoneTransform[10] = { {-0.022821f, -0.000000f, 0.000000f, 1}, {1.000000f, 0.000000f , -0.000000f , 0.000000f} };
				outBoneTransform[11] = { {-0.003937f, 0.006967f, 0.016424f, 1}, {0.532293f, -0.531137f , 0.472074f , 0.460113f} };
				outBoneTransform[12] = { {-0.070953f, -0.000779f, -0.000997f, 1}, {0.908154f, -0.139967f , -0.013210f , 0.394323f} };
				outBoneTransform[13] = { {-0.043108f, -0.000000f, -0.000000f, 1}, {0.977887f, 0.015350f , 0.008912f , 0.208378f} };
				outBoneTransform[14] = { {-0.033266f, -0.000000f, -0.000000f, 1}, {0.992487f, -0.002006f , -0.020888f , 0.120540f} };
				outBoneTransform[15] = { {-0.025892f, 0.000000f, -0.000000f, 1}, {0.999195f, 0.000000f , 0.000000f , 0.040126f} };
				outBoneTransform[16] = { {-0.001282f, -0.006612f, 0.016394f, 1}, {0.544460f, -0.511334f , 0.436935f , 0.501187f} };
				outBoneTransform[17] = { {-0.065876f, -0.001786f, -0.000693f, 1}, {0.971233f, -0.064561f , -0.071188f , 0.217877f} };
				outBoneTransform[18] = { {-0.040619f, -0.000000f, -0.000000f, 1}, {0.978211f, -0.001419f , 0.000451f , 0.207607f} };
				outBoneTransform[19] = { {-0.028715f, 0.000000f, 0.000000f, 1}, {0.987488f, -0.001166f , -0.010852f , 0.157314f} };
				outBoneTransform[20] = { {-0.022430f, 0.000000f, -0.000000f, 1}, {1.000000f, 0.000000f , 0.000000f , 0.000000f} };
				outBoneTransform[21] = { {0.002032f, -0.019020f, 0.015240f, 1}, {0.513640f, -0.518192f , 0.337332f , 0.594860f} };
				outBoneTransform[22] = { {-0.062878f, -0.002844f, -0.000332f, 1}, {0.983501f, -0.050059f , -0.104491f , 0.138930f} };
				outBoneTransform[23] = { {-0.030177f, -0.000000f, -0.000000f, 1}, {0.981170f, 0.000501f , -0.001363f , 0.193138f} };
				outBoneTransform[24] = { {-0.018187f, -0.000000f, -0.000000f, 1}, {0.997801f, 0.000487f , -0.051933f , -0.041173f} };
				outBoneTransform[25] = { {-0.018018f, -0.000000f, 0.000000f, 1}, {1.000000f, -0.000000f , -0.000000f , -0.000000f} };
				outBoneTransform[26] = { {0.004574f, 0.055518f, 0.060226f, 1}, {0.745334f, 0.161961f , -0.597782f , -0.246784f} };
				outBoneTransform[27] = { {0.013831f, -0.004360f, 0.069547f, 1}, {-0.117443f, 0.257604f , 0.890065f , 0.357255f} };
				outBoneTransform[28] = { {0.038220f, -0.074817f, 0.046428f, 1}, {-0.205767f, 0.697939f , 0.635107f , 0.259191f} };
				outBoneTransform[29] = { {0.035802f, -0.089658f, 0.081733f, 1}, {-0.196007f, 0.758396f , 0.604341f , 0.145564f} };
				outBoneTransform[30] = { {0.029364f, -0.086069f, 0.119701f, 1}, {-0.028444f, 0.787767f , 0.601616f , 0.129123f} };
			}
		}
	}
	else {
		// no touch
		if (isLeftHand) {
			outBoneTransform[6] = { {0.000632f, 0.026866f, 0.015002f, 1}, {0.644251f, 0.421979f , -0.478202f , 0.422133f} };
			outBoneTransform[7] = { {0.074204f, -0.005002f, 0.000234f, 1}, {0.995332f, 0.007007f , -0.039124f , 0.087949f} };
			outBoneTransform[8] = { {0.043930f, -0.000000f, -0.000000f, 1}, {0.997891f, 0.045808f , 0.002142f , -0.045943f} };
			outBoneTransform[9] = { {0.028695f, 0.000000f, 0.000000f, 1}, {0.999649f, 0.001850f , -0.022782f , -0.013409f} };
			outBoneTransform[10] = { {0.022821f, 0.000000f, -0.000000f, 1}, {1.000000f, 0.000000f , -0.000000f , 0.000000f} };
			outBoneTransform[11] = { {0.002177f, 0.007120f, 0.016319f, 1}, {0.546723f, 0.541277f , -0.442520f , 0.460749f} };
			outBoneTransform[12] = { {0.070953f, 0.000779f, 0.000997f, 1}, {0.980294f, -0.167261f , -0.078959f , 0.069368f} };
			outBoneTransform[13] = { {0.043108f, 0.000000f, 0.000000f, 1}, {0.997947f, 0.018493f , 0.013192f , 0.059886f} };
			outBoneTransform[14] = { {0.033266f, 0.000000f, 0.000000f, 1}, {0.997394f, -0.003328f , -0.028225f , -0.066315f} };
			outBoneTransform[15] = { {0.025892f, -0.000000f, 0.000000f, 1}, {0.999195f, 0.000000f , 0.000000f , 0.040126f} };
			outBoneTransform[16] = { {0.000513f, -0.006545f, 0.016348f, 1}, {0.516692f, 0.550144f , -0.495548f , 0.429888f} };
			outBoneTransform[17] = { {0.065876f, 0.001786f, 0.000693f, 1}, {0.990420f, -0.058696f , -0.101820f , 0.072495f} };
			outBoneTransform[18] = { {0.040697f, 0.000000f, 0.000000f, 1}, {0.999545f, -0.002240f , 0.000004f , 0.030081f} };
			outBoneTransform[19] = { {0.028747f, -0.000000f, -0.000000f, 1}, {0.999102f, -0.000721f , -0.012693f , 0.040420f} };
			outBoneTransform[20] = { {0.022430f, -0.000000f, 0.000000f, 1}, {1.000000f, 0.000000f , 0.000000f , 0.000000f} };
			outBoneTransform[21] = { {-0.002478f, -0.018981f, 0.015214f, 1}, {0.526918f, 0.523940f , -0.584025f , 0.326740f} };
			outBoneTransform[22] = { {0.062878f, 0.002844f, 0.000332f, 1}, {0.986609f, -0.059615f , -0.135163f , 0.069132f} };
			outBoneTransform[23] = { {0.030220f, 0.000000f, 0.000000f, 1}, {0.994317f, 0.001896f , -0.000132f , 0.106446f} };
			outBoneTransform[24] = { {0.018187f, 0.000000f, 0.000000f, 1}, {0.995931f, -0.002010f , -0.052079f , -0.073526f} };
			outBoneTransform[25] = { {0.018018f, 0.000000f, -0.000000f, 1}, {1.000000f, -0.000000f , -0.000000f , -0.000000f} };
			outBoneTransform[26] = { {-0.006059f, 0.056285f, 0.060064f, 1}, {0.737238f, 0.202745f , 0.594267f , 0.249441f} };
			outBoneTransform[27] = { {-0.040416f, -0.043018f, 0.019345f, 1}, {-0.290330f, 0.623527f , -0.663809f , -0.293734f} };
			outBoneTransform[28] = { {-0.039354f, -0.075674f, 0.047048f, 1}, {-0.187047f, 0.678062f , -0.659285f , -0.265683f} };
			outBoneTransform[29] = { {-0.038340f, -0.090987f, 0.082579f, 1}, {-0.183037f, 0.736793f , -0.634757f , -0.143936f} };
			outBoneTransform[30] = { {-0.031806f, -0.087214f, 0.121015f, 1}, {-0.003659f, 0.758407f , -0.639342f , -0.126678f} };
		}
		else {
			outBoneTransform[6] = { {-0.000632f, 0.026866f, 0.015002f, 1}, {0.421833f, -0.643793f , 0.422458f , 0.478661f} };
			outBoneTransform[7] = { {-0.074204f, 0.005002f, -0.000234f, 1}, {0.994784f, 0.007053f , -0.041286f , 0.093009f} };
			outBoneTransform[8] = { {-0.043930f, 0.000000f, 0.000000f, 1}, {0.998404f, 0.045905f , 0.002780f , -0.032767f} };
			outBoneTransform[9] = { {-0.028695f, -0.000000f, -0.000000f, 1}, {0.999704f, 0.001955f , -0.022774f , -0.008282f} };
			outBoneTransform[10] = { {-0.022821f, -0.000000f, 0.000000f, 1}, {1.000000f, 0.000000f , -0.000000f , 0.000000f} };
			outBoneTransform[11] = { {-0.002177f, 0.007120f, 0.016319f, 1}, {0.541874f, -0.547427f , 0.459996f , 0.441701f} };
			outBoneTransform[12] = { {-0.070953f, -0.000779f, -0.000997f, 1}, {0.979837f, -0.168061f , -0.075910f , 0.076899f} };
			outBoneTransform[13] = { {-0.043108f, -0.000000f, -0.000000f, 1}, {0.997271f, 0.018278f , 0.013375f , 0.070266f} };
			outBoneTransform[14] = { {-0.033266f, -0.000000f, -0.000000f, 1}, {0.998402f, -0.003143f , -0.026423f , -0.049849f} };
			outBoneTransform[15] = { {-0.025892f, 0.000000f, -0.000000f, 1}, {0.999195f, 0.000000f , 0.000000f , 0.040126f} };
			outBoneTransform[16] = { {-0.000513f, -0.006545f, 0.016348f, 1}, {0.548983f, -0.519068f , 0.426914f , 0.496920f} };
			outBoneTransform[17] = { {-0.065876f, -0.001786f, -0.000693f, 1}, {0.989791f, -0.065882f , -0.096417f , 0.081716f} };
			outBoneTransform[18] = { {-0.040697f, -0.000000f, -0.000000f, 1}, {0.999102f, -0.002168f , -0.000020f , 0.042317f} };
			outBoneTransform[19] = { {-0.028747f, 0.000000f, 0.000000f, 1}, {0.998584f, -0.000674f , -0.012714f , 0.051653f} };
			outBoneTransform[20] = { {-0.022430f, 0.000000f, -0.000000f, 1}, {1.000000f, 0.000000f , 0.000000f , 0.000000f} };
			outBoneTransform[21] = { {0.002478f, -0.018981f, 0.015214f, 1}, {0.518597f, -0.527304f , 0.328264f , 0.587580f} };
			outBoneTransform[22] = { {-0.062878f, -0.002844f, -0.000332f, 1}, {0.987294f, -0.063356f , -0.125964f , 0.073274f} };
			outBoneTransform[23] = { {-0.030220f, -0.000000f, -0.000000f, 1}, {0.993413f, 0.001573f , -0.000147f , 0.114578f} };
			outBoneTransform[24] = { {-0.018187f, -0.000000f, -0.000000f, 1}, {0.997047f, -0.000695f , -0.052009f , -0.056495f} };
			outBoneTransform[25] = { {-0.018018f, -0.000000f, 0.000000f, 1}, {1.000000f, -0.000000f , -0.000000f , -0.000000f} };
			outBoneTransform[26] = { {0.005198f, 0.054204f, 0.060030f, 1}, {0.747318f, 0.182508f , -0.599586f , -0.220688f} };
			outBoneTransform[27] = { {0.038779f, -0.042973f, 0.019824f, 1}, {-0.297445f, 0.639373f , 0.648910f , 0.285734f} };
			outBoneTransform[28] = { {0.038027f, -0.074844f, 0.046941f, 1}, {-0.199898f, 0.698218f , 0.635767f , 0.261406f} };
			outBoneTransform[29] = { {0.036845f, -0.089781f, 0.081973f, 1}, {-0.190960f, 0.756469f , 0.607591f , 0.148733f} };
			outBoneTransform[30] = { {0.030251f, -0.086056f, 0.119887f, 1}, {-0.018948f, 0.779249f , 0.612180f , 0.132846f} };
		}
	}
}

void GetGripClickBoneTransform(bool withController, bool isLeftHand, vr::VRBoneTransform_t outBoneTransform[]) {
	if (withController) {
		if (isLeftHand) {
			outBoneTransform[11] = { {0.002177f, 0.007120f, 0.016319f, 1}, {0.529359f, 0.540512f , -0.463783f , 0.461011f} };
			outBoneTransform[12] = { {0.070953f, 0.000779f, 0.000997f, 1}, {-0.831727f, 0.270927f , 0.175647f , -0.451638f} };
			outBoneTransform[13] = { {0.043108f, 0.000000f, 0.000000f, 1}, {-0.854886f, -0.008231f , -0.028107f , -0.517990f} };
			outBoneTransform[14] = { {0.033266f, -0.000000f, 0.000000f, 1}, {-0.825759f, 0.085208f , 0.086456f , -0.550805f} };
			outBoneTransform[15] = { {0.025892f, -0.000000f, 0.000000f, 1}, {0.999195f, -0.000000f , 0.000000f , 0.040126f} };
			outBoneTransform[16] = { {0.000513f, -0.006545f, 0.016348f, 1}, {0.500244f, 0.530784f , -0.516215f , 0.448939f} };
			outBoneTransform[17] = { {0.065876f, 0.001786f, 0.000693f, 1}, {0.831617f, -0.242931f , -0.139695f , 0.479461f} };
			outBoneTransform[18] = { {0.040697f, 0.000000f, 0.000000f, 1}, {0.769163f, -0.001746f , 0.001363f , 0.639049f} };
			outBoneTransform[19] = { {0.028747f, -0.000000f, -0.000000f, 1}, {0.968615f, -0.064537f , -0.046586f , 0.235477f} };
			outBoneTransform[20] = { {0.022430f, -0.000000f, 0.000000f, 1}, {1.000000f, 0.000000f , -0.000000f , -0.000000f} };
			outBoneTransform[21] = { {-0.002478f, -0.018981f, 0.015214f, 1}, {0.474671f, 0.434670f , -0.653212f , 0.398827f} };
			outBoneTransform[22] = { {0.062878f, 0.002844f, 0.000332f, 1}, {0.798788f, -0.199577f , -0.094418f , 0.559636f} };
			outBoneTransform[23] = { {0.030220f, 0.000002f, -0.000000f, 1}, {0.853087f, 0.001644f , -0.000913f , 0.521765f} };
			outBoneTransform[24] = { {0.018187f, -0.000002f, 0.000000f, 1}, {0.974249f, 0.052491f , 0.003591f , 0.219249f} };
			outBoneTransform[25] = { {0.018018f, 0.000000f, -0.000000f, 1}, {1.000000f, 0.000000f , 0.000000f , 0.000000f} };

			outBoneTransform[28] = { {0.016642f, -0.029992f, 0.083200f, 1}, {-0.094577f, 0.694550f , 0.702845f , 0.121100f} };
			outBoneTransform[29] = { {0.011144f, -0.028727f, 0.108366f, 1}, {-0.076328f, 0.788280f , 0.605097f , 0.081527f} };
			outBoneTransform[30] = { {0.011333f, -0.026044f, 0.128585f, 1}, {-0.144791f, 0.737451f , 0.656958f , -0.060069f} };
		}
		else {
			outBoneTransform[11] = { {0.002177f, 0.007120f, 0.016319f, 1}, {0.529359f, 0.540512f , -0.463783f , 0.461011f} };
			outBoneTransform[12] = { {0.070953f, 0.000779f, 0.000997f, 1}, {-0.831727f, 0.270927f , 0.175647f , -0.451638f} };
			outBoneTransform[13] = { {0.043108f, 0.000000f, 0.000000f, 1}, {-0.854886f, -0.008231f , -0.028107f , -0.517990f} };
			outBoneTransform[14] = { {0.033266f, -0.000000f, 0.000000f, 1}, {-0.825759f, 0.085208f , 0.086456f , -0.550805f} };
			outBoneTransform[15] = { {0.025892f, -0.000000f, 0.000000f, 1}, {0.999195f, -0.000000f , 0.000000f , 0.040126f} };
			outBoneTransform[16] = { {0.000513f, -0.006545f, 0.016348f, 1}, {0.500244f, 0.530784f , -0.516215f , 0.448939f} };
			outBoneTransform[17] = { {0.065876f, 0.001786f, 0.000693f, 1}, {0.831617f, -0.242931f , -0.139695f , 0.479461f} };
			outBoneTransform[18] = { {0.040697f, 0.000000f, 0.000000f, 1}, {0.769163f, -0.001746f , 0.001363f , 0.639049f} };
			outBoneTransform[19] = { {0.028747f, -0.000000f, -0.000000f, 1}, {0.968615f, -0.064537f , -0.046586f , 0.235477f} };
			outBoneTransform[20] = { {0.022430f, -0.000000f, 0.000000f, 1}, {1.000000f, 0.000000f , -0.000000f , -0.000000f} };
			outBoneTransform[21] = { {-0.002478f, -0.018981f, 0.015214f, 1}, {0.474671f, 0.434670f , -0.653212f , 0.398827f} };
			outBoneTransform[22] = { {0.062878f, 0.002844f, 0.000332f, 1}, {0.798788f, -0.199577f , -0.094418f , 0.559636f} };
			outBoneTransform[23] = { {0.030220f, 0.000002f, -0.000000f, 1}, {0.853087f, 0.001644f , -0.000913f , 0.521765f} };
			outBoneTransform[24] = { {0.018187f, -0.000002f, 0.000000f, 1}, {0.974249f, 0.052491f , 0.003591f , 0.219249f} };
			outBoneTransform[25] = { {0.018018f, 0.000000f, -0.000000f, 1}, {1.000000f, 0.000000f , 0.000000f , 0.000000f} };

			outBoneTransform[28] = { {0.016642f, -0.029992f, 0.083200f, 1}, {-0.094577f, 0.694550f , 0.702845f , 0.121100f} };
			outBoneTransform[29] = { {0.011144f, -0.028727f, 0.108366f, 1}, {-0.076328f, 0.788280f , 0.605097f , 0.081527f} };
			outBoneTransform[30] = { {0.011333f, -0.026044f, 0.128585f, 1}, {-0.144791f, 0.737451f , 0.656958f , -0.060069f} };
		}

	}
	else {
		if (isLeftHand) {
			outBoneTransform[11] = { {0.005787f, 0.006806f, 0.016534f, 1}, {0.514203f, 0.522315f , -0.478348f , 0.483700f} };
			outBoneTransform[12] = { {0.070953f, 0.000779f, 0.000997f, 1}, {0.723653f, -0.097901f , 0.048546f , 0.681458f} };
			outBoneTransform[13] = { {0.043108f, 0.000000f, 0.000000f, 1}, {0.637464f, -0.002366f , -0.002831f , 0.770472f} };
			outBoneTransform[14] = { {0.033266f, 0.000000f, 0.000000f, 1}, {0.658008f, 0.002610f , 0.003196f , 0.753000f} };
			outBoneTransform[15] = { {0.025892f, -0.000000f, 0.000000f, 1}, {0.999195f, 0.000000f , 0.000000f , 0.040126f} };
			outBoneTransform[16] = { {0.004123f, -0.006858f, 0.016563f, 1}, {0.489609f, 0.523374f , -0.520644f , 0.463997f} };
			outBoneTransform[17] = { {0.065876f, 0.001786f, 0.000693f, 1}, {0.759970f, -0.055609f , 0.011571f , 0.647471f} };
			outBoneTransform[18] = { {0.040331f, 0.000000f, 0.000000f, 1}, {0.664315f, 0.001595f , 0.001967f , 0.747449f} };
			outBoneTransform[19] = { {0.028489f, -0.000000f, -0.000000f, 1}, {0.626957f, -0.002784f , -0.003234f , 0.779042f} };
			outBoneTransform[20] = { {0.022430f, -0.000000f, 0.000000f, 1}, {1.000000f, 0.000000f , 0.000000f , 0.000000f} };
			outBoneTransform[21] = { {0.001131f, -0.019295f, 0.015429f, 1}, {0.479766f, 0.477833f , -0.630198f , 0.379934f} };
			outBoneTransform[22] = { {0.062878f, 0.002844f, 0.000332f, 1}, {0.827001f, 0.034282f , 0.003440f , 0.561144f} };
			outBoneTransform[23] = { {0.029874f, 0.000000f, 0.000000f, 1}, {0.702185f, -0.006716f , -0.009289f , 0.711903f} };
			outBoneTransform[24] = { {0.017979f, 0.000000f, 0.000000f, 1}, {0.676853f, 0.007956f , 0.009917f , 0.736009f} };
			outBoneTransform[25] = { {0.018018f, 0.000000f, -0.000000f, 1}, {1.000000f, -0.000000f , -0.000000f , -0.000000f} };

			outBoneTransform[28] = { {0.000448f, 0.001536f, 0.116543f, 1}, {-0.039357f, 0.105143f , -0.928833f , -0.353079f} };
			outBoneTransform[29] = { {0.003949f, -0.014869f, 0.130608f, 1}, {-0.055071f, 0.068695f , -0.944016f , -0.317933f} };
			outBoneTransform[30] = { {0.003263f, -0.034685f, 0.139926f, 1}, {0.019690f, -0.100741f , -0.957331f , -0.270149f} };
		}
		else {
			outBoneTransform[11] = { {-0.005787f, 0.006806f, 0.016534f, 1}, {0.522315f, -0.514203f , 0.483700f , 0.478348f} };
			outBoneTransform[12] = { {-0.070953f, -0.000779f, -0.000997f, 1}, {0.723653f, -0.097901f , 0.048546f , 0.681458f} };
			outBoneTransform[13] = { {-0.043108f, -0.000000f, -0.000000f, 1}, {0.637464f, -0.002366f , -0.002831f , 0.770472f} };
			outBoneTransform[14] = { {-0.033266f, -0.000000f, -0.000000f, 1}, {0.658008f, 0.002610f , 0.003196f , 0.753000f} };
			outBoneTransform[15] = { {-0.025892f, 0.000000f, -0.000000f, 1}, {0.999195f, 0.000000f , 0.000000f , 0.040126f} };
			outBoneTransform[16] = { {-0.004123f, -0.006858f, 0.016563f, 1}, {0.523374f, -0.489609f , 0.463997f , 0.520644f} };
			outBoneTransform[17] = { {-0.065876f, -0.001786f, -0.000693f, 1}, {0.759970f, -0.055609f , 0.011571f , 0.647471f} };
			outBoneTransform[18] = { {-0.040331f, -0.000000f, -0.000000f, 1}, {0.664315f, 0.001595f , 0.001967f , 0.747449f} };
			outBoneTransform[19] = { {-0.028489f, 0.000000f, 0.000000f, 1}, {0.626957f, -0.002784f , -0.003234f , 0.779042f} };
			outBoneTransform[20] = { {-0.022430f, 0.000000f, -0.000000f, 1}, {1.000000f, 0.000000f , 0.000000f , 0.000000f} };
			outBoneTransform[21] = { {-0.001131f, -0.019295f, 0.015429f, 1}, {0.477833f, -0.479766f , 0.379935f , 0.630198f} };
			outBoneTransform[22] = { {-0.062878f, -0.002844f, -0.000332f, 1}, {0.827001f, 0.034282f , 0.003440f , 0.561144f} };
			outBoneTransform[23] = { {-0.029874f, -0.000000f, -0.000000f, 1}, {0.702185f, -0.006716f , -0.009289f , 0.711903f} };
			outBoneTransform[24] = { {-0.017979f, -0.000000f, -0.000000f, 1}, {0.676853f, 0.007956f , 0.009917f , 0.736009f} };
			outBoneTransform[25] = { {-0.018018f, -0.000000f, 0.000000f, 1}, {1.000000f, -0.000000f , -0.000000f , -0.000000f} };

			outBoneTransform[28] = { {-0.000448f, 0.001536f, 0.116543f, 1}, {-0.039357f, 0.105143f , 0.928833f , 0.353079f} };
			outBoneTransform[29] = { {-0.003949f, -0.014869f, 0.130608f, 1}, {-0.055071f, 0.068695f , 0.944016f , 0.317933f} };
			outBoneTransform[30] = { {-0.003263f, -0.034685f, 0.139926f, 1}, {0.019690f, -0.100741f , 0.957331f , 0.270149f} };
		}
	}
}

void OvrController::GetBoneTransform(bool withController, bool isLeftHand, float thumbAnimationProgress, float indexAnimationProgress, uint64_t lastPoseButtons, const TrackingInfo::Controller& c, vr::VRBoneTransform_t outBoneTransform[]) {

	vr::VRBoneTransform_t boneTransform1[SKELETON_BONE_COUNT];
	vr::VRBoneTransform_t boneTransform2[SKELETON_BONE_COUNT];

	// root and wrist
	outBoneTransform[0] = { {0.000000f, 0.000000f, 0.000000f, 1}, {1.000000f, -0.000000f , -0.000000f , 0.000000f} };
	if (isLeftHand) {
		outBoneTransform[1] = { {-0.034038f, 0.036503f, 0.164722f, 1}, {-0.055147f, -0.078608f , -0.920279f , 0.379296f} };
	}
	else {
		outBoneTransform[1] = { {0.034038f, 0.036503f, 0.164722f, 1}, {-0.055147f, -0.078608f , 0.920279f , -0.379296f} };
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
