#pragma once

struct HmdMatrix34_t
{
  float m[3][4];
};

struct HmdVector3_t
{
  float v[3];
};

struct TrackedDevicePose_t
{
  HmdMatrix34_t mDeviceToAbsoluteTracking;
  HmdVector3_t vVelocity;       // velocity in tracker space in m/s 
  HmdVector3_t vAngularVelocity;    // angular velocity in radians/s (?)
  int eTrackingResult;
  char bPoseIsValid;

  // This indicates that there is a device connected for this spot in the pose array.
  // It could go from true to false if the user unplugs the device.
  char bDeviceIsConnected;
};

const TrackedDevicePose_t & find_pose_in_call_stack();
