#pragma once

#include <openvr_driver.h>

class Hmd;

class ViveTrackerProxy final : public vr::ITrackedDeviceServerDriver
{
	vr::TrackedDeviceIndex_t m_unObjectId;
    Hmd* m_HMDOwner;    
public:

    ViveTrackerProxy(Hmd& owner);

    ViveTrackerProxy(const ViveTrackerProxy&) = delete;
    ViveTrackerProxy& operator=(const ViveTrackerProxy&) = delete;

    constexpr inline const char* GetSerialNumber() const { return "ALVR HMD Tracker Proxy"; }

    virtual vr::EVRInitError Activate( vr::TrackedDeviceIndex_t unObjectId ) override;
    
    virtual inline void Deactivate() override
	{
		m_unObjectId = vr::k_unTrackedDeviceIndexInvalid;
	}

	virtual inline void EnterStandby() override {}	
	virtual inline void *GetComponent( const char */*pchComponentNameAndVersion*/ ) override
	{
		// override this to add a component to a driver
		return nullptr;
	}

	virtual inline void DebugRequest( const char */*pchRequest*/, char *pchResponseBuffer, uint32_t unResponseBufferSize ) override
	{
		if ( unResponseBufferSize >= 1 )
			pchResponseBuffer[0] = 0;
	}
    
	virtual vr::DriverPose_t GetPose() override;

    void update();
};