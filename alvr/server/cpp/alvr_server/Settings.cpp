#include "Settings.h"
#include "Logger.h"
#include "ipctools.h"
#include "resource.h"
#define PICOJSON_USE_INT64
#include <picojson.h>
#include <string>
#include <fstream>
#include <streambuf>

using namespace std;

extern uint64_t g_DriverTestMode;

Settings Settings::m_Instance;

Settings::Settings()
	: m_EnableOffsetPos(false), m_loaded(false)
{
	m_OffsetPos[0] = 0.0f;
	m_OffsetPos[1] = 0.0f;
	m_OffsetPos[2] = 0.0f;
}

Settings::~Settings()
{
}

void Settings::Load()
{
	try
	{
		auto sessionFile = std::ifstream(g_alvrDir + "/session.json"s);

		auto json = std::string(
			std::istreambuf_iterator<char>(sessionFile),
			std::istreambuf_iterator<char>());

		picojson::value v;
		std::string err = picojson::parse(v, json);
		if (!err.empty())
		{
			Error("Error on parsing json: %hs\n", err.c_str());
			return;
		}

		auto config = v.get("openvrConfig");

		m_universeId = config.get("universe_id").get<int64_t>();

		mSerialNumber = config.get("headset_serial_number").get<std::string>();
		mTrackingSystemName = config.get("headset_tracking_system_name").get<std::string>();
		mModelNumber = config.get("headset_model_number").get<std::string>();
		mDriverVersion = config.get("headset_driver_version").get<std::string>();
		mManufacturerName = config.get("headset_manufacturer_name").get<std::string>();
		mRenderModelName = config.get("headset_render_model_name").get<std::string>();
		mRegisteredDeviceType = config.get("headset_registered_device_type").get<std::string>();

		m_renderWidth = config.get("eye_resolution_width").get<int64_t>() * 2;
		m_renderHeight = config.get("eye_resolution_height").get<int64_t>();

		m_recommendedTargetWidth = config.get("target_eye_resolution_width").get<int64_t>() * 2;
		m_recommendedTargetHeight = config.get("target_eye_resolution_height").get<int64_t>();

		for (int eye = 0; eye < 2; eye++)
		{
			m_eyeFov[eye].left = 45;
			m_eyeFov[eye].right = 45;
			m_eyeFov[eye].top = 45;
			m_eyeFov[eye].bottom = 45;
		}

		m_enableSound = config.get("enable_game_audio").get<bool>();
		m_soundDevice = config.get("game_audio_device").get<std::string>();
		m_streamMic = config.get("enable_microphone").get<bool>();
		m_microphoneDevice = config.get("microphone_device").get<std::string>();

		m_flSecondsFromVsyncToPhotons = (float)config.get("seconds_from_vsync_to_photons").get<double>();

		m_flIPD = (float)config.get("ipd").get<double>();

		m_clientRecvBufferSize = (uint32_t)config.get("client_buffer_size").get<int64_t>();
		m_frameQueueSize = (uint32_t)config.get("frame_queue_size").get<int64_t>();

		m_force3DOF = config.get("force_3dof").get<bool>();

		m_aggressiveKeyframeResend = config.get("aggressive_keyframe_resend").get<bool>();

		m_nAdapterIndex = (int32_t)config.get("adapter_index").get<int64_t>();

		m_codec = (int32_t)config.get("codec").get<int64_t>();
		m_refreshRate = (int)config.get("refresh_rate").get<int64_t>();
		mEncodeBitrate = Bitrate::fromMiBits((int)config.get("encode_bitrate_mbs").get<int64_t>());

		mThrottlingBitrate = Bitrate::fromBits((int)config.get("throttling_bitrate_bits").get<int64_t>());

		// Listener Parameters
		m_Port = (int)config.get("listen_port").get<int64_t>();

		m_ConnectedClient = config.get("client_address").get<std::string>();

		m_controllerTrackingSystemName = config.get("controllers_tracking_system_name").get<std::string>();
		m_controllerManufacturerName = config.get("controllers_manufacturer_name").get<std::string>();
		m_controllerModelNumber = config.get("controllers_model_number").get<std::string>();
		m_controllerRenderModelNameLeft = config.get("render_model_name_left_controller").get<std::string>();
		m_controllerRenderModelNameRight = config.get("render_model_name_right_controller").get<std::string>();
		m_controllerSerialNumber = config.get("controllers_serial_number").get<std::string>();
		m_controllerType = config.get("controllers_type").get<std::string>();
		mControllerRegisteredDeviceType = config.get("controllers_registered_device_type").get<std::string>();
		m_controllerInputProfilePath = config.get("controllers_input_profile_path").get<std::string>();

		m_controllerMode = (int32_t)config.get("controllers_mode_idx").get<int64_t>();

		m_disableController = !config.get("controllers_enabled").get<bool>();

		m_EnableOffsetPos = true;
		auto headsetPositionOffset = config.get("position_offset").get<picojson::array>();
		m_OffsetPos[0] = (float)headsetPositionOffset[0].get<double>();
		m_OffsetPos[1] = (float)headsetPositionOffset[1].get<double>();
		m_OffsetPos[2] = (float)headsetPositionOffset[2].get<double>();

		m_trackingFrameOffset = (int32_t)config.get("tracking_frame_offset").get<int64_t>();
		m_controllerPoseOffset = (double)config.get("controller_pose_offset").get<double>();

		auto leftControllerPositionOffset = config.get("position_offset_left").get<picojson::array>();
		m_leftControllerPositionOffset[0] = leftControllerPositionOffset[0].get<double>();
		m_leftControllerPositionOffset[1] = leftControllerPositionOffset[1].get<double>();
		m_leftControllerPositionOffset[2] = leftControllerPositionOffset[2].get<double>();

		auto leftControllerRotationOffset = config.get("rotation_offset_left").get<picojson::array>();
		m_leftControllerRotationOffset[0] = leftControllerRotationOffset[0].get<double>();
		m_leftControllerRotationOffset[1] = leftControllerRotationOffset[1].get<double>();
		m_leftControllerRotationOffset[2] = leftControllerRotationOffset[2].get<double>();

		m_hapticsIntensity = config.get("haptics_intensity").get<double>();

		m_enableFoveatedRendering = config.get("enable_foveated_rendering").get<bool>();
		m_foveationStrength = (float)config.get("foveation_strength").get<double>();
		m_foveationShape = (float)config.get("foveation_shape").get<double>();
		m_foveationVerticalOffset = (float)config.get("foveation_vertical_offset").get<double>();

		m_enableColorCorrection = config.get("enable_color_correction").get<bool>();
		m_brightness = (float)config.get("brightness").get<double>();
		m_contrast = (float)config.get("contrast").get<double>();
		m_saturation = (float)config.get("saturation").get<double>();
		m_gamma = (float)config.get("gamma").get<double>();
		m_sharpening = (float)config.get("sharpening").get<double>();

		Debug("Config JSON: %hs\n", json.c_str());
		Info("Serial Number: %hs\n", mSerialNumber.c_str());
		Info("Model Number: %hs\n", mModelNumber.c_str());
		Info("Render Target: %d %d\n", m_renderWidth, m_renderHeight);
		Info("Seconds from Vsync to Photons: %f\n", m_flSecondsFromVsyncToPhotons);
		Info("Refresh Rate: %d\n", m_refreshRate);
		Info("IPD: %f\n", m_flIPD);

		Info("EncoderOptions: %hs\n", m_EncoderOptions.c_str());

		m_loaded = true;
	}
	catch (std::exception &e)
	{
		Error("Exception on parsing json: %hs\n", e.what());
	}
}
