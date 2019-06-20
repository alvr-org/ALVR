//===================== Copyright (c) Valve Corporation. All Rights Reserved. ======================
//
// Example OpenVR driver for demonstrating IVRVirtualDisplay interface.
//
//==================================================================================================

#pragma once

#include <openvr_driver.h>

#include "Listener.h"
#include "resource.h"
#include "FrameRender.h"
#include "FrameEncoder.h"
#include "VSyncThread.h"
#include "Tracking.h"
#include "AudioCapture.h"
#include "RecenterManager.h"
#include "OpenVRDisplayComponent.h"
#include "OpenVRDirectModeComponent.h"

class OpenVRServerDriver : public vr::ITrackedDeviceServerDriver, public Listener::Callback
{
public:
	OpenVRServerDriver(std::shared_ptr<Listener> listener);
	virtual ~OpenVRServerDriver();

	std::string GetSerialNumber() const;
	void Enable();
	virtual vr::EVRInitError Activate(vr::TrackedDeviceIndex_t unObjectId) override;
	virtual void Deactivate() override;
	virtual void EnterStandby() override;
	void *GetComponent(const char *pchComponentNameAndVersion) override;
	virtual void DebugRequest(const char *pchRequest, char *pchResponseBuffer, uint32_t unResponseBufferSize) override;
	virtual vr::DriverPose_t GetPose() override;

	void RunFrame();

	//
	// Implementation of Listener::Callback
	//

	virtual void OnCommand(std::string commandName, std::string args);
	virtual void OnLauncher();;
	virtual void OnPoseUpdated();
	virtual void OnNewClient();
	virtual void OnStreamStart();
	virtual void OnFrameAck(bool result, bool isIDR, uint64_t startFrame, uint64_t endFrame);;
	virtual void OnShutdown();
private:
	bool m_added;
	bool mActivated;
	vr::TrackedDeviceIndex_t m_unObjectId;
	vr::PropertyContainerHandle_t m_ulPropertyContainer;

	std::wstring m_adapterName;

	std::shared_ptr<CD3DRender> m_D3DRender;
	std::shared_ptr<FrameEncoder> m_encoder;
	std::shared_ptr<AudioCapture> m_audioCapture;
	std::shared_ptr<Listener> m_Listener;
	std::shared_ptr<VSyncThread> m_VSyncThread;
	std::shared_ptr<RecenterManager> m_recenterManager;

	std::shared_ptr<OpenVRDisplayComponent> m_displayComponent;
	std::shared_ptr<OpenVRDirectModeComponent> m_directModeComponent;

	std::shared_ptr<TrackingReference> m_trackingReference;
};
