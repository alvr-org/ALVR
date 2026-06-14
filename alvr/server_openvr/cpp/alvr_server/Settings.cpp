#include "Settings.h"
#include "Logger.h"
#include "bindings.h"
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <streambuf>
#include <string>

#define PICOJSON_USE_INT64
#include "include/picojson.h"

using namespace std;

extern uint64_t g_DriverTestMode;

Settings g_settings;
bool g_settingsLoaded = false;

void Settings_Load() {
    try {
        auto sessionFile = std::ifstream(g_sessionPath);

        auto json = std::string(
            std::istreambuf_iterator<char>(sessionFile), std::istreambuf_iterator<char>()
        );

        picojson::value v;
        std::string err = picojson::parse(v, json);
        if (!err.empty()) {
            Error("Error on parsing session config (%s): %hs\n", g_sessionPath, err.c_str());
            return;
        }

        auto config = v.get("openvr_config");

        g_settings.m_refreshRate = (int)config.get("refresh_rate").get<int64_t>();
        g_settings.m_renderWidth = config.get("eye_resolution_width").get<int64_t>() * 2;
        g_settings.m_renderHeight = config.get("eye_resolution_height").get<int64_t>();
        g_settings.m_recommendedTargetWidth
            = config.get("target_eye_resolution_width").get<int64_t>() * 2;
        g_settings.m_recommendedTargetHeight
            = config.get("target_eye_resolution_height").get<int64_t>();
        g_settings.m_nAdapterIndex = (int)config.get("adapter_index").get<int64_t>();
        strncpy(
            g_settings.m_captureFrameDir,
            config.get("capture_frame_dir").get<std::string>().c_str(),
            sizeof(g_settings.m_captureFrameDir) - 1
        );

        g_settings.m_enableFoveatedEncoding = config.get("enable_foveated_encoding").get<bool>();
        g_settings.m_foveationCenterSizeX
            = (float)config.get("foveation_center_size_x").get<double>();
        g_settings.m_foveationCenterSizeY
            = (float)config.get("foveation_center_size_y").get<double>();
        g_settings.m_foveationCenterShiftX
            = (float)config.get("foveation_center_shift_x").get<double>();
        g_settings.m_foveationCenterShiftY
            = (float)config.get("foveation_center_shift_y").get<double>();
        g_settings.m_foveationEdgeRatioX
            = (float)config.get("foveation_edge_ratio_x").get<double>();
        g_settings.m_foveationEdgeRatioY
            = (float)config.get("foveation_edge_ratio_y").get<double>();

        g_settings.m_enableColorCorrection = config.get("enable_color_correction").get<bool>();
        g_settings.m_brightness = (float)config.get("brightness").get<double>();
        g_settings.m_contrast = (float)config.get("contrast").get<double>();
        g_settings.m_saturation = (float)config.get("saturation").get<double>();
        g_settings.m_gamma = (float)config.get("gamma").get<double>();
        g_settings.m_sharpening = (float)config.get("sharpening").get<double>();

        g_settings.m_codec = (int)config.get("codec").get<int64_t>();
        g_settings.m_h264Profile = (int)config.get("h264_profile").get<int64_t>();
        g_settings.m_rateControlMode = (unsigned int)config.get("rate_control_mode").get<int64_t>();
        g_settings.m_fillerData = config.get("filler_data").get<bool>();
        g_settings.m_entropyCoding = (unsigned int)config.get("entropy_coding").get<int64_t>();
        g_settings.m_use10bitEncoder = config.get("use_10bit_encoder").get<bool>();
        g_settings.m_encodingGamma = config.get("encoding_gamma").get<double>();
        g_settings.m_enableHdr = config.get("enable_hdr").get<bool>();
        g_settings.m_forceHdrSrgbCorrection = config.get("force_hdr_srgb_correction").get<bool>();
        g_settings.m_clampHdrExtendedRange = config.get("clamp_hdr_extended_range").get<bool>();
        g_settings.m_enableAmfPreAnalysis = config.get("enable_amf_pre_analysis").get<bool>();
        g_settings.m_enableVbaq = config.get("enable_vbaq").get<bool>();
        g_settings.m_enableAmfHmqb = config.get("enable_amf_hmqb").get<bool>();
        g_settings.m_useAmfPreproc = config.get("use_amf_preproc").get<bool>();
        g_settings.m_amfPreProcSigma = (unsigned int)config.get("amf_preproc_sigma").get<int64_t>();
        g_settings.m_amfPreProcTor = (unsigned int)config.get("amf_preproc_tor").get<int64_t>();
        g_settings.m_encoderQualityPreset
            = (unsigned int)config.get("encoder_quality_preset").get<int64_t>();
        g_settings.m_amdBitrateCorruptionFix
            = (bool)config.get("amd_bitrate_corruption_fix").get<bool>();
        g_settings.m_nvencQualityPreset
            = (unsigned int)config.get("nvenc_quality_preset").get<int64_t>();
        g_settings.m_force_sw_encoding = config.get("force_sw_encoding").get<bool>();
        g_settings.m_swThreadCount = (int)config.get("sw_thread_count").get<int64_t>();

        g_settings.m_nvencTuningPreset
            = (unsigned int)config.get("nvenc_tuning_preset").get<int64_t>();
        g_settings.m_nvencMultiPass = (unsigned int)config.get("nvenc_multi_pass").get<int64_t>();
        g_settings.m_nvencAdaptiveQuantizationMode
            = (unsigned int)config.get("nvenc_adaptive_quantization_mode").get<int64_t>();
        g_settings.m_nvencLowDelayKeyFrameScale
            = config.get("nvenc_low_delay_key_frame_scale").get<int64_t>();
        g_settings.m_nvencRefreshRate = config.get("nvenc_refresh_rate").get<int64_t>();
        g_settings.m_nvencEnableIntraRefresh = config.get("enable_intra_refresh").get<bool>();
        g_settings.m_nvencIntraRefreshPeriod = config.get("intra_refresh_period").get<int64_t>();
        g_settings.m_nvencIntraRefreshCount = config.get("intra_refresh_count").get<int64_t>();
        g_settings.m_nvencMaxNumRefFrames = config.get("max_num_ref_frames").get<int64_t>();
        g_settings.m_nvencGopLength = config.get("gop_length").get<int64_t>();
        g_settings.m_nvencPFrameStrategy = config.get("p_frame_strategy").get<int64_t>();
        g_settings.m_nvencRateControlMode = config.get("nvenc_rate_control_mode").get<int64_t>();
        g_settings.m_nvencRcBufferSize = config.get("rc_buffer_size").get<int64_t>();
        g_settings.m_nvencRcInitialDelay = config.get("rc_initial_delay").get<int64_t>();
        g_settings.m_nvencRcMaxBitrate = config.get("rc_max_bitrate").get<int64_t>();
        g_settings.m_nvencRcAverageBitrate = config.get("rc_average_bitrate").get<int64_t>();
        g_settings.m_nvencEnableWeightedPrediction
            = config.get("nvenc_enable_weighted_prediction").get<bool>();

        g_settings.m_minimumIdrIntervalMs = config.get("minimum_idr_interval_ms").get<int64_t>();

        g_settings.m_enableViveTrackerProxy = config.get("enable_vive_tracker_proxy").get<bool>();
        g_settings.m_TrackingRefOnly = config.get("tracking_ref_only").get<bool>();
        g_settings.m_enableLinuxVulkanAsyncCompute = config.get("linux_async_compute").get<bool>();
        g_settings.m_enableLinuxAsyncReprojection
            = config.get("linux_async_reprojection").get<bool>();

        g_settings.m_enableControllers = config.get("controllers_enabled").get<bool>();
        g_settings.m_controllerIsTracker = config.get("controller_is_tracker").get<bool>();

        g_settings.m_enableBodyTrackingFakeVive
            = config.get("body_tracking_vive_enabled").get<bool>();
        g_settings.m_bodyTrackingHasLegs = config.get("body_tracking_has_legs").get<bool>();

        g_settings.m_useSeparateHandTrackers = config.get("use_separate_hand_trackers").get<bool>();

        Info("Render Target: %d %d\n", g_settings.m_renderWidth, g_settings.m_renderHeight);
        Info("Refresh Rate: %d\n", g_settings.m_refreshRate);
        g_settingsLoaded = true;
    } catch (std::exception& e) {
        Error("Exception on parsing session config (%s): %hs\n", g_sessionPath, e.what());
    }
}
