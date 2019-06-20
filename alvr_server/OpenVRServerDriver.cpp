//===================== Copyright (c) Valve Corporation. All Rights Reserved. ======================
//
// Example OpenVR driver for demonstrating IVRVirtualDisplay interface.
//
//==================================================================================================

#include "OpenVRServerDriver.h"

uint64_t g_DriverTestMode = 0;

OpenVRServerDriver::OpenVRServerDriver(std::shared_ptr<Listener> listener)
	: m_unObjectId(vr::k_unTrackedDeviceIndexInvalid)
	, m_added(false)
	, mActivated(false)
	, m_Listener(listener)
{
	m_unObjectId = vr::k_unTrackedDeviceIndexInvalid;
	m_ulPropertyContainer = vr::k_ulInvalidPropertyContainer;

	Log(L"Startup: %hs %hs", APP_MODULE_NAME, APP_VERSION_STRING);

	m_Listener->SetCallback(this);

	Log(L"OpenVRServerDriver successfully initialized.");
}

OpenVRServerDriver::~OpenVRServerDriver()
{
	if (m_encoder)
	{
		m_encoder->Stop();
		m_encoder.reset();
	}

	if (m_audioCapture)
	{
		m_audioCapture->Shutdown();
		m_audioCapture.reset();
	}

	if (m_Listener)
	{
		m_Listener->Stop();
		m_Listener.reset();
	}

	if (m_VSyncThread)
	{
		m_VSyncThread->Shutdown();
		m_VSyncThread.reset();
	}

	if (m_D3DRender)
	{
		m_D3DRender->Shutdown();
		m_D3DRender.reset();
	}

	m_recenterManager.reset();
}

std::string OpenVRServerDriver::GetSerialNumber() const
{
	return Settings::Instance().mSerialNumber;
}

void OpenVRServerDriver::Enable()
{
	if (m_added) {
		return;
	}
	m_added = true;
	bool ret;
	ret = vr::VRServerDriverHost()->TrackedDeviceAdded(
		GetSerialNumber().c_str(),
		vr::TrackedDeviceClass_HMD,
		this);
	Log(L"TrackedDeviceAdded(HMD) Ret=%d SerialNumber=%hs", ret, GetSerialNumber().c_str());
	if (Settings::Instance().mUseTrackingReference) {
		m_trackingReference = std::make_shared<TrackingReference>();
		ret = vr::VRServerDriverHost()->TrackedDeviceAdded(
			m_trackingReference->GetSerialNumber().c_str(),
			vr::TrackedDeviceClass_TrackingReference,
			m_trackingReference.get());
		Log(L"TrackedDeviceAdded(TrackingReference) Ret=%d SerialNumber=%hs", ret, GetSerialNumber().c_str());
	}

}

