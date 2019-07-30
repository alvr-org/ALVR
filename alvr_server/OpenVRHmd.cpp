//===================== Copyright (c) Valve Corporation. All Rights Reserved. ======================
//
// Example OpenVR driver for demonstrating IVRVirtualDisplay interface.
//
//==================================================================================================

#include "OpenVRHmd.h"

uint64_t gDriverTestMode = 0;

OpenVRHmd::OpenVRHmd(std::shared_ptr<Listener> listener)
	: mObjectId(vr::k_unTrackedDeviceIndexInvalid)
	, mAdded(false)
	, mActivated(false)
	, mListener(listener)
{
	mObjectId = vr::k_unTrackedDeviceIndexInvalid;
	mPropertyContainer = vr::k_ulInvalidPropertyContainer;

	Log(L"Startup: %hs %hs", APP_MODULE_NAME, APP_VERSION_STRING);

	mListener->SetCallback(this);

	Log(L"OpenVRHmd successfully initialized.");
}

OpenVRHmd::~OpenVRHmd()
{
	if (mEncoder)
	{
		mEncoder->Stop();
		mEncoder.reset();
	}

	if (mAudioCapture)
	{
		mAudioCapture->Shutdown();
		mAudioCapture.reset();
	}

	if (mListener)
	{
		mListener->Stop();
		mListener.reset();
	}

	if (mVSyncThread)
	{
		mVSyncThread->Shutdown();
		mVSyncThread.reset();
	}

	if (mD3DRender)
	{
		mD3DRender->Shutdown();
		mD3DRender.reset();
	}

	mRecenterManager.reset();
}

std::string OpenVRHmd::GetSerialNumber() const
{
	return Settings::Instance().mSerialNumber;
}

void OpenVRHmd::Enable()
{
	if (mAdded) {
		return;
	}
	mAdded = true;
	bool ret;
	ret = vr::VRServerDriverHost()->TrackedDeviceAdded(
		GetSerialNumber().c_str(),
		vr::TrackedDeviceClass_HMD,
		this);
	Log(L"TrackedDeviceAdded(HMD) Ret=%d SerialNumber=%hs", ret, GetSerialNumber().c_str());
	if (Settings::Instance().mUseTrackingReference) {
		mTrackingReference = std::make_shared<OpenVRFakeTrackingReference>();
		ret = vr::VRServerDriverHost()->TrackedDeviceAdded(
			mTrackingReference->GetSerialNumber().c_str(),
			vr::TrackedDeviceClass_TrackingReference,
			mTrackingReference.get());
		Log(L"TrackedDeviceAdded(OpenVRFakeTrackingReference) Ret=%d SerialNumber=%hs", ret, GetSerialNumber().c_str());
	}

}

