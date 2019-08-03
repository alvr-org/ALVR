#include "ovrTrackedDeviceManager.h"



ovrTrackedDeviceManager::ovrTrackedDeviceManager()
{
}


ovrTrackedDeviceManager::~ovrTrackedDeviceManager()
{
}


void ovrTrackedDeviceManager::enableTrackedHMD () {
	if (m_HmdAdded) {
		return;
	}
	m_HmdAdded = true;
	bool ret;
	ret = vr::VRServerDriverHost()->TrackedDeviceAdded(
		Settings::Instance().mSerialNumber.c_str(),
		vr::TrackedDeviceClass_HMD,
		this);
	
}