#include "OvrViveTrackerProxy.h"
#include "OvrHMD.h"

#include <cassert>

OvrViveTrackerProxy::OvrViveTrackerProxy(OvrHmd& owner)
:   m_unObjectId(vr::k_unTrackedDeviceIndexInvalid),
    m_HMDOwner(&owner)
{}

vr::DriverPose_t OvrViveTrackerProxy::GetPose()
{
    assert(m_HMDOwner != nullptr);
    return m_HMDOwner->GetPose();
}

vr::EVRInitError OvrViveTrackerProxy::Activate( vr::TrackedDeviceIndex_t unObjectId )
{
    m_unObjectId = unObjectId;
    assert(m_unObjectId != vr::k_unTrackedDeviceIndexInvalid);
	const auto propertyContainer = vr::VRProperties()->TrackedDeviceToPropertyContainer( m_unObjectId );

    // Normally a vive tracker emulator would (logically) always set the tracking system to "lighthouse" but in order to do space calibration
    // with existing tools such as OpenVR Space calibrator and be able to calibrate to/from ALVR HMD (and the proxy tracker) space to/from
    // a native HMD/tracked device which is already using "lighthouse" as the tracking system the proxy tracker needs to be in a different
    // tracking system to treat them differently and prevent those tools doing the same space transform to the proxy tracker.
    vr::VRProperties()->SetStringProperty(propertyContainer, vr::Prop_TrackingSystemName_String, Settings::Instance().mTrackingSystemName.c_str());//"lighthouse");
    vr::VRProperties()->SetStringProperty(propertyContainer, vr::Prop_ModelNumber_String, "Vive Tracker Pro MV");
    vr::VRProperties()->SetStringProperty(propertyContainer, vr::Prop_SerialNumber_String, GetSerialNumber()); // Changed
    vr::VRProperties()->SetStringProperty(propertyContainer, vr::Prop_RenderModelName_String, "{htc}vr_tracker_vive_1_0");
    vr::VRProperties()->SetBoolProperty(propertyContainer, vr::Prop_WillDriftInYaw_Bool, false);
    vr::VRProperties()->SetStringProperty(propertyContainer, vr::Prop_ManufacturerName_String, "HTC");
    vr::VRProperties()->SetStringProperty(propertyContainer, vr::Prop_TrackingFirmwareVersion_String, "1541800000 RUNNER-WATCHMAN$runner-watchman@runner-watchman 2018-01-01 FPGA 512(2.56/0/0) BL 0 VRC 1541800000 Radio 1518800000"); // Changed
    vr::VRProperties()->SetStringProperty(propertyContainer, vr::Prop_HardwareRevision_String, "product 128 rev 2.5.6 lot 2000/0/0 0"); // Changed
    vr::VRProperties()->SetStringProperty(propertyContainer, vr::Prop_ConnectedWirelessDongle_String, "D0000BE000"); // Changed
    vr::VRProperties()->SetBoolProperty(propertyContainer, vr::Prop_DeviceIsWireless_Bool, true);
    vr::VRProperties()->SetBoolProperty(propertyContainer, vr::Prop_DeviceIsCharging_Bool, false);
    vr::VRProperties()->SetFloatProperty(propertyContainer, vr::Prop_DeviceBatteryPercentage_Float, 1.f); // Always charged

    vr::HmdMatrix34_t l_transform = { -1.f, 0.f, 0.f, 0.f, 0.f, 0.f, -1.f, 0.f, 0.f, -1.f, 0.f, 0.f };
    vr::VRProperties()->SetProperty(propertyContainer, vr::Prop_StatusDisplayTransform_Matrix34, &l_transform, sizeof(vr::HmdMatrix34_t), vr::k_unHmdMatrix34PropertyTag);

    vr::VRProperties()->SetBoolProperty(propertyContainer, vr::Prop_Firmware_UpdateAvailable_Bool, false);
    vr::VRProperties()->SetBoolProperty(propertyContainer, vr::Prop_Firmware_ManualUpdate_Bool, false);
    vr::VRProperties()->SetStringProperty(propertyContainer, vr::Prop_Firmware_ManualUpdateURL_String, "https://developer.valvesoftware.com/wiki/SteamVR/HowTo_Update_Firmware");
    vr::VRProperties()->SetUint64Property(propertyContainer, vr::Prop_HardwareRevision_Uint64, 2214720000); // Changed
    vr::VRProperties()->SetUint64Property(propertyContainer, vr::Prop_FirmwareVersion_Uint64, 1541800000); // Changed
    vr::VRProperties()->SetUint64Property(propertyContainer, vr::Prop_FPGAVersion_Uint64, 512); // Changed
    vr::VRProperties()->SetUint64Property(propertyContainer, vr::Prop_VRCVersion_Uint64, 1514800000); // Changed
    vr::VRProperties()->SetUint64Property(propertyContainer, vr::Prop_RadioVersion_Uint64, 1518800000); // Changed
    vr::VRProperties()->SetUint64Property(propertyContainer, vr::Prop_DongleVersion_Uint64, 8933539758); // Changed, based on vr::Prop_ConnectedWirelessDongle_String above
    vr::VRProperties()->SetBoolProperty(propertyContainer, vr::Prop_DeviceProvidesBatteryStatus_Bool, true);
    vr::VRProperties()->SetBoolProperty(propertyContainer, vr::Prop_DeviceCanPowerOff_Bool, true);
    vr::VRProperties()->SetStringProperty(propertyContainer, vr::Prop_Firmware_ProgrammingTarget_String, GetSerialNumber());
    vr::VRProperties()->SetInt32Property(propertyContainer, vr::Prop_DeviceClass_Int32, vr::TrackedDeviceClass_GenericTracker);
    vr::VRProperties()->SetBoolProperty(propertyContainer, vr::Prop_Firmware_ForceUpdateRequired_Bool, false);
    vr::VRProperties()->SetStringProperty(propertyContainer, vr::Prop_ResourceRoot_String, "htc");
    vr::VRProperties()->SetStringProperty(propertyContainer, vr::Prop_RegisteredDeviceType_String, "ALVR/tracker/hmd_proxy");
    vr::VRProperties()->SetStringProperty(propertyContainer, vr::Prop_InputProfilePath_String, "{htc}/input/vive_tracker_profile.json");
    vr::VRProperties()->SetBoolProperty(propertyContainer, vr::Prop_Identifiable_Bool, false);
    vr::VRProperties()->SetBoolProperty(propertyContainer, vr::Prop_Firmware_RemindUpdate_Bool, false);
    vr::VRProperties()->SetInt32Property(propertyContainer, vr::Prop_ControllerRoleHint_Int32, vr::TrackedControllerRole_Invalid);
    vr::VRProperties()->SetStringProperty(propertyContainer, vr::Prop_ControllerType_String, "vive_tracker_waist");
    vr::VRProperties()->SetInt32Property(propertyContainer, vr::Prop_ControllerHandSelectionPriority_Int32, -1);
    vr::VRProperties()->SetStringProperty(propertyContainer, vr::Prop_NamedIconPathDeviceOff_String, "{htc}/icons/tracker_status_off.png");
    vr::VRProperties()->SetStringProperty(propertyContainer, vr::Prop_NamedIconPathDeviceSearching_String, "{htc}/icons/tracker_status_searching.gif");
    vr::VRProperties()->SetStringProperty(propertyContainer, vr::Prop_NamedIconPathDeviceSearchingAlert_String, "{htc}/icons/tracker_status_searching_alert.gif");
    vr::VRProperties()->SetStringProperty(propertyContainer, vr::Prop_NamedIconPathDeviceReady_String, "{htc}/icons/tracker_status_ready.png");
    vr::VRProperties()->SetStringProperty(propertyContainer, vr::Prop_NamedIconPathDeviceReadyAlert_String, "{htc}/icons/tracker_status_ready_alert.png");
    vr::VRProperties()->SetStringProperty(propertyContainer, vr::Prop_NamedIconPathDeviceNotReady_String, "{htc}/icons/tracker_status_error.png");
    vr::VRProperties()->SetStringProperty(propertyContainer, vr::Prop_NamedIconPathDeviceStandby_String, "{htc}/icons/tracker_status_standby.png");
    vr::VRProperties()->SetStringProperty(propertyContainer, vr::Prop_NamedIconPathDeviceAlertLow_String, "{htc}/icons/tracker_status_ready_low.png");
    vr::VRProperties()->SetBoolProperty(propertyContainer, vr::Prop_HasDisplayComponent_Bool, false);
    vr::VRProperties()->SetBoolProperty(propertyContainer, vr::Prop_HasCameraComponent_Bool, false);
    vr::VRProperties()->SetBoolProperty(propertyContainer, vr::Prop_HasDriverDirectModeComponent_Bool, false);
    vr::VRProperties()->SetBoolProperty(propertyContainer, vr::Prop_HasVirtualDisplayComponent_Bool, false);
    return vr::VRInitError_None;
}

void OvrViveTrackerProxy::update()
{
    auto newPose =  GetPose();
    vr::VRServerDriverHost()->TrackedDevicePoseUpdated(m_unObjectId, newPose, sizeof(vr::DriverPose_t));
}