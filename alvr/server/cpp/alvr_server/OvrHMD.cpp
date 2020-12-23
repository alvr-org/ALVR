#include "OvrHMD.h"

void fixInvalidHaptics(float hapticFeedback[3])
{
	// Assign a 5ms duration to legacy haptics pulses which otherwise have 0 duration and wouldn't play.
	if (hapticFeedback[1] == 0.0f) {
		hapticFeedback[1] = 0.005f;
	}
}

OvrHmd::OvrHmd()
		: m_unObjectId(vr::k_unTrackedDeviceIndexInvalid)
		, m_baseComponentsInitialized(false)
		, m_streamComponentsInitialized(false)
	{
		m_ulPropertyContainer = vr::k_ulInvalidPropertyContainer;

		Debug("Startup: %hs %hs\n", APP_MODULE_NAME, APP_VERSION_STRING);

		bool ret;
		ret = vr::VRServerDriverHost()->TrackedDeviceAdded(
			GetSerialNumber().c_str(),
			vr::TrackedDeviceClass_HMD,
			this);

		if (!Settings::Instance().m_disableController) {
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

		Debug("CRemoteHmd successfully initialized.\n");
	}

	OvrHmd::~OvrHmd()
	{
		ShutdownRuntime();

		StopStreaming();

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

	 vr::EVRInitError OvrHmd::Activate(vr::TrackedDeviceIndex_t unObjectId)
	{
		Debug("CRemoteHmd Activate %d\n", unObjectId);

		m_unObjectId = unObjectId;
		m_ulPropertyContainer = vr::VRProperties()->TrackedDeviceToPropertyContainer(m_unObjectId);

		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_TrackingSystemName_String, Settings::Instance().mTrackingSystemName.c_str());
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_ModelNumber_String, Settings::Instance().mModelNumber.c_str());
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_ManufacturerName_String, Settings::Instance().mManufacturerName.c_str());
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_RenderModelName_String, Settings::Instance().mRenderModelName.c_str());
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_RegisteredDeviceType_String, Settings::Instance().mRegisteredDeviceType.c_str());
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_DriverVersion_String, Settings::Instance().mDriverVersion.c_str());
		vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_UserIpdMeters_Float, Settings::Instance().m_flIPD);
		vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_UserHeadToEyeDepthMeters_Float, 0.f);
		vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_DisplayFrequency_Float, static_cast<float>(Settings::Instance().m_refreshRate));
		vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_SecondsFromVsyncToPhotons_Float, Settings::Instance().m_flSecondsFromVsyncToPhotons);

		// return a constant that's not 0 (invalid) or 1 (reserved for Oculus)
		vr::VRProperties()->SetUint64Property(m_ulPropertyContainer, vr::Prop_CurrentUniverseId_Uint64, Settings::Instance().m_universeId);

		// avoid "not fullscreen" warnings from vrmonitor
		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_IsOnDesktop_Bool, false);

		// Manually send VSync events on direct mode. ref:https://github.com/ValveSoftware/virtual_display/issues/1
		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_DriverDirectModeSendsVsyncEvents_Bool, true);

		// Set battery as true
		vr::VRProperties()->SetBoolProperty(m_ulPropertyContainer, vr::Prop_DeviceProvidesBatteryStatus_Bool, true);

		float originalIPD = vr::VRSettings()->GetFloat(vr::k_pch_SteamVR_Section, vr::k_pch_SteamVR_IPD_Float);
		vr::VRSettings()->SetFloat(vr::k_pch_SteamVR_Section, vr::k_pch_SteamVR_IPD_Float, Settings::Instance().m_flIPD);

		HmdMatrix_SetIdentity(&m_eyeToHeadLeft);
		HmdMatrix_SetIdentity(&m_eyeToHeadRight);

		//set the icons in steamvr to the default icons used for Oculus Link
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceOff_String, "{oculus}/icons/quest_headset_off.png");
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceSearching_String, "{oculus}/icons/quest_headset_searching.gif");
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceSearchingAlert_String, "{oculus}/icons/quest_headset_alert_searching.gif");
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceReady_String, "{oculus}/icons/quest_headset_ready.png");
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceReadyAlert_String, "{oculus}/icons/quest_headset_ready_alert.png");
		vr::VRProperties()->SetStringProperty(m_ulPropertyContainer, vr::Prop_NamedIconPathDeviceStandby_String, "{oculus}/icons/quest_headset_standby.png");
			   		 	  	  
		
		if (!m_baseComponentsInitialized) {
			m_baseComponentsInitialized = true;

			m_D3DRender = std::make_shared<CD3DRender>();

			// Use the same adapter as vrcompositor uses. If another adapter is used, vrcompositor says "failed to open shared texture" and then crashes.
			// It seems vrcompositor selects always(?) first adapter. vrcompositor may use Intel iGPU when user sets it as primary adapter. I don't know what happens on laptop which support optimus.
			// Prop_GraphicsAdapterLuid_Uint64 is only for redirect display and is ignored on direct mode driver. So we can't specify an adapter for vrcompositor.
			// m_nAdapterIndex is set 0 on the launcher.
			if (!m_D3DRender->Initialize(Settings::Instance().m_nAdapterIndex))
			{
				Error("Could not create graphics device for adapter %d.  Requires a minimum of two graphics cards.\n", Settings::Instance().m_nAdapterIndex);
				return vr::VRInitError_Driver_Failed;
			}

			int32_t nDisplayAdapterIndex;
			if (!m_D3DRender->GetAdapterInfo(&nDisplayAdapterIndex, m_adapterName))
			{
				Error("Failed to get primary adapter info!\n");
				return vr::VRInitError_Driver_Failed;
			}

			Info("Using %ls as primary graphics adapter.\n", m_adapterName.c_str());
			Info("OSVer: %ls\n", GetWindowsOSVersion().c_str());

			m_VSyncThread = std::make_shared<VSyncThread>(Settings::Instance().m_refreshRate);
			m_VSyncThread->Start();

			m_displayComponent = std::make_shared<OvrDisplayComponent>();
			m_directModeComponent = std::make_shared<OvrDirectModeComponent>(m_D3DRender);

			m_ChaperoneUpdater = std::make_shared<ChaperoneUpdater>();
			m_ChaperoneUpdater->ResetData(0, 0);
			m_ChaperoneUpdater->GenerateStandingChaperone();
			m_ChaperoneUpdater->MaybeCommitData();

			DriverReadyIdle();
		}

		vr::VREvent_Data_t eventData;
		eventData.ipd = { Settings::Instance().m_flIPD };
		vr::VRServerDriverHost()->VendorSpecificEvent(m_unObjectId, vr::VREvent_IpdChanged, eventData, 0);

		return vr::VRInitError_None;
	}

	 void OvrHmd::Deactivate() 
	{
		Debug("CRemoteHmd Deactivate\n");
		m_unObjectId = vr::k_unTrackedDeviceIndexInvalid;
	}

	 void OvrHmd::EnterStandby()
	{
	}

	void* OvrHmd::GetComponent(const char *pchComponentNameAndVersion)
	{
		Debug("GetComponent %hs\n", pchComponentNameAndVersion);
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

		if (m_Listener && m_Listener->HasValidTrackingInfo()) {

			TrackingInfo info;
			m_Listener->GetTrackingInfo(info);


			pose.qRotation = HmdQuaternion_Init(info.HeadPose_Pose_Orientation.w,
				info.HeadPose_Pose_Orientation.x, 
				info.HeadPose_Pose_Orientation.y,
				info.HeadPose_Pose_Orientation.z);

			
			pose.vecPosition[0] = info.HeadPose_Pose_Position.x;
			pose.vecPosition[1] = info.HeadPose_Pose_Position.y;
			pose.vecPosition[2] = info.HeadPose_Pose_Position.z;

			// set battery percentage
			vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer, vr::Prop_DeviceBatteryPercentage_Float, info.battery / 100.0f);

			Debug("GetPose: Rotation=(%f, %f, %f, %f) Position=(%f, %f, %f)\n",
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
			//LogDriver("RunFrame");
			//vr::VRServerDriverHost()->TrackedDevicePoseUpdated(m_unObjectId, GetPose(), sizeof(vr::DriverPose_t));
		}
	}

	void OvrHmd::OnPoseUpdated() {
		if (m_unObjectId != vr::k_unTrackedDeviceIndexInvalid)
		{
			if (!m_Listener || !m_Listener->HasValidTrackingInfo()) {
				return;
			}
			
			TrackingInfo info;
			m_Listener->GetTrackingInfo(info);

			//TODO: Right order?

			if (!Settings::Instance().m_disableController) {
				updateController(info);
			}

			if (std::fabs(info.ipd - Settings::Instance().m_flIPD) > 0.0001f
				|| std::fabs(info.eyeFov[0].left - Settings::Instance().m_eyeFov[0].left) > 0.1f
				|| std::fabs(info.eyeFov[0].right - Settings::Instance().m_eyeFov[0].right) > 0.1f) {
				updateIPDandFoV(info);
			}

			m_directModeComponent->OnPoseUpdated(info);
		
			vr::VRServerDriverHost()->TrackedDevicePoseUpdated(m_unObjectId, GetPose(), sizeof(vr::DriverPose_t));

		}
	}

	void OvrHmd::StartStreaming() {
		if (m_streamComponentsInitialized) {
			return;
		}

		//create listener
		m_Listener.reset(new ClientConnection(
			m_ChaperoneUpdater, [&]() { OnPoseUpdated(); }, [&]() { OnPacketLoss(); }));

		// Spin up a separate thread to handle the overlapped encoding/transmit step.
		m_encoder = std::make_shared<CEncoder>();
		try {
			m_encoder->Initialize(m_D3DRender, m_Listener);
		}
		catch (Exception e) {
			Error("Failed to initialize CEncoder. %s\n", e.what());
		}
		m_encoder->Start();

		if (Settings::Instance().m_enableSound) {
			m_audioCapture = std::make_shared<AudioCapture>(m_Listener);
			try {
				m_audioCapture->Start(Settings::Instance().m_soundDevice);
			}
			catch (Exception e) {
				Error("Failed to start audio capture. %s\n", e.what());
			}
		}

		m_directModeComponent->SetEncoder(m_encoder);

		m_encoder->OnStreamStart();

		m_streamComponentsInitialized = true;
	}

	void OvrHmd::StopStreaming() {
		if (m_encoder)
		{
			Debug("OvrHmd::StopStreaming(): Stopping encoder...\n");
			m_encoder->Stop();
			m_encoder.reset();
		}

		if (m_audioCapture)
		{
			Debug("OvrHmd::StopStreaming(): Stopping audio capture...\n");
			m_audioCapture->Shutdown();
			m_audioCapture.reset();
		}

		if (m_Listener)
		{
			Debug("OvrHmd::StopStreaming(): Stopping network...\n");
			m_Listener->Stop();
			m_Listener.reset();
		}
	}

	void OvrHmd::updateIPDandFoV(const TrackingInfo& info) {
		Info("Setting new IPD to: %f\n", info.ipd);

		m_eyeToHeadLeft.m[0][3]  = -info.ipd / 2.0f;
		m_eyeToHeadRight.m[0][3] =  info.ipd / 2.0f;
		vr::VRServerDriverHost()->SetDisplayEyeToHead(m_unObjectId, m_eyeToHeadLeft, m_eyeToHeadRight);
		vr::VRSettings()->SetFloat(vr::k_pch_SteamVR_Section, vr::k_pch_SteamVR_IPD_Float, info.ipd);

		Settings::Instance().m_eyeFov[0] = info.eyeFov[0];
		Settings::Instance().m_eyeFov[1] = info.eyeFov[1];

		m_displayComponent->GetProjectionRaw(vr::EVREye::Eye_Left,
			&m_eyeFoVLeft.vTopLeft.v[0],
			&m_eyeFoVLeft.vBottomRight.v[0],
			&m_eyeFoVLeft.vTopLeft.v[1],
			&m_eyeFoVLeft.vBottomRight.v[1]);
		m_displayComponent->GetProjectionRaw(vr::EVREye::Eye_Right,
			&m_eyeFoVRight.vTopLeft.v[0],
			&m_eyeFoVRight.vBottomRight.v[0],
			&m_eyeFoVRight.vTopLeft.v[1],
			&m_eyeFoVRight.vBottomRight.v[1]);

		vr::VRServerDriverHost()->SetDisplayProjectionRaw(m_unObjectId, m_eyeFoVLeft, m_eyeFoVRight);
		Settings::Instance().m_flIPD = info.ipd;

		vr::VRServerDriverHost()->VendorSpecificEvent(m_unObjectId, vr::VREvent_LensDistortionChanged, {0, 0}, 0);
	}

	void OvrHmd::updateController(const TrackingInfo& info) {
		//haptic feedback
		float  hapticFeedbackLeft[3]{0,0,0};
		float  hapticFeedbackRight[3]{ 0,0,0 };
		vr::VREvent_t vrEvent;

		//collect events since the last update
		while (vr::VRServerDriverHost()->PollNextEvent(&vrEvent, sizeof(vrEvent)))
		{
			if (vrEvent.eventType == vr::VREvent_Input_HapticVibration)
			{

				// if multiple events occurred within one frame, they are ignored except for last event
				
					if (m_leftController->getHapticComponent() == vrEvent.data.hapticVibration.componentHandle) {
					
						hapticFeedbackLeft[0] = vrEvent.data.hapticVibration.fAmplitude * Settings::Instance().m_hapticsIntensity;
						hapticFeedbackLeft[1] = vrEvent.data.hapticVibration.fDurationSeconds;
						hapticFeedbackLeft[2] = vrEvent.data.hapticVibration.fFrequency;

						fixInvalidHaptics(hapticFeedbackLeft);

					} else if (m_rightController->getHapticComponent() == vrEvent.data.hapticVibration.componentHandle) {
					
						hapticFeedbackRight[0] = vrEvent.data.hapticVibration.fAmplitude * Settings::Instance().m_hapticsIntensity;
						hapticFeedbackRight[1] = vrEvent.data.hapticVibration.fDurationSeconds;
						hapticFeedbackRight[2] = vrEvent.data.hapticVibration.fFrequency;

						fixInvalidHaptics(hapticFeedbackRight);
					}
				
			}
		}

		if (m_Listener) {
			//send feedback if changed
			if (hapticFeedbackLeft[0] != 0 ||
				hapticFeedbackLeft[1] != 0 ||
				hapticFeedbackLeft[2] != 0 ) {

				m_Listener->SendHapticsFeedback(0,
					hapticFeedbackLeft[0],
					hapticFeedbackLeft[1],
					hapticFeedbackLeft[2],
					m_leftController->GetHand() ? 1 : 0);

			}

			if (hapticFeedbackRight[0] != 0 ||
				hapticFeedbackRight[1] != 0 ||
				hapticFeedbackRight[2] != 0) {

				m_Listener->SendHapticsFeedback(0,
					hapticFeedbackRight[0],
					hapticFeedbackRight[1],
					hapticFeedbackRight[2],
					m_rightController->GetHand() ? 1 : 0);

			}
		}
		
		//Update controller

		for (int i = 0; i < 2; i++) {	

			bool leftHand = (info.controller[i].flags & TrackingInfo::Controller::FLAG_CONTROLLER_LEFTHAND) != 0;
		
			if (leftHand) {
				m_leftController->onPoseUpdate(i, info);
			} else {
				m_rightController->onPoseUpdate(i, info);
			}
		}
	}

	void OvrHmd::OnPacketLoss() {
		if (!m_streamComponentsInitialized) {
			return;
		}
		Debug("OnPacketLoss()\n");
		m_encoder->OnPacketLoss();
	}

	void OvrHmd::OnShutdown() {
		Info("Sending shutdown signal to vrserver.\n");
		vr::VREvent_Reserved_t data = { 0, 0 };
		vr::VRServerDriverHost()->VendorSpecificEvent(m_unObjectId, vr::VREvent_DriverRequestedQuit, (vr::VREvent_Data_t&)data, 0);
	}

	void OvrHmd::RequestIDR() {
		if (!m_streamComponentsInitialized) {
			return;
		}
		Debug("RequestIDR()\n");
		m_encoder->InsertIDR();
	}