vr::EVRInitError OpenVRHmd::Activate(vr::TrackedDeviceIndex_t unObjectId)
{
	Log(L"OpenVRHmd Activate %d", unObjectId);

	mObjectId = unObjectId;
	mPropertyContainer = vr::VRProperties()->TrackedDeviceToPropertyContainer(mObjectId);

	vr::VRProperties()->SetStringProperty(mPropertyContainer, vr::Prop_TrackingSystemName_String, Settings::Instance().mTrackingSystemName.c_str());
	vr::VRProperties()->SetStringProperty(mPropertyContainer, vr::Prop_ModelNumber_String, Settings::Instance().mModelNumber.c_str());
	vr::VRProperties()->SetStringProperty(mPropertyContainer, vr::Prop_ManufacturerName_String, Settings::Instance().mManufacturerName.c_str());
	vr::VRProperties()->SetStringProperty(mPropertyContainer, vr::Prop_RenderModelName_String, Settings::Instance().mRenderModelName.c_str());
	vr::VRProperties()->SetStringProperty(mPropertyContainer, vr::Prop_RegisteredDeviceType_String, Settings::Instance().mRegisteredDeviceType.c_str());
	vr::VRProperties()->SetFloatProperty(mPropertyContainer, vr::Prop_UserIpdMeters_Float, Settings::Instance().mIPD);
	vr::VRProperties()->SetFloatProperty(mPropertyContainer, vr::Prop_UserHeadToEyeDepthMeters_Float, 0.f);
	vr::VRProperties()->SetFloatProperty(mPropertyContainer, vr::Prop_DisplayFrequency_Float, static_cast<float>(Settings::Instance().mRefreshRate));
	vr::VRProperties()->SetFloatProperty(mPropertyContainer, vr::Prop_SecondsFromVsyncToPhotons_Float, Settings::Instance().mSecondsFromVsyncToPhotons);

	// return a constant that's not 0 (invalid) or 1 (reserved for Oculus)
	vr::VRProperties()->SetUint64Property(mPropertyContainer, vr::Prop_CurrentUniverseId_Uint64, 2);

	// avoid "not fullscreen" warnings from vrmonitor
	vr::VRProperties()->SetBoolProperty(mPropertyContainer, vr::Prop_IsOnDesktop_Bool, false);

	// Manually send VSync events on direct mode. ref:https://github.com/ValveSoftware/virtual_display/issues/1
	vr::VRProperties()->SetBoolProperty(mPropertyContainer, vr::Prop_DriverDirectModeSendsVsyncEvents_Bool, true);

	float originalIPD = vr::VRSettings()->GetFloat(vr::k_pch_SteamVR_Section, vr::k_pch_SteamVR_IPD_Float);
	vr::VRSettings()->SetFloat(vr::k_pch_SteamVR_Section, vr::k_pch_SteamVR_IPD_Float, Settings::Instance().mIPD);

	mD3DRender = std::make_shared<CD3DRender>();

	// Use the same adapter as vrcompositor uses. If another adapter is used, vrcompositor says "failed to open shared texture" and then crashes.
	// It seems vrcompositor selects always(?) first adapter. vrcompositor may use Intel iGPU when user sets it as primary adapter. I don't know what happens on laptop which support optimus.
	// Prop_GraphicsAdapterLuid_Uint64 is only for redirect display and is ignored on direct mode driver. So we can't specify an adapter for vrcompositor.
	// m_nAdapterIndex is set 0 on the launcher.
	if (!mD3DRender->Initialize(Settings::Instance().mAdapterIndex))
	{
		FatalLog(L"Could not create graphics device for adapter %d.  Requires a minimum of two graphics cards.", Settings::Instance().mAdapterIndex);
		return vr::VRInitError_Driver_Failed;
	}

	int32_t nDisplayAdapterIndex;
	if (!mD3DRender->GetAdapterInfo(&nDisplayAdapterIndex, mAdapterName))
	{
		FatalLog(L"Failed to get primary adapter info!");
		return vr::VRInitError_Driver_Failed;
	}

	Log(L"Using %s as primary graphics adapter.", mAdapterName.c_str());
	Log(L"OSVer: %s", GetWindowsOSVersion().c_str());

	// Spin up a separate thread to handle the overlapped encoding/transmit step.
	mEncoder = std::make_shared<FrameEncoder>();
	try {
		mEncoder->Initialize(mD3DRender, mListener);
	}
	catch (Exception e) {
		FatalLog(L"Failed to initialize CEncoder. %s", e.what());
		return vr::VRInitError_Driver_Failed;
	}
	mEncoder->Start();

	if (Settings::Instance().mEnableSound) {
		mAudioCapture = std::make_shared<AudioCapture>(mListener);
		try {
			mAudioCapture->Start(ToWstring(Settings::Instance().mSoundDevice));
		}
		catch (Exception e) {
			FatalLog(L"Failed to start audio capture. %s", e.what());
			return vr::VRInitError_Driver_Failed;
		}
	}

	mVSyncThread = std::make_shared<VSyncThread>(Settings::Instance().mRefreshRate);
	mVSyncThread->Start();

	mRecenterManager = std::make_shared<RecenterManager>();

	mDisplayComponent = std::make_shared<OpenVRDisplayComponent>();
	mDirectModeComponent = std::make_shared<OpenVRDirectModeComponent>(mD3DRender, mEncoder, mListener, mRecenterManager);

	mActivated = true;

	return vr::VRInitError_None;
}

void OpenVRHmd::Deactivate()
{
	Log(L"OpenVRHmd Deactivate");
	mActivated = false;
	mObjectId = vr::k_unTrackedDeviceIndexInvalid;
}

void OpenVRHmd::EnterStandby()
{
}

void * OpenVRHmd::GetComponent(const char * pchComponentNameAndVersion)
{
	Log(L"GetComponent %hs", pchComponentNameAndVersion);
	if (!_stricmp(pchComponentNameAndVersion, vr::IVRDisplayComponent_Version))
	{
		return mDisplayComponent.get();
	}
	if (!_stricmp(pchComponentNameAndVersion, vr::IVRDriverDirectModeComponent_Version))
	{
		return mDirectModeComponent.get();
	}

	// override this to add a component to a driver
	return NULL;
}

/** debug request from a client */

