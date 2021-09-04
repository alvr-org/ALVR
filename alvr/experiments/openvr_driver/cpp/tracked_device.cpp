#include "tracked_device.h"

vr::EVRInitError TrackedDevice::Activate(uint32_t object_id) {
    this->object_id = object_id;
    this->property_container = vr::VRProperties()->TrackedDeviceToPropertyContainer(object_id);

    this->activate_inner();

    set_extra_properties(this->device_index);

    return vr::VRInitError_None;
};

void TrackedDevice::set_motion(MotionData motion, double time_offset_s) {
    this->pose = {};

    if (motion.connected) {
        this->pose.result = vr::TrackingResult_Running_OK;
        this->pose.poseIsValid = true;
        this->pose.deviceIsConnected = true;

        this->pose.poseTimeOffset = time_offset_s;

        this->pose.vecPosition[0] = motion.position[0];
        this->pose.vecPosition[1] = motion.position[1];
        this->pose.vecPosition[2] = motion.position[2];

        if (motion.has_linear_velocity) {
            this->pose.vecVelocity[0] = motion.linear_velocity[0];
            this->pose.vecVelocity[1] = motion.linear_velocity[1];
            this->pose.vecVelocity[2] = motion.linear_velocity[2];
        }

        this->pose.qRotation = motion.orientation;

        if (motion.has_angular_velocity) {
            this->pose.vecAngularVelocity[0] = motion.angular_velocity[0];
            this->pose.vecAngularVelocity[1] = motion.angular_velocity[1];
            this->pose.vecAngularVelocity[2] = motion.angular_velocity[2];
        }
    }

    vr::VRServerDriverHost()->TrackedDevicePoseUpdated(
        this->object_id, this->pose, sizeof(vr::DriverPose_t));
}