vr::EVRInitError OpenVRServerDriver::Activate(vr::TrackedDeviceIndex_t unObjectId)
{
	Log(L"OpenVRServerDriver Activate %d", unObjectId);

	m_unObjectId = unObjectId;
	m_ulPropertyContainer = vr::VRProperties()->TrackedDeviceToPropertyContainer(m_unObjectId);

	vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_TrackingSystemName_String, Settings::Instance().mTrackingSystemName.c_str());
	vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_ModelNumber_String, Settings::Instance().mModelNumber.c_str());
	vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_ManufacturerName_String, Settings::Instance().mManufacturerName.c_str());
	vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_RenderModelName_String, Settings::Instance().mRenderModelName.c_str());
	vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_RegisteredDeviceType_String, Settings::Instance().mRegisteredDeviceType.c_str());
	vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_UserIpdMeters_Float, Settings::Instance().mIPD);
	vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_UserHeadToEyeDepthMeters_Float, 0.f);
	vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_DisplayFrequency_Float, static_cast<float>(Settings::Instance().mRefreshRate));
	vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_SecondsFromVsyncToPhotons_Float, Settings::Instance().mSecondsFromVsyncToPhotons);

	// return a constant that's not 0 (invalid) or 1 (reserved for Oculus)
	vr::VRProperties()->SetUint64Property(m_ulPropertyContainer, vr::Prop_CurrentUniverseId_Uint64, 2);

	// avoid "not fullscreen" warnings from vrmonitor
	vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_IsOnDesktop_Bool, false);

	// Manually send VSync events on direct mode. ref:https://github.com/ValveSoftware/virtual_display/issues/1
	vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_DriverDirectModeSendsVsyncEvents_Bool, true);

	float originalIPD = vr::VRSettings()->GetFloat(vr::k_pch_SteamVR_Section, vr::k_pch_SteamVR_IPD_Float);
	vr::VRSettings()->SetFloat(vr::k_pch_SteamVR_Section, vr::k_pch_SteamVR_IPD_Float, Settings::Instance().mIPD);

	m_D3DRender = std::make_shared<CD3DRender>();

	// Use the same adapter as vrcompositor uses. If another adapter is used, vrcompositor says "failed to open shared texture" and then crashes.
	// It seems vrcompositor selects always(?) first adapter. vrcompositor may use Intel iGPU when user sets it as primary adapter. I don't know what happens on laptop which support optimus.
	// Prop_GraphicsAdapterLuid_Uint64 is only for redirect display and is ignored on direct mode driver. So we can't specify an adapter for vrcompositor.
	// m_nAdapterIndex is set 0 on the launcher.
	if (!m_D3DRender->Initialize(Settings::Instance().mAdapterIndex))
	{
		FatalLog(L"Could not create graphics device for adapter %d.  Requires a minimum of two graphics cards.", Settings::Instance().mAdapterIndex);
		return vr::VRInitError_Driver_Failed;
	}

	int32_t nDisplayAdapterIndex;
	if (!m_D3DRender->GetAdapterInfo(&nDisplayAdapterIndex, m_adapterName))
	{
		FatalLog(L"Failed to get primary adapter info!");
		return vr::VRInitError_Driver_Failed;
	}

	Log(L"Using %s as primary graphics adapter.", m_adapterName.c_str());
	Log(L"OSVer: %s", GetWindowsOSVersion().c_str());

	// Spin up a separate thread to handle the overlapped encoding/transmit step.
	m_encoder = std::make_shared<FrameEncoder>();
	try {
		m_encoder->Initialize(m_D3DRender, m_Listener);
	}
	catch (Exception e) {
		FatalLog(L"Failed to initialize CEncoder. %s", e.what());
		return vr::VRInitError_Driver_Failed;
	}
	m_encoder->Start();

	if (Settings::Instance().mEnableSound) {
		m_audioCapture = std::make_shared<AudioCapture>(m_Listener);
		try {
			m_audioCapture->Start(ToWstring(Settings::Instance().mSoundDevice));
		}
		catch (Exception e) {
			FatalLog(L"Failed to start audio capture. %s", e.what());
			return vr::VRInitError_Driver_Failed;
		}
	}

	m_VSyncThread = std::make_shared<VSyncThread>(Settings::Instance().mRefreshRate);
	m_VSyncThread->Start();

	m_recenterManager = std::make_shared<RecenterManager>();

	m_displayComponent = std::make_shared<OpenVRDisplayComponent>();
	m_directModeComponent = std::make_shared<OpenVRDirectModeComponent>(m_D3DRender, m_encoder, m_Listener, m_recenterManager);

	mActivated = true;

	return vr::VRInitError_None;
}

void OpenVRServerDriver::Deactivate()
{
	Log(L"OpenVRServerDriver Deactivate");
	mActivated = false;
	m_unObjectId = vr::k_unTrackedDeviceIndexInvalid;
}

void OpenVRServerDriver::EnterStandby()
{
}

void * OpenVRServerDriver::GetComponent(const char * pchComponentNameAndVersion)
{
	Log(L"GetComponent %hs", pchComponentNameAndVersion);
	if (!_stricmp(pchComponentNameAndVersion, vr::IVRDisplayComponent_Version))
	{
		return m_displayComponent.get();
	}
	if (!_stricmp(pchComponentNameAndVersion, vr::IVRDriverDirectModeComponent_Version))
	{
		return m_directModeComponent.get();
	}

	// override this to add a component to a driver
	return NULL;
}

/** debug request from a client */

void OpenVRServerDriver::DebugRequest(const char * pchRequest, char * pchResponseBuffer, uint32_t unResponseBufferSize)
{
	if (unResponseBufferSize >= 1)
		pchResponseBuffer[0] = 0;
}

