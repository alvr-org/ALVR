#include "Settings.h"
#include "Logger.h"
#define PICOJSON_USE_INT64
#include "include/picojson.h"
#include <string>
#include <fstream>
#include <streambuf>
#include <filesystem>
#include <cstdlib>
#include "bindings.h"

using namespace std;

extern uint64_t g_DriverTestMode;

Settings Settings::m_Instance;

Settings::Settings()
	: m_loaded(false)
{
}

Settings::~Settings()
{
}

void Settings::Load()
{
	try
	{
		auto sessionFile = std::ifstream(g_sessionPath);

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

		auto config = v.get("openvr_config");

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

		m_flSecondsFromVsyncToPhotons = (float)config.get("seconds_from_vsync_to_photons").get<double>();

		m_flIPD = 0.063;

		m_TrackingRefOnly = config.get("tracking_ref_only").get<bool>();

		m_enableViveTrackerProxy = config.get("enable_vive_tracker_proxy").get<bool>();

		m_aggressiveKeyframeResend = config.get("aggressive_keyframe_resend").get<bool>();

		m_nAdapterIndex = (int32_t)config.get("adapter_index").get<int64_t>();

		m_codec = (int32_t)config.get("codec").get<int64_t>();
		m_rateControlMode = (uint32_t)config.get("rate_control_mode").get<int64_t>();
		m_entropyCoding = (uint32_t)config.get("entropy_coding").get<int64_t>();
		m_refreshRate = (int)config.get("refresh_rate").get<int64_t>();
		m_use10bitEncoder = config.get("use_10bit_encoder").get<bool>();
		m_enableVbaq = config.get("enable_vbaq").get<bool>();
		m_usePreproc = config.get("use_preproc").get<bool>();
		m_preProcSigma = (uint32_t)config.get("preproc_sigma").get<int64_t>();
		m_preProcTor = (uint32_t)config.get("preproc_tor").get<int64_t>();
		m_encoderQualityPreset = (uint32_t)config.get("encoder_quality_preset").get<int64_t>();
		m_force_sw_encoding = config.get("force_sw_encoding").get<bool>();
		m_swThreadCount = (int32_t)config.get("sw_thread_count").get<int64_t>();

		m_controllerTrackingSystemName = config.get("controllers_tracking_system_name").get<std::string>();
		m_controllerManufacturerName = config.get("controllers_manufacturer_name").get<std::string>();
		m_controllerModelNumber = config.get("controllers_model_number").get<std::string>();
		m_controllerRenderModelNameLeft = config.get("render_model_name_left_controller").get<std::string>();
		m_controllerRenderModelNameRight = config.get("render_model_name_right_controller").get<std::string>();
		m_controllerSerialNumber = config.get("controllers_serial_number").get<std::string>();
		m_controllerTypeLeft = config.get("controllers_type_left").get<std::string>();
		m_controllerTypeRight = config.get("controllers_type_right").get<std::string>();
		mControllerRegisteredDeviceType = config.get("controllers_registered_device_type").get<std::string>();
		m_controllerInputProfilePath = config.get("controllers_input_profile_path").get<std::string>();

		m_controllerMode = (int32_t)config.get("controllers_mode_idx").get<int64_t>();

		m_disableController = !config.get("controllers_enabled").get<bool>();

		m_overrideTriggerThreshold = config.get("override_trigger_threshold").get<bool>();
		m_triggerThreshold = config.get("trigger_threshold").get<double>();
		m_overrideGripThreshold = config.get("override_grip_threshold").get<bool>();
		m_gripThreshold = config.get("grip_threshold").get<double>();

		m_useHeadsetTrackingSystem = config.get("use_headset_tracking_system").get<bool>();

		m_enableFoveatedRendering = config.get("enable_foveated_rendering").get<bool>();
		m_foveationCenterSizeX = (float)config.get("foveation_center_size_x").get<double>();
		m_foveationCenterSizeY = (float)config.get("foveation_center_size_y").get<double>();
		m_foveationCenterShiftX = (float)config.get("foveation_center_shift_x").get<double>();
		m_foveationCenterShiftY = (float)config.get("foveation_center_shift_y").get<double>();
		m_foveationEdgeRatioX = (float)config.get("foveation_edge_ratio_x").get<double>();
		m_foveationEdgeRatioY = (float)config.get("foveation_edge_ratio_y").get<double>();

		m_enableColorCorrection = config.get("enable_color_correction").get<bool>();
		m_brightness = (float)config.get("brightness").get<double>();
		m_contrast = (float)config.get("contrast").get<double>();
		m_saturation = (float)config.get("saturation").get<double>();
		m_gamma = (float)config.get("gamma").get<double>();
		m_sharpening = (float)config.get("sharpening").get<double>();

		m_enableLinuxVulkanAsync = config.get("linux_async_reprojection").get<bool>();

		m_nvencTuningPreset = (uint32_t)config.get("nvenc_tuning_preset").get<int64_t>();
		m_nvencMultiPass = (uint32_t)config.get("nvenc_multi_pass").get<int64_t>();
		m_nvencAdaptiveQuantizationMode = (uint32_t)config.get("nvenc_adaptive_quantization_mode").get<int64_t>();
		m_nvencLowDelayKeyFrameScale = config.get("nvenc_low_delay_key_frame_scale").get<int64_t>();
		m_nvencRefreshRate = config.get("nvenc_refresh_rate").get<int64_t>();
		m_nvencEnableIntraRefresh = config.get("enable_intra_refresh").get<bool>();
		m_nvencIntraRefreshPeriod = config.get("intra_refresh_period").get<int64_t>();
		m_nvencIntraRefreshCount = config.get("intra_refresh_count").get<int64_t>();
		m_nvencMaxNumRefFrames = config.get("max_num_ref_frames").get<int64_t>();
		m_nvencGopLength = config.get("gop_length").get<int64_t>();
		m_nvencPFrameStrategy = config.get("p_frame_strategy").get<int64_t>();
		m_nvencRateControlMode = config.get("nvenc_rate_control_mode").get<int64_t>();
		m_nvencRcBufferSize = config.get("rc_buffer_size").get<int64_t>();
		m_nvencRcInitialDelay = config.get("rc_initial_delay").get<int64_t>();
		m_nvencRcMaxBitrate = config.get("rc_max_bitrate").get<int64_t>();
		m_nvencRcAverageBitrate = config.get("rc_average_bitrate").get<int64_t>();
		m_nvencEnableWeightedPrediction = config.get("nvenc_enable_weighted_prediction").get<bool>();

		m_captureFrameDir = config.get("capture_frame_dir").get<std::string>();

		Debug("Config JSON: %hs\n", json.c_str());
		Info("Serial Number: %hs\n", mSerialNumber.c_str());
		Info("Model Number: %hs\n", mModelNumber.c_str());
		Info("Render Target: %d %d\n", m_renderWidth, m_renderHeight);
		Info("Seconds from Vsync to Photons: %f\n", m_flSecondsFromVsyncToPhotons);
		Info("Refresh Rate: %d\n", m_refreshRate);
		m_loaded = true;
	}
	catch (std::exception &e)
	{
		Error("Exception on parsing json: %hs\n", e.what());
	}
}
