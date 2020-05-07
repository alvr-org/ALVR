#pragma once
#include <d3d11_1.h>
#include <wincodec.h>
#include <wincodecsdk.h>

#include "openvr_driver.h"
#include "sharedstate.h"
#include "Logger.h"
#include "ClientConnection.h"
#include "Utils.h"
#include "FrameRender.h"
#include "Settings.h"

#include "packet_types.h"
#include "resource.h"
#include "AudioCapture.h"
#include "VideoEncoder.h"
#include "VideoEncoderNVENC.h"
#include "VideoEncoderVCE.h"
#include "IDRScheduler.h"
#include "CEncoder.h"
#include "VSyncThread.h"
#include "OvrDisplayComponent.h"
#include "OvrDirectModeComponent.h"
#include "OvrController.h"

//-----------------------------------------------------------------------------
// Purpose:
//-----------------------------------------------------------------------------
class OvrHmd : public vr::ITrackedDeviceServerDriver
{
public:
	OvrHmd(std::shared_ptr<ClientConnection> listener);


	virtual ~OvrHmd();

	std::string GetSerialNumber() const { return Settings::Instance().mSerialNumber; }

	void Enable();

	virtual vr::EVRInitError Activate(vr::TrackedDeviceIndex_t unObjectId);

	virtual void Deactivate();
	virtual void EnterStandby();

	void *GetComponent(const char *pchComponentNameAndVersion);

	/** debug request from a client */
	virtual void DebugRequest(const char *pchRequest, char *pchResponseBuffer, uint32_t unResponseBufferSize);

	virtual vr::DriverPose_t GetPose();


	void RunFrame();


	void CommandCallback(std::string commandName, std::string args);

	void OnPoseUpdated();

	void OnNewClient();

	void OnStreamStart();

	void OnPacketLoss();

	void OnShutdown();


	void updateController(const TrackingInfo& info);

private:
	bool m_added;
	bool mActivated;
	vr::TrackedDeviceIndex_t m_unObjectId;
	vr::PropertyContainerHandle_t m_ulPropertyContainer;



	std::wstring m_adapterName;


	std::shared_ptr<CD3DRender> m_D3DRender;
	std::shared_ptr<CEncoder> m_encoder;
	std::shared_ptr<AudioCapture> m_audioCapture;
	std::shared_ptr<ClientConnection> m_Listener;
	std::shared_ptr<VSyncThread> m_VSyncThread;

	std::shared_ptr<OvrController> m_leftController;
	std::shared_ptr<OvrController> m_rightController;





	std::shared_ptr<OvrDisplayComponent> m_displayComponent;
	std::shared_ptr<OvrDirectModeComponent> m_directModeComponent;
};