vr::DriverPose_t OpenVRServerDriver::GetPose()
{
	vr::DriverPose_t pose = { 0 };
	pose.poseIsValid = true;
	pose.result = vr::TrackingResult_Running_OK;
	pose.deviceIsConnected = true;

	pose.qWorldFromDriverRotation = HmdQuaternion_Init(1, 0, 0, 0);
	pose.qDriverFromHeadRotation = HmdQuaternion_Init(1, 0, 0, 0);
	pose.qRotation = HmdQuaternion_Init(1, 0, 0, 0);

	if (m_recenterManager->HasValidTrackingInfo()) {
		pose.qRotation = m_recenterManager->GetRecenteredHMD();

		TrackingVector3 position = m_recenterManager->GetRecenteredPositionHMD();
		pose.vecPosition[0] = position.x;
		pose.vecPosition[1] = position.y;
		pose.vecPosition[2] = position.z;

		Log(L"GetPose: Rotation=(%f, %f, %f, %f) Position=(%f, %f, %f)",
			pose.qRotation.x,
			pose.qRotation.y,
			pose.qRotation.z,
			pose.qRotation.w,
			pose.vecPosition[0],
			pose.vecPosition[1],
			pose.vecPosition[2]
		);

		// To disable time warp (or pose prediction), we dont set (set to zero) velocity and acceleration.

		pose.poseTimeOffset = 0;
	}

	return pose;
}

void OpenVRServerDriver::RunFrame()
{
	// In a real driver, this should happen from some pose tracking thread.
	// The RunFrame interval is unspecified and can be very irregular if some other
	// driver blocks it for some periodic task.
	if (m_unObjectId != vr::k_unTrackedDeviceIndexInvalid)
	{
		//Log(L"RunFrame");
		//vr::VRServerDriverHost()->TrackedDevicePoseUpdated(m_unObjectId, GetPose(), sizeof(vr::DriverPose_t));
	}
}

//
// Implementation of Listener::Callback
//

void OpenVRServerDriver::OnCommand(std::string commandName, std::string args)
{
	if (commandName == "EnableDriverTestMode") {
		g_DriverTestMode = strtoull(args.c_str(), NULL, 0);
		m_Listener->SendCommandResponse("OK\n");
	}
	else if (commandName == "GetConfig") {
		char buf[4000];
		snprintf(buf, sizeof(buf)
			, "%s"
			"%s %d\n"
			"%s %d\n"
			"%s %d\n"
			"%s %d\n"
			"%s %d\n"
			"%s %d\n"
			"%s %d\n"
			"%s %d\n"
			"%s %d\n"
			"%s %d\n"
			"GPU %s\n"
			"Codec %d\n"
			"Bitrate %lluMbps\n"
			"Resolution %dx%d\n"
			"RefreshRate %d\n"
			, m_Listener->DumpConfig().c_str()
			, k_pch_Settings_DebugLog_Bool, Settings::Instance().mDebugLog
			, k_pch_Settings_DebugFrameIndex_Bool, Settings::Instance().mDebugFrameIndex
			, k_pch_Settings_DebugFrameOutput_Bool, Settings::Instance().mDebugFrameOutput
			, k_pch_Settings_DebugCaptureOutput_Bool, Settings::Instance().mDebugCaptureOutput
			, k_pch_Settings_UseKeyedMutex_Bool, Settings::Instance().mUseKeyedMutex
			, k_pch_Settings_ControllerTriggerMode_Int32, Settings::Instance().mControllerTriggerMode
			, k_pch_Settings_ControllerTrackpadClickMode_Int32, Settings::Instance().mControllerTrackpadClickMode
			, k_pch_Settings_ControllerTrackpadTouchMode_Int32, Settings::Instance().mControllerTrackpadTouchMode
			, k_pch_Settings_ControllerBackMode_Int32, Settings::Instance().mControllerBackMode
			, k_pch_Settings_ControllerRecenterButton_Int32, Settings::Instance().mControllerRecenterButton
			, ToUTF8(m_adapterName).c_str() // TODO: Proper treatment of UNICODE. Sanitizing.
			, Settings::Instance().mCodec
			, Settings::Instance().mEncodeBitrate.toMiBits()
			, Settings::Instance().mRenderWidth, Settings::Instance().mRenderHeight
			, Settings::Instance().mRefreshRate
		);
		m_Listener->SendCommandResponse(buf);
	}
	else if (commandName == "SetConfig") {
		auto index = args.find(" ");
		if (index == std::string::npos) {
			m_Listener->SendCommandResponse("NG\n");
		}
		else {
			auto name = args.substr(0, index);
			if (name == k_pch_Settings_DebugFrameIndex_Bool) {
				Settings::Instance().mDebugFrameIndex = atoi(args.substr(index + 1).c_str());
			}
			else if (name == k_pch_Settings_DebugFrameOutput_Bool) {
				Settings::Instance().mDebugFrameOutput = atoi(args.substr(index + 1).c_str());
			}
			else if (name == k_pch_Settings_DebugCaptureOutput_Bool) {
				Settings::Instance().mDebugCaptureOutput = atoi(args.substr(index + 1).c_str());
			}
			else if (name == k_pch_Settings_UseKeyedMutex_Bool) {
				Settings::Instance().mUseKeyedMutex = atoi(args.substr(index + 1).c_str());
			}
			else if (name == k_pch_Settings_ControllerTriggerMode_Int32) {
				Settings::Instance().mControllerTriggerMode = atoi(args.substr(index + 1).c_str());
			}
			else if (name == k_pch_Settings_ControllerTrackpadClickMode_Int32) {
				Settings::Instance().mControllerTrackpadClickMode = atoi(args.substr(index + 1).c_str());
			}
			else if (name == k_pch_Settings_ControllerTrackpadTouchMode_Int32) {
				Settings::Instance().mControllerTrackpadTouchMode = atoi(args.substr(index + 1).c_str());
			}
			else if (name == k_pch_Settings_ControllerBackMode_Int32) {
				Settings::Instance().mControllerBackMode = atoi(args.substr(index + 1).c_str());
			}
			else if (name == k_pch_Settings_ControllerRecenterButton_Int32) {
				Settings::Instance().mControllerRecenterButton = atoi(args.substr(index + 1).c_str());
			}
			else if (name == "causePacketLoss") {
				Settings::Instance().mCausePacketLoss = atoi(args.substr(index + 1).c_str());
			}
			else if (name == "trackingFrameOffset") {
				Settings::Instance().mTrackingFrameOffset = atoi(args.substr(index + 1).c_str());
			}
			else if (name == "captureLayerDDS") {
				Settings::Instance().mCaptureLayerDDSTrigger = atoi(args.substr(index + 1).c_str());
			}
			else if (name == "captureComposedDDS") {
				Settings::Instance().mCaptureComposedDDSTrigger = atoi(args.substr(index + 1).c_str());
			}
			else {
				m_Listener->SendCommandResponse("NG\n");
				return;
			}
			m_Listener->SendCommandResponse("OK\n");
		}
	}
	else if (commandName == "SetOffsetPos") {
		std::string enabled = GetNextToken(args, " ");
		std::string x = GetNextToken(args, " ");
		std::string y = GetNextToken(args, " ");
		std::string z = GetNextToken(args, " ");
		Settings::Instance().mOffsetPos[0] = (float)atof(x.c_str());
		Settings::Instance().mOffsetPos[1] = (float)atof(y.c_str());
		Settings::Instance().mOffsetPos[2] = (float)atof(z.c_str());

		Settings::Instance().mEnableOffsetPos = atoi(enabled.c_str()) != 0;

		m_Listener->SendCommandResponse("OK\n");
	}
	else {
		Log(L"Invalid control command: %hs", commandName.c_str());
		m_Listener->SendCommandResponse("NG\n");
	}

}