void OpenVRHmd::DebugRequest(const char * pchRequest, char * pchResponseBuffer, uint32_t unResponseBufferSize)
{
	if (unResponseBufferSize >= 1)
		pchResponseBuffer[0] = 0;
}

vr::DriverPose_t OpenVRHmd::GetPose()
{
	vr::DriverPose_t pose = { 0 };
	pose.poseIsValid = true;
	pose.result = vr::TrackingResult_Running_OK;
	pose.deviceIsConnected = true;

	pose.qWorldFromDriverRotation = HmdQuaternion_Init(1, 0, 0, 0);
	pose.qDriverFromHeadRotation = HmdQuaternion_Init(1, 0, 0, 0);
	pose.qRotation = HmdQuaternion_Init(1, 0, 0, 0);

	if (mRecenterManager->HasValidTrackingInfo()) {
		pose.qRotation = mRecenterManager->GetRecenteredHMD();

		TrackingVector3 position = mRecenterManager->GetRecenteredPositionHMD();
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

void OpenVRHmd::RunFrame()
{
	// In a real driver, this should happen from some pose tracking thread.
	// The RunFrame interval is unspecified and can be very irregular if some other
	// driver blocks it for some periodic task.
	if (mObjectId != vr::k_unTrackedDeviceIndexInvalid)
	{
		//Log(L"RunFrame");
		//vr::VRServerDriverHost()->TrackedDevicePoseUpdated(m_unObjectId, GetPose(), sizeof(vr::DriverPose_t));
	}
}

//
// Implementation of Listener::Callback
//

void OpenVRHmd::OnCommand(std::string commandName, std::string args)
{
	if (commandName == "EnableDriverTestMode") {
		gDriverTestMode = strtoull(args.c_str(), NULL, 0);
		mListener->SendCommandResponse("OK\n");
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
			, mListener->DumpConfig().c_str()
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
			, ToUTF8(mAdapterName).c_str() // TODO: Proper treatment of UNICODE. Sanitizing.
			, Settings::Instance().mCodec
			, Settings::Instance().mEncodeBitrate.toMiBits()
			, Settings::Instance().mRenderWidth, Settings::Instance().mRenderHeight
			, Settings::Instance().mRefreshRate
		);
		mListener->SendCommandResponse(buf);
	}
	else if (commandName == "SetConfig") {
		auto index = args.find(" ");
		if (index == std::string::npos) {
			mListener->SendCommandResponse("NG\n");
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
				mListener->SendCommandResponse("NG\n");
				return;
			}
			mListener->SendCommandResponse("OK\n");
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

		mListener->SendCommandResponse("OK\n");
	}
	else {
		Log(L"Invalid control command: %hs", commandName.c_str());
		mListener->SendCommandResponse("NG\n");
	}

}

void OpenVRHmd::OnLauncher() {
	Enable();
}

void OpenVRHmd::OnPoseUpdated() {
	if (mObjectId != vr::k_unTrackedDeviceIndexInvalid)
	{
		if (!mListener->HasValidTrackingInfo()) {
			return;
		}
		if (!mAdded || !mActivated) {
			return;
		}

		TrackingInfo info;
		mListener->GetTrackingInfo(info);

		mRecenterManager->OnPoseUpdated(info, mListener.get());
		mDirectModeComponent->OnPoseUpdated(info);

		vr::VRServerDriverHost()->TrackedDevicePoseUpdated(mObjectId, GetPose(), sizeof(vr::DriverPose_t));

		if (mTrackingReference) {
			mTrackingReference->OnPoseUpdated();
		}
	}
}

void OpenVRHmd::OnNewClient() {
}

void OpenVRHmd::OnStreamStart() {
	if (!mAdded || !mActivated) {
		return;
	}
	Log(L"OnStreamStart()");
	// Insert IDR frame for faster startup of decoding.
	mEncoder->OnStreamStart();
}

void OpenVRHmd::OnFrameAck(bool result, bool isIDR, uint64_t startFrame, uint64_t endFrame) {
	if (!mAdded || !mActivated) {
		return;
	}
	mEncoder->OnFrameAck(result, isIDR, startFrame, endFrame);
}

void OpenVRHmd::OnShutdown() {
	if (!mAdded || !mActivated) {
		return;
	}
	Log(L"Sending shutdown signal to vrserver.");
	vr::VREvent_Reserved_t data = { 0, 0 };
	vr::VRServerDriverHost()->VendorSpecificEvent(mObjectId, vr::VREvent_DriverRequestedQuit, (vr::VREvent_Data_t&)data, 0);
}
