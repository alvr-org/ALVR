#include "FakeViveTracker.h"

#include "Logger.h"
#include "Paths.h"
#include "Settings.h"
#include "Utils.h"
#include "bindings.h"
#include <cassert>

FakeViveTracker::FakeViveTracker(uint64_t deviceID)
    : TrackedDevice(deviceID, vr::TrackedDeviceClass_GenericTracker) { }

bool FakeViveTracker::activate() {
    Debug("FakeViveTracker::Activate");

    auto vr_properties = vr::VRProperties();

    // Normally a vive tracker emulator would (logically) always set the tracking system to
    // "lighthouse" but in order to do space calibration with existing tools such as OpenVR Space
    // calibrator and be able to calibrate to/from ALVR HMD (and the proxy tracker) space to/from a
    // native HMD/tracked device which is already using "lighthouse" as the tracking system the
    // proxy tracker needs to be in a different tracking system to treat them differently and
    // prevent those tools doing the same space transform to the proxy tracker.
    vr_properties->SetStringProperty(
        this->prop_container, vr::Prop_TrackingSystemName_String, "ALVRTrackerCustom"
    ); //"lighthouse");
    vr_properties->SetStringProperty(
        this->prop_container, vr::Prop_ModelNumber_String, "Vive Tracker Pro MV"
    );
    vr_properties->SetStringProperty(
        this->prop_container, vr::Prop_SerialNumber_String, this->get_serial_number().c_str()
    ); // Changed
    vr_properties->SetStringProperty(
        this->prop_container, vr::Prop_RenderModelName_String, "{htc}vr_tracker_vive_1_0"
    );
    vr_properties->SetBoolProperty(this->prop_container, vr::Prop_WillDriftInYaw_Bool, false);
    vr_properties->SetStringProperty(this->prop_container, vr::Prop_ManufacturerName_String, "HTC");
    vr_properties->SetStringProperty(
        this->prop_container,
        vr::Prop_TrackingFirmwareVersion_String,
        "1541800000 RUNNER-WATCHMAN$runner-watchman@runner-watchman 2018-01-01 FPGA 512(2.56/0/0) "
        "BL 0 VRC 1541800000 Radio 1518800000"
    ); // Changed
    vr_properties->SetStringProperty(
        this->prop_container,
        vr::Prop_HardwareRevision_String,
        "product 128 rev 2.5.6 lot 2000/0/0 0"
    ); // Changed
    vr_properties->SetStringProperty(
        this->prop_container, vr::Prop_ConnectedWirelessDongle_String, "D0000BE000"
    ); // Changed
    vr_properties->SetBoolProperty(this->prop_container, vr::Prop_DeviceIsWireless_Bool, true);
    vr_properties->SetBoolProperty(this->prop_container, vr::Prop_DeviceIsCharging_Bool, false);
    vr_properties->SetFloatProperty(
        this->prop_container, vr::Prop_DeviceBatteryPercentage_Float, 1.f
    ); // Always charged

    vr::HmdMatrix34_t l_transform
        = { { { -1.f, 0.f, 0.f, 0.f }, { 0.f, 0.f, -1.f, 0.f }, { 0.f, -1.f, 0.f, 0.f } } };
    vr_properties->SetProperty(
        this->prop_container,
        vr::Prop_StatusDisplayTransform_Matrix34,
        &l_transform,
        sizeof(vr::HmdMatrix34_t),
        vr::k_unHmdMatrix34PropertyTag
    );

    vr_properties->SetBoolProperty(
        this->prop_container, vr::Prop_Firmware_UpdateAvailable_Bool, false
    );
    vr_properties->SetBoolProperty(
        this->prop_container, vr::Prop_Firmware_ManualUpdate_Bool, false
    );
    vr_properties->SetStringProperty(
        this->prop_container,
        vr::Prop_Firmware_ManualUpdateURL_String,
        "https://developer.valvesoftware.com/wiki/SteamVR/HowTo_Update_Firmware"
    );
    vr_properties->SetUint64Property(
        this->prop_container, vr::Prop_HardwareRevision_Uint64, 2214720000
    ); // Changed
    vr_properties->SetUint64Property(
        this->prop_container, vr::Prop_FirmwareVersion_Uint64, 1541800000
    ); // Changed
    vr_properties->SetUint64Property(
        this->prop_container, vr::Prop_FPGAVersion_Uint64, 512
    ); // Changed
    vr_properties->SetUint64Property(
        this->prop_container, vr::Prop_VRCVersion_Uint64, 1514800000
    ); // Changed
    vr_properties->SetUint64Property(
        this->prop_container, vr::Prop_RadioVersion_Uint64, 1518800000
    ); // Changed
    vr_properties->SetUint64Property(
        this->prop_container, vr::Prop_DongleVersion_Uint64, 8933539758
    ); // Changed, based on vr::Prop_ConnectedWirelessDongle_String above
    vr_properties->SetBoolProperty(
        this->prop_container, vr::Prop_DeviceProvidesBatteryStatus_Bool, true
    );
    vr_properties->SetBoolProperty(this->prop_container, vr::Prop_DeviceCanPowerOff_Bool, true);
    vr_properties->SetStringProperty(
        this->prop_container,
        vr::Prop_Firmware_ProgrammingTarget_String,
        this->get_serial_number().c_str()
    );
    vr_properties->SetInt32Property(
        this->prop_container, vr::Prop_DeviceClass_Int32, vr::TrackedDeviceClass_GenericTracker
    );
    vr_properties->SetBoolProperty(
        this->prop_container, vr::Prop_Firmware_ForceUpdateRequired_Bool, false
    );
    vr_properties->SetStringProperty(this->prop_container, vr::Prop_ResourceRoot_String, "htc");

    const char* name;
    if (this->device_id == BODY_CHEST_ID) {
        name = "ALVR/tracker/chest";
    } else if (this->device_id == BODY_HIPS_ID) {
        name = "ALVR/tracker/waist";
    } else if (this->device_id == BODY_LEFT_FOOT_ID) {
        name = "ALVR/tracker/left_foot";
    } else if (this->device_id == BODY_RIGHT_FOOT_ID) {
        name = "ALVR/tracker/right_foot";
    } else if (this->device_id == BODY_LEFT_KNEE_ID) {
        name = "ALVR/tracker/left_knee";
    } else if (this->device_id == BODY_RIGHT_KNEE_ID) {
        name = "ALVR/tracker/right_knee";
    } else {
        name = "ALVR/tracker/unknown";
    }
    vr_properties->SetStringProperty(
        this->prop_container, vr::Prop_RegisteredDeviceType_String, name
    );
    vr_properties->SetStringProperty(
        this->prop_container,
        vr::Prop_InputProfilePath_String,
        "{htc}/input/vive_tracker_profile.json"
    );
    vr_properties->SetBoolProperty(this->prop_container, vr::Prop_Identifiable_Bool, false);
    vr_properties->SetBoolProperty(
        this->prop_container, vr::Prop_Firmware_RemindUpdate_Bool, false
    );
    vr_properties->SetInt32Property(
        this->prop_container, vr::Prop_ControllerRoleHint_Int32, vr::TrackedControllerRole_Invalid
    );
    vr_properties->SetStringProperty(
        this->prop_container, vr::Prop_ControllerType_String, "vive_tracker_waist"
    );
    vr_properties->SetInt32Property(
        this->prop_container, vr::Prop_ControllerHandSelectionPriority_Int32, -1
    );
    vr_properties->SetStringProperty(
        this->prop_container,
        vr::Prop_NamedIconPathDeviceOff_String,
        "{htc}/icons/tracker_status_off.png"
    );
    vr_properties->SetStringProperty(
        this->prop_container,
        vr::Prop_NamedIconPathDeviceSearching_String,
        "{htc}/icons/tracker_status_searching.gif"
    );
    vr_properties->SetStringProperty(
        this->prop_container,
        vr::Prop_NamedIconPathDeviceSearchingAlert_String,
        "{htc}/icons/tracker_status_searching_alert.gif"
    );
    vr_properties->SetStringProperty(
        this->prop_container,
        vr::Prop_NamedIconPathDeviceReady_String,
        "{htc}/icons/tracker_status_ready.png"
    );
    vr_properties->SetStringProperty(
        this->prop_container,
        vr::Prop_NamedIconPathDeviceReadyAlert_String,
        "{htc}/icons/tracker_status_ready_alert.png"
    );
    vr_properties->SetStringProperty(
        this->prop_container,
        vr::Prop_NamedIconPathDeviceNotReady_String,
        "{htc}/icons/tracker_status_error.png"
    );
    vr_properties->SetStringProperty(
        this->prop_container,
        vr::Prop_NamedIconPathDeviceStandby_String,
        "{htc}/icons/tracker_status_standby.png"
    );
    vr_properties->SetStringProperty(
        this->prop_container,
        vr::Prop_NamedIconPathDeviceAlertLow_String,
        "{htc}/icons/tracker_status_ready_low.png"
    );
    vr_properties->SetBoolProperty(this->prop_container, vr::Prop_HasDisplayComponent_Bool, false);
    vr_properties->SetBoolProperty(this->prop_container, vr::Prop_HasCameraComponent_Bool, false);
    vr_properties->SetBoolProperty(
        this->prop_container, vr::Prop_HasDriverDirectModeComponent_Bool, false
    );
    vr_properties->SetBoolProperty(
        this->prop_container, vr::Prop_HasVirtualDisplayComponent_Bool, false
    );
    return true;
}

void FakeViveTracker::OnPoseUpdated(uint64_t targetTimestampNs, const FfiDeviceMotion* motion) {
    if (this->object_id == vr::k_unTrackedDeviceIndexInvalid) {
        return;
    }

    bool tracked = motion != nullptr;

    auto pose = vr::DriverPose_t {};
    pose.poseIsValid = tracked;
    pose.deviceIsConnected = tracked;
    pose.result = tracked ? vr::TrackingResult_Running_OK : vr::TrackingResult_Uninitialized;

    pose.qWorldFromDriverRotation = HmdQuaternion_Init(1, 0, 0, 0);
    pose.qDriverFromHeadRotation = HmdQuaternion_Init(1, 0, 0, 0);

    if (motion != nullptr) {
        pose.qRotation = HmdQuaternion_Init(
            motion->orientation.w,
            motion->orientation.x,
            motion->orientation.y,
            motion->orientation.z
        );

        pose.vecPosition[0] = motion->position[0];
        pose.vecPosition[1] = motion->position[1];
        pose.vecPosition[2] = motion->position[2];
    }

    this->submit_pose(pose);
}