void OpenVRServerDriver::OnLauncher() {
	Enable();
}

void OpenVRServerDriver::OnPoseUpdated() {
	if (m_unObjectId != vr::k_unTrackedDeviceIndexInvalid)
	{
		if (!m_Listener->HasValidTrackingInfo()) {
			return;
		}
		if (!m_added || !mActivated) {
			return;
		}

		TrackingInfo info;
		m_Listener->GetTrackingInfo(info);

		m_recenterManager->OnPoseUpdated(info, m_Listener.get());
		m_directModeComponent->OnPoseUpdated(info);

		vr::VRServerDriverHost()->TrackedDevicePoseUpdated(m_unObjectId, GetPose(), sizeof(vr::DriverPose_t));

		if (m_trackingReference) {
			m_trackingReference->OnPoseUpdated();
		}
	}
}

void OpenVRServerDriver::OnNewClient() {
}

void OpenVRServerDriver::OnStreamStart() {
	if (!m_added || !mActivated) {
		return;
	}
	Log(L"OnStreamStart()");
	// Insert IDR frame for faster startup of decoding.
	m_encoder->OnStreamStart();
}

void OpenVRServerDriver::OnFrameAck(bool result, bool isIDR, uint64_t startFrame, uint64_t endFrame) {
	if (!m_added || !mActivated) {
		return;
	}
	m_encoder->OnFrameAck(result, isIDR, startFrame, endFrame);
}

void OpenVRServerDriver::OnShutdown() {
	if (!m_added || !mActivated) {
		return;
	}
	Log(L"Sending shutdown signal to vrserver.");
	vr::VREvent_Reserved_t data = { 0, 0 };
	vr::VRServerDriverHost()->VendorSpecificEvent(m_unObjectId, vr::VREvent_DriverRequestedQuit, (vr::VREvent_Data_t&)data, 0);
}
