//===================== Copyright (c) Valve Corporation. All Rights Reserved. ======================
//
// Example OpenVR driver for demonstrating IVRVirtualDisplay interface.
//
//==================================================================================================

#pragma once

#include <openvr_driver.h>

#include "Listener.h"
#include "resource.h"
#include "FrameEncoder.h"
#include "VSyncThread.h"
#include "AudioCapture.h"
#include "RecenterManager.h"
#include "OpenVRDisplayComponent.h"
#include "OpenVRDirectModeComponent.h"
#include "OpenVRFakeTrackingReference.h"

class OpenVRHmd : public vr::ITrackedDeviceServerDriver, public Listener::Callback
{
public:
	OpenVRHmd(std::shared_ptr<Listener> listener);
	virtual ~OpenVRHmd();

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
	bool mAdded;
	bool mActivated;
	vr::TrackedDeviceIndex_t mObjectId;
	vr::PropertyContainerHandle_t mPropertyContainer;

	std::wstring mAdapterName;

	std::shared_ptr<CD3DRender> mD3DRender;
	std::shared_ptr<FrameEncoder> mEncoder;
	std::shared_ptr<AudioCapture> mAudioCapture;
	std::shared_ptr<Listener> mListener;
	std::shared_ptr<VSyncThread> mVSyncThread;
	std::shared_ptr<RecenterManager> mRecenterManager;

	std::shared_ptr<OpenVRDisplayComponent> mDisplayComponent;
	std::shared_ptr<OpenVRDirectModeComponent> mDirectModeComponent;

	std::shared_ptr<OpenVRFakeTrackingReference> mTrackingReference;
};
