#pragma once

#include <openvr_driver.h>
#include <memory>

#include "ALVR-common/packet_types.h"

#ifdef _WIN32
#include "platform/win32/OvrDirectModeComponent.h"
#endif

class ClientConnection;
class VSyncThread;

class OvrController;
class OvrController;

class OvrDisplayComponent;
class CEncoder;
#ifdef _WIN32
class CD3DRender;
#endif
class PoseHistory;

//-----------------------------------------------------------------------------
// Purpose:
//-----------------------------------------------------------------------------
class OvrHmd : public vr::ITrackedDeviceServerDriver
{
public:
	OvrHmd();

	virtual ~OvrHmd();

	std::string GetSerialNumber() const;

	virtual vr::EVRInitError Activate(vr::TrackedDeviceIndex_t unObjectId);

	virtual void Deactivate();
	virtual void EnterStandby();

	void *GetComponent(const char *pchComponentNameAndVersion);

	/** debug request from a client */
	virtual void DebugRequest(const char *pchRequest, char *pchResponseBuffer, uint32_t unResponseBufferSize);

	virtual vr::DriverPose_t GetPose();


	void RunFrame();

	void OnPoseUpdated();

	void StartStreaming();

	void StopStreaming();

	void OnStreamStart();

	void OnPacketLoss();

	void OnShutdown();

	void RequestIDR();


	void updateController(const TrackingInfo& info);

	void updateIPDandFoV(const TrackingInfo& info);

	bool IsTrackingRef() const { return m_deviceClass == vr::TrackedDeviceClass_TrackingReference; }
	bool IsHMD() const { return m_deviceClass == vr::TrackedDeviceClass_HMD; }

	std::shared_ptr<ClientConnection> m_Listener;
private:
	bool m_baseComponentsInitialized;
	bool m_streamComponentsInitialized;
	vr::ETrackedDeviceClass m_deviceClass;
	vr::TrackedDeviceIndex_t m_unObjectId;
	vr::PropertyContainerHandle_t m_ulPropertyContainer;
	
	vr::HmdMatrix34_t m_eyeToHeadLeft;
	vr::HmdMatrix34_t m_eyeToHeadRight;
	vr::HmdRect2_t m_eyeFoVLeft;
	vr::HmdRect2_t m_eyeFoVRight;

	std::wstring m_adapterName;

#ifdef _WIN32
	std::shared_ptr<CD3DRender> m_D3DRender;
#endif
	std::shared_ptr<CEncoder> m_encoder;
	std::shared_ptr<VSyncThread> m_VSyncThread;

	std::shared_ptr<OvrController> m_leftController;
	std::shared_ptr<OvrController> m_rightController;

	std::shared_ptr<OvrDisplayComponent> m_displayComponent;
#ifdef _WIN32
	std::shared_ptr<OvrDirectModeComponent> m_directModeComponent;
#endif
	std::shared_ptr<PoseHistory> m_poseHistory;
};
