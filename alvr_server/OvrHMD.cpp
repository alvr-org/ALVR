#include "OvrHMD.h"

OvrHmd::OvrHmd(std::shared_ptr<ClientConnection> listener)
		: m_unObjectId(vr::k_unTrackedDeviceIndexInvalid)
		, m_added(false)
		, mActivated(false)
		, m_Listener(listener)
	{
		m_unObjectId = vr::k_unTrackedDeviceIndexInvalid;
		m_ulPropertyContainer = vr::k_ulInvalidPropertyContainer;

		Log(L"Startup: %hs %hs", APP_MODULE_NAME, APP_VERSION_STRING);

		std::function<void()> launcherCallback = [&]() { Enable(); };
		std::function<void(std::string, std::string)> commandCallback = [&](std::string commandName, std::string args) { CommandCallback(commandName, args); };
		std::function<void()> poseCallback = [&]() { OnPoseUpdated(); };
		std::function<void()> newClientCallback = [&]() { OnNewClient(); };
		std::function<void()> streamStartCallback = [&]() { OnStreamStart(); };
		std::function<void()> packetLossCallback = [&]() { OnPacketLoss(); };
		std::function<void()> shutdownCallback = [&]() { OnShutdown(); };

		m_Listener->SetLauncherCallback(launcherCallback);
		m_Listener->SetCommandCallback(commandCallback);
		m_Listener->SetPoseUpdatedCallback(poseCallback);
		m_Listener->SetNewClientCallback(newClientCallback);
		m_Listener->SetStreamStartCallback(streamStartCallback);
		m_Listener->SetPacketLossCallback(packetLossCallback);
		m_Listener->SetShutdownCallback(shutdownCallback);

		Log(L"CRemoteHmd successfully initialized.");
	}

	OvrHmd::~OvrHmd()
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

	
	}


	void OvrHmd::Enable()
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


		m_leftController = std::make_shared<OvrController>(true, 0);
		ret = vr::VRServerDriverHost()->TrackedDeviceAdded(
			m_leftController->GetSerialNumber().c_str(),
			vr::TrackedDeviceClass_Controller,
			m_leftController.get());

		m_rightController = std::make_shared<OvrController>(false, 1);
		ret = vr::VRServerDriverHost()->TrackedDeviceAdded(
			m_rightController->GetSerialNumber().c_str(),
			vr::TrackedDeviceClass_Controller,
			m_rightController.get());


	}

	 vr::EVRInitError OvrHmd::Activate(vr::TrackedDeviceIndex_t unObjectId)
	{
		Log(L"CRemoteHmd Activate %d", unObjectId);

		m_unObjectId = unObjectId;
		m_ulPropertyContainer = vr::VRProperties()->TrackedDeviceToPropertyContainer(m_unObjectId);

		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_TrackingSystemName_String, Settings::Instance().mTrackingSystemName.c_str());
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_ModelNumber_String, Settings::Instance().mModelNumber.c_str());
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_ManufacturerName_String, Settings::Instance().mManufacturerName.c_str());
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_RenderModelName_String, Settings::Instance().mRenderModelName.c_str());
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_RegisteredDeviceType_String, Settings::Instance().mRegisteredDeviceType.c_str());
		vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_UserIpdMeters_Float, Settings::Instance().m_flIPD);
		vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_UserHeadToEyeDepthMeters_Float, 0.f);
		vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_DisplayFrequency_Float, static_cast<float>(Settings::Instance().m_refreshRate));
		vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_SecondsFromVsyncToPhotons_Float, Settings::Instance().m_flSecondsFromVsyncToPhotons);

		// return a constant that's not 0 (invalid) or 1 (reserved for Oculus)
		vr::VRProperties()->SetUint64Property(m_ulPropertyContainer, vr::Prop_CurrentUniverseId_Uint64, 2);

		// avoid "not fullscreen" warnings from vrmonitor
		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_IsOnDesktop_Bool, false);

		// Manually send VSync events on direct mode. ref:https://github.com/ValveSoftware/virtual_display/issues/1
		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_DriverDirectModeSendsVsyncEvents_Bool, true);

		float originalIPD = vr::VRSettings()->GetFloat(vr::k_pch_SteamVR_Section, vr::k_pch_SteamVR_IPD_Float);
		vr::VRSettings()->SetFloat(vr::k_pch_SteamVR_Section, vr::k_pch_SteamVR_IPD_Float, Settings::Instance().m_flIPD);

		m_D3DRender = std::make_shared<CD3DRender>();

		// Use the same adapter as vrcompositor uses. If another adapter is used, vrcompositor says "failed to open shared texture" and then crashes.
		// It seems vrcompositor selects always(?) first adapter. vrcompositor may use Intel iGPU when user sets it as primary adapter. I don't know what happens on laptop which support optimus.
		// Prop_GraphicsAdapterLuid_Uint64 is only for redirect display and is ignored on direct mode driver. So we can't specify an adapter for vrcompositor.
		// m_nAdapterIndex is set 0 on the launcher.
		if (!m_D3DRender->Initialize(Settings::Instance().m_nAdapterIndex))
		{
			FatalLog(L"Could not create graphics device for adapter %d.  Requires a minimum of two graphics cards.", Settings::Instance().m_nAdapterIndex);
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
		m_encoder = std::make_shared<CEncoder>();
		try {
			m_encoder->Initialize(m_D3DRender, m_Listener);
		}
		catch (Exception e) {
			FatalLog(L"Failed to initialize CEncoder. %s", e.what());
			return vr::VRInitError_Driver_Failed;
		}
		m_encoder->Start();

		if (Settings::Instance().m_enableSound) {
			m_audioCapture = std::make_shared<AudioCapture>(m_Listener);
			try {
				m_audioCapture->Start(ToWstring(Settings::Instance().m_soundDevice));
			}
			catch (Exception e) {
				FatalLog(L"Failed to start audio capture. %s", e.what());
				return vr::VRInitError_Driver_Failed;
			}
		}

		m_VSyncThread = std::make_shared<VSyncThread>(Settings::Instance().m_refreshRate);
		m_VSyncThread->Start();

	

		m_displayComponent = std::make_shared<OvrDisplayComponent>();
		m_directModeComponent = std::make_shared<OvrDirectModeComponent>(m_D3DRender, m_encoder, m_Listener);

		mActivated = true;


		return vr::VRInitError_None;
	}

	 void OvrHmd::Deactivate() 
	{
		Log(L"CRemoteHmd Deactivate");
		mActivated = false;
		m_unObjectId = vr::k_unTrackedDeviceIndexInvalid;
	}

	 void OvrHmd::EnterStandby()
	{
	}

	void* OvrHmd::GetComponent(const char *pchComponentNameAndVersion)
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
	void OvrHmd::DebugRequest(const char *pchRequest, char *pchResponseBuffer, uint32_t unResponseBufferSize)
	{
		if (unResponseBufferSize >= 1)
			pchResponseBuffer[0] = 0;
	}

	vr::DriverPose_t OvrHmd::GetPose()
	{
		vr::DriverPose_t pose = { 0 };
		pose.poseIsValid = true;
		pose.result = vr::TrackingResult_Running_OK;
		pose.deviceIsConnected = true;

		pose.qWorldFromDriverRotation = HmdQuaternion_Init(1, 0, 0, 0);
		pose.qDriverFromHeadRotation = HmdQuaternion_Init(1, 0, 0, 0);
		pose.qRotation = HmdQuaternion_Init(1, 0, 0, 0);

		if (m_Listener->HasValidTrackingInfo()) {

			TrackingInfo info;
			m_Listener->GetTrackingInfo(info);


			pose.qRotation = HmdQuaternion_Init(info.HeadPose_Pose_Orientation.w,
				info.HeadPose_Pose_Orientation.x, 
				info.HeadPose_Pose_Orientation.y,
				info.HeadPose_Pose_Orientation.z);

			
			pose.vecPosition[0] = info.HeadPose_Pose_Position.x;
			pose.vecPosition[1] = info.HeadPose_Pose_Position.y;
			pose.vecPosition[2] = info.HeadPose_Pose_Position.z;

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


	void OvrHmd::RunFrame()
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


	void OvrHmd::CommandCallback(std::string commandName, std::string args)
	{
		if (commandName == "EnableDriverTestMode") {
			Settings::Instance().m_DriverTestMode = strtoull(args.c_str(), NULL, 0);
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
				, k_pch_Settings_DebugLog_Bool, Settings::Instance().m_DebugLog
				, k_pch_Settings_DebugFrameIndex_Bool, Settings::Instance().m_DebugFrameIndex
				, k_pch_Settings_DebugFrameOutput_Bool, Settings::Instance().m_DebugFrameOutput
				, k_pch_Settings_DebugCaptureOutput_Bool, Settings::Instance().m_DebugCaptureOutput
				, k_pch_Settings_UseKeyedMutex_Bool, Settings::Instance().m_UseKeyedMutex
				, k_pch_Settings_ControllerTriggerMode_Int32, Settings::Instance().m_controllerTriggerMode
				, k_pch_Settings_ControllerTrackpadClickMode_Int32, Settings::Instance().m_controllerTrackpadClickMode
				, k_pch_Settings_ControllerTrackpadTouchMode_Int32, Settings::Instance().m_controllerTrackpadTouchMode
				, k_pch_Settings_ControllerBackMode_Int32, Settings::Instance().m_controllerBackMode
				, k_pch_Settings_ControllerRecenterButton_Int32, Settings::Instance().m_controllerRecenterButton
				, ToUTF8(m_adapterName).c_str() // TODO: Proper treatment of UNICODE. Sanitizing.
				, Settings::Instance().m_codec
				, Settings::Instance().mEncodeBitrate.toMiBits()
				, Settings::Instance().m_renderWidth, Settings::Instance().m_renderHeight
				, Settings::Instance().m_refreshRate
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
					Settings::Instance().m_DebugFrameIndex = atoi(args.substr(index + 1).c_str());
				}
				else if (name == k_pch_Settings_DebugFrameOutput_Bool) {
					Settings::Instance().m_DebugFrameOutput = atoi(args.substr(index + 1).c_str());
				}
				else if (name == k_pch_Settings_DebugCaptureOutput_Bool) {
					Settings::Instance().m_DebugCaptureOutput = atoi(args.substr(index + 1).c_str());
				}
				else if (name == k_pch_Settings_UseKeyedMutex_Bool) {
					Settings::Instance().m_UseKeyedMutex = atoi(args.substr(index + 1).c_str());
				}
				else if (name == k_pch_Settings_ControllerTriggerMode_Int32) {
					Settings::Instance().m_controllerTriggerMode = atoi(args.substr(index + 1).c_str());
				}
				else if (name == k_pch_Settings_ControllerTrackpadClickMode_Int32) {
					Settings::Instance().m_controllerTrackpadClickMode = atoi(args.substr(index + 1).c_str());
				}
				else if (name == k_pch_Settings_ControllerTrackpadTouchMode_Int32) {
					Settings::Instance().m_controllerTrackpadTouchMode = atoi(args.substr(index + 1).c_str());
				}
				else if (name == k_pch_Settings_ControllerBackMode_Int32) {
					Settings::Instance().m_controllerBackMode = atoi(args.substr(index + 1).c_str());
				}
				else if (name == k_pch_Settings_ControllerRecenterButton_Int32) {
					Settings::Instance().m_controllerRecenterButton = atoi(args.substr(index + 1).c_str());
				}
				else if (name == "causePacketLoss") {
					Settings::Instance().m_causePacketLoss = atoi(args.substr(index + 1).c_str());
				}
				else if (name == "trackingFrameOffset") {
					Settings::Instance().m_trackingFrameOffset = atoi(args.substr(index + 1).c_str());
				}
				else if (name == "captureLayerDDS") {
					Settings::Instance().m_captureLayerDDSTrigger = atoi(args.substr(index + 1).c_str());
				}
				else if (name == "captureComposedDDS") {
					Settings::Instance().m_captureComposedDDSTrigger = atoi(args.substr(index + 1).c_str());
				}
				else if (name == "controllerPoseOffset") {
					Settings::Instance().m_controllerPoseOffset = (float)atof(args.substr(index + 1).c_str());
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
			Settings::Instance().m_OffsetPos[0] = (float)atof(x.c_str());
			Settings::Instance().m_OffsetPos[1] = (float)atof(y.c_str());
			Settings::Instance().m_OffsetPos[2] = (float)atof(z.c_str());

			Settings::Instance().m_EnableOffsetPos = atoi(enabled.c_str()) != 0;

			m_Listener->SendCommandResponse("OK\n");
		}
		else {
			Log(L"Invalid control command: %hs", commandName.c_str());
			m_Listener->SendCommandResponse("NG\n");
		}

	}

	void OvrHmd::OnPoseUpdated() {
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

			//TODO: Right order?
			updateController(info);

			m_directModeComponent->OnPoseUpdated(info);
		
			vr::VRServerDriverHost()->TrackedDevicePoseUpdated(m_unObjectId, GetPose(), sizeof(vr::DriverPose_t));

		}
	}

	void OvrHmd::updateController(const TrackingInfo& info) {
		
		
		//haptic feedback
		double  hapticFeedbackLeft[3]{0,0,0};
		double  hapticFeedbackRight[3]{ 0,0,0 };
		vr::VREvent_t vrEvent;

		//collect events since the last update
		while (vr::VRServerDriverHost()->PollNextEvent(&vrEvent, sizeof(vrEvent)))
		{
			if (vrEvent.eventType == vr::VREvent_Input_HapticVibration)
			{

				// if multiple events occurred within one frame, they are ignored except for last event
				
					if (m_leftController->getHapticComponent() == vrEvent.data.hapticVibration.componentHandle) {
					
						hapticFeedbackLeft[0] = vrEvent.data.hapticVibration.fAmplitude;
						hapticFeedbackLeft[1] = vrEvent.data.hapticVibration.fDurationSeconds;
						hapticFeedbackLeft[2] = vrEvent.data.hapticVibration.fFrequency;

					} else if (m_rightController->getHapticComponent() == vrEvent.data.hapticVibration.componentHandle) {
					
						hapticFeedbackRight[0] = vrEvent.data.hapticVibration.fAmplitude;
						hapticFeedbackRight[1] = vrEvent.data.hapticVibration.fDurationSeconds;
						hapticFeedbackRight[2] = vrEvent.data.hapticVibration.fFrequency;
					}
				
			}
		}

		


		//send feedback if changed
		if (hapticFeedbackLeft[0] != 0 ||
			hapticFeedbackLeft[1] != 0 ||
			hapticFeedbackLeft[2] != 0 ) {
	
			m_Listener->SendHapticsFeedback(0,
				static_cast<float>(hapticFeedbackLeft[0]),
				static_cast<float>(hapticFeedbackLeft[1]),
				static_cast<float>(hapticFeedbackLeft[2]),
				m_leftController->GetHand() ? 1 : 0);

		}
		
		
		if (hapticFeedbackRight[0] != 0 ||
			hapticFeedbackRight[1] != 0 ||
			hapticFeedbackRight[2] != 0) {

	
			m_Listener->SendHapticsFeedback(0,
				static_cast<float>(hapticFeedbackRight[0]),
				static_cast<float>(hapticFeedbackRight[1]),
				static_cast<float>(hapticFeedbackRight[2]),
				m_rightController->GetHand() ? 1 : 0);

		}
		
		
		//Update controller

		for (int i = 0; i < 2; i++) {	

			bool leftHand = (info.controller[i].flags & TrackingInfo::Controller::FLAG_CONTROLLER_LEFTHAND) != 0;
			Log(L"Updating %d controller deviceID %d", i, info.controller[i].deviceIndex);

			if (leftHand) {
				m_leftController->onPoseUpdate(i, info);
			} else {
				m_rightController->onPoseUpdate(i, info);
			}

		}
		
		
	}

	void OvrHmd::OnNewClient() {
	}

	void OvrHmd::OnStreamStart() {
		if (!m_added || !mActivated) {
			return;
		}
		Log(L"OnStreamStart()");
		// Insert IDR frame for faster startup of decoding.
		m_encoder->OnStreamStart();
	}

	void OvrHmd::OnPacketLoss() {
		if (!m_added || !mActivated) {
			return;
		}
		Log(L"OnPacketLoss()");
		m_encoder->OnPacketLoss();
	}

	void OvrHmd::OnShutdown() {
		if (!m_added || !mActivated) {
			return;
		}
		Log(L"Sending shutdown signal to vrserver.");
		vr::VREvent_Reserved_t data = { 0, 0 };
		vr::VRServerDriverHost()->VendorSpecificEvent(m_unObjectId, vr::VREvent_DriverRequestedQuit, (vr::VREvent_Data_t&)data, 0);
	}
