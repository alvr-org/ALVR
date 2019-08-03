#pragma once
#include "openvr_driver.h"
#include "packet_types.h"
#include "Settings.h"

class ovrTrackedDeviceManager : public vr::ITrackedDeviceServerDriver
{
public:
	ovrTrackedDeviceManager();
	~ovrTrackedDeviceManager();

	void enableTrackedHMD();
	void enableTrackedController();
	void onPoseUpdate(TrackingInfo* info);

private:
	bool m_HmdAdded = false;
	
};

