define({
    "root": {
        // Video tab
        "_root_video_tab.name": "Video",
        "_root_video_adapterIndex.name": "GPU index", // adv
        "_root_video_displayRefreshRate.name": "Refresh rate",
        "_root_video_displayRefreshRate.description": "Refresh rate to set for both SteamVR and the headset. Higher values require faster PC. 72 Hz is the maximum for Quest 1.",
        "_root_video_preferredFps.name": "Custom refresh rate", // adv
        "_root_video_resolutionDropdown.name": "Video resolution",
        "_root_video_resolutionDropdown.description": "100% results in the native resolution of the Oculus Quest. \nSetting the resolution can bring some improvement in visual quality, but is not recommended. \nA resolution lower than 100% can reduce latency and increase network performance",
        "_root_video_renderResolution-choice-.name": "Video encoding resolution base", // adv
        "_root_video_renderResolution_scale-choice-.name": "Scale", // adv
        "_root_video_renderResolution_absolute-choice-.name": "Absolute", // adv
        "_root_video_renderResolution_scale.name": "Scale", // adv
        "_root_video_recommendedTargetResolution-choice-.name": "Preferred game rendering resolution", // adv
        "_root_video_recommendedTargetResolution_scale-choice-.name": "Scale", // adv
        "_root_video_recommendedTargetResolution_absolute-choice-.name": "Absolute", // adv
        "_root_video_recommendedTargetResolution_scale.name": "Scale", // adv
        "_root_video_secondsFromVsyncToPhotons.name": "Seconds from VSync to image", // adv
        "_root_video_secondsFromVsyncToPhotons.description": "The time elapsed from the virtual VSync until the image is visible on the viewer screen", // adv
        "_root_video_foveatedRendering.name": "Foveated encoding",
        // "_root_video_foveatedRendering.description": use "_root_video_foveatedRendering_enabled.description"
        "_root_video_foveatedRendering_enabled.description": "Rendering technique that reduces the resolution of the image at the periphery of the vision to reduce the computational load on the GPU. Results in a much lower video resolution that needs to be transmitted over the network.",
        "_root_video_foveatedRendering_content_strength.name": "Strength",
        "_root_video_foveatedRendering_content_strength.description": "Higher value means less detail toward the edges of the frame and more artifacts",
        "_root_video_foveatedRendering_content_shape.name": "Shape", // adv
        "_root_video_foveatedRendering_content_shape.description": "The shape of the foveated rendering", // adv
        "_root_video_foveatedRendering_content_verticalOffset.name": "Vertical offset",
        "_root_video_foveatedRendering_content_verticalOffset.description": "Higher value means the high quality frame region is moved further down",
        "_root_video_colorCorrection.name": "Color correction",
        // "_root_video_colorCorrection.description": use "_root_video_colorCorrection_enabled.description"
        "_root_video_colorCorrection_enabled.description": "Color correction are applied in the following order: Sharpening, Gamma, Brightness, Contrast, and Saturation.",
        "_root_video_colorCorrection_content_brightness.name": "Brightness",
        "_root_video_colorCorrection_content_brightness.description": "Brightness: -1 means completely black and 1 means completely white.",
        "_root_video_colorCorrection_content_contrast.name": "Contrast",
        "_root_video_colorCorrection_content_contrast.description": "Contrast: -1 means completely gray.",
        "_root_video_colorCorrection_content_saturation.name": "Saturation",
        "_root_video_colorCorrection_content_saturation.description": "Saturation: -1 means the image is black and white.",
        "_root_video_colorCorrection_content_gamma.name": "Gamma",
        "_root_video_colorCorrection_content_gamma.description": "Gamut: Use a value of 2.2 to correct the color from sRGB to RGB. This controls the brightness but keeps blacks to black and whites to white",
        "_root_video_colorCorrection_content_sharpening.name": "Sharpening",
        "_root_video_colorCorrection_content_sharpening.description": "Sharpness: emphasizes the edges of the image.",
        "_root_video_codec-choice-.name": "Video codec",
        "_root_video_codec-choice-.description": "HEVC is preferred to achieve better visual quality on lower bitrates. AMD video cards work best with HEVC.",
        "_root_video_codec_H264-choice-.name": "h264",
        "_root_video_codec_HEVC-choice-.name": "HEVC (h265)",
        "_root_video_clientRequestRealtimeDecoder.name": "Request realtime decoder priority (client)", // adv
        "_root_video_use10bitEncoder.name": "Reduce color banding (newer nVidia cards only)",
        "_root_video_use10bitEncoder.description": "This increases visual quality by streaming 10 bit per color channel instead of 8",
        "_root_video_encodeBitrateMbs.name": "Video Bitrate",
        "_root_video_encodeBitrateMbs.description": "Bitrate of video streaming. 30Mbps is recommended. \nHigher bitrates result in better image but also higher latency and network traffic ",
        // Audio tab
        "_root_audio_tab.name": "Audio",
        "_root_audio_gameAudio.name": "Stream game audio",
        // "_root_audio_gameAudio.description": use "_root_audio_gameAudio_enabled.description"
        "_root_audio_gameAudio_enabled.description": "Audio device used to capture game audio. This is used to configure SteamVR audio output.",
        "_root_audio_gameAudio_content_deviceDropdown.name": "Select audio device",
        "_root_audio_gameAudio_content_deviceId-choice-.name": "Audio device",
        "_root_audio_gameAudio_content_deviceId_default-choice-.name": "Default",
        "_root_audio_gameAudio_content_deviceId_name-choice-.name": "By name",
        "_root_audio_gameAudio_content_deviceId_index-choice-.name": "By index",
        "_root_audio_gameAudio_content_muteWhenStreaming.name": "Mute output when streaming",
        "_root_audio_gameAudio_content_muteWhenStreaming.description": "Mutes the audio output (speakers/headphones) when streaming to the headset. Only the physical output is muted (to avoid double audio), stream to the headset and other capturing software will not be affected.",
        "_root_audio_gameAudio_content_config.name": "Configuration",
        "_root_audio_gameAudio_content_config_averageBufferingMs.name": "Buffering (ms)",
        "_root_audio_gameAudio_content_config_averageBufferingMs.description": "Increasing this value may reduce audio stuttering.",
        "_root_audio_microphone.name": "Stream headset microphone",
        // "_root_audio_microphone.description": use "_root_audio_microphone_enabled.description"
        "_root_audio_microphone_enabled.description": "Streams the headset microphone to SteamVR. \nTo make the microphone work you need to install VB-CABLE Virtual Audio Device or another equivalent software.\nThe virtual microphone input is the recording device, the virtual microphone output is the audio rendering device, which is used to configure SteamVR microphone.",
        "_root_audio_microphone_content_inputDeviceDropdown.name": "Select virtual microphone input",
        "_root_audio_microphone_content_inputDeviceDropdown.description": "Output device used to render the microphone audio.",
        "_root_audio_microphone_content_inputDeviceId-choice-.name": "Virtual microphone input",
        "_root_audio_microphone_content_inputDeviceId_default-choice-.name": "Default",
        "_root_audio_microphone_content_inputDeviceId_name-choice-.name": "By name",
        "_root_audio_microphone_content_inputDeviceId_index-choice-.name": "By index",
        "_root_audio_microphone_content_outputDeviceDropdown.name": "Select virtual microphone output",
        "_root_audio_microphone_content_outputDeviceDropdown.description": "Input device used as microphone. This is used to configure SteamVR microphone.",
        "_root_audio_microphone_content_outputDeviceId-choice-.name": "Virtual microphone output",
        "_root_audio_microphone_content_outputDeviceId_default-choice-.name": "Default",
        "_root_audio_microphone_content_outputDeviceId_name-choice-.name": "By name",
        "_root_audio_microphone_content_outputDeviceId_index-choice-.name": "By index",
        "_root_audio_microphone_content_config.name": "Configuration",
        "_root_audio_microphone_content_config_averageBufferingMs.name": "Buffering (ms)",
        "_root_audio_microphone_content_config_averageBufferingMs.description": "Increasing this value may reduce audio stuttering.",
        // Headset tab
        "_root_headset_tab.name": "Headset",
        "_root_headset_headsetEmulationMode.name": "Headset emulation mode",
        "_root_headset_headsetEmulationMode.description": "Emulates different headsets for better compatibility",
        "_root_headset_universeId.name": "Universe ID", // adv
        "_root_headset_serialNumber.name": "Serial number", // adv
        "_root_headset_serialNumber.description": "Serial number of the emulated headset", // adv
        "_root_headset_trackingSystemName.name": "Tracking system name", // adv
        "_root_headset_trackingSystemName.description": "Name of the emulated headset tracking system", // adv
        "_root_headset_modelNumber.name": "Model number", // adv
        "_root_headset_modelNumber.description": "Model number of the emulated headset", // adv
        "_root_headset_driverVersion.name": "Driver version", // adv
        "_root_headset_driverVersion.description": "Driver version of the emulated headset", // adv
        "_root_headset_manufacturerName.name": "Manufacturer name", // adv
        "_root_headset_manufacturerName.description": "Manufacturer name of the emulated headset", // adv
        "_root_headset_renderModelName.name": "Render model name", // adv
        "_root_headset_renderModelName.description": "Render model name of the emulated headset", // adv
        "_root_headset_registeredDeviceType.name": "Registered device type", // adv
        "_root_headset_registeredDeviceType.description": "Registered device type of the emulated headset", // adv
        "_root_headset_trackingFrameOffset.name": "Tracking frame offset",
        "_root_headset_trackingFrameOffset.description": "Offset for the pose prediction algorithm",
        "_root_headset_positionOffset.name": "Headset position offset", // adv
        "_root_headset_positionOffset.description": "Headset position offset used by the position prediction algorithm.", // adv
        "_root_headset_positionOffset_0.name": "X", // adv
        "_root_headset_positionOffset_1.name": "Y", // adv
        "_root_headset_positionOffset_2.name": "Z", // adv
        "_root_headset_force3dof.name": "Force 3Dof",
        "_root_headset_force3dof.description": "Forces the 3 degrees of freedom mode (like Oculus Go)",
        "_root_headset_controllers.name": "Controllers",
        // "_root_headset_controllers.description": use "_root_headset_controllers_enabled.description"
        "_root_headset_controllers_enabled.description": "Allow the use of the controllers",
        "_root_headset_controllers_content_controllerMode.name": "Controller emulation mode",
        "_root_headset_controllers_content_controllerMode.description": "Emulates different controller for better compatibility or enables hand tracking",
        "_root_headset_controllers_content_modeIdx.name": "Mode Index", // adv
        "_root_headset_controllers_content_modeIdx.description": "Mode index of the emulated controller", // adv
        "_root_headset_controllers_content_trackingSystemName.name": "Tracking system name", // adv
        "_root_headset_controllers_content_trackingSystemName.description": "Name of the emulated controller tracking system", // adv
        "_root_headset_controllers_content_manufacturerName.name": "Manufacturer name", // adv
        "_root_headset_controllers_content_manufacturerName.description": "Manufacturer name of the emulated controller", // adv
        "_root_headset_controllers_content_modelNumber.name": "Model number", // adv
        "_root_headset_controllers_content_modelNumber.description": "Model number of the emulated controller", // adv
        "_root_headset_controllers_content_renderModelNameLeft.name": "Model number (Left hand)", // adv
        "_root_headset_controllers_content_renderModelNameLeft.description": "Model number of the emulated left hand controller", // adv
        "_root_headset_controllers_content_renderModelNameRight.name": "Model number (Right hand)", // adv
        "_root_headset_controllers_content_renderModelNameRight.description": "Model number of the emulated right hand controller", // adv
        "_root_headset_controllers_content_serialNumber.name": "Serial number", // adv
        "_root_headset_controllers_content_serialNumber.description": "Serial number of the emulated controller", // adv
        "_root_headset_controllers_content_ctrlType.name": "Controler type", // adv
        "_root_headset_controllers_content_ctrlType.description": "Type of the emulated controller", // adv
        "_root_headset_controllers_content_registeredDeviceType.name": "Registered device type", // adv
        "_root_headset_controllers_content_registeredDeviceType.description": "Registered device type of the emulated controller", // adv
        "_root_headset_controllers_content_inputProfilePath.name": "Input profile path", // adv
        "_root_headset_controllers_content_inputProfilePath.description": "Input profile path of the emulated controller", // adv
        "_root_headset_controllers_content_trackingSpeed.name": "Tracking speed",
        "_root_headset_controllers_content_trackingSpeed.description": "For fast paced games like Beatsaber, choose medium or fast. For slower games like Skyrim leave it on normal. \nOculus prediction means controller position is predicted on the headset instead of on the PC through SteamVR.",
        "_root_headset_controllers_content_poseTimeOffset.name": "Pose time offset", // adv
        "_root_headset_controllers_content_poseTimeOffset.description": "Offset for the pose prediction algorithm", // adv
        "_root_headset_controllers_content_positionOffsetLeft.name": "Position offset", // adv
        "_root_headset_controllers_content_positionOffsetLeft.description": "Position offset in meters for the left controller. \nFor the right controller, x value is mirrored", // adv
        "_root_headset_controllers_content_positionOffsetLeft_0.name": "X", // adv
        "_root_headset_controllers_content_positionOffsetLeft_1.name": "Y", // adv
        "_root_headset_controllers_content_positionOffsetLeft_2.name": "Z", // adv
        "_root_headset_controllers_content_rotationOffsetLeft.name": "Rotation offset", // adv
        "_root_headset_controllers_content_rotationOffsetLeft.description": "Rotation offset in degrees for the left controller. \nFor the right controller, rotations along the Y and Z axes are mirrored", // adv
        "_root_headset_controllers_content_rotationOffsetLeft_0.name": "X", // adv
        "_root_headset_controllers_content_rotationOffsetLeft_1.name": "Y", // adv
        "_root_headset_controllers_content_rotationOffsetLeft_2.name": "Z", // adv
        "_root_headset_controllers_content_hapticsIntensity.name": "Haptics intensity",
        "_root_headset_controllers_content_hapticsIntensity.description": "Factor to reduce or increase the intensity of the vibration of the controls.",
        "_root_headset_trackingSpace-choice-.name": "Tracking Space",
        "_root_headset_trackingSpace-choice-.description": "Sets what the headset uses as its reference for tracking and how the center of the space is defined. Stage tracking space behaves like a wired headset: the center of the space stays in one place after recentering the headset. This must be set if you want to use Vive trackers.",
        "_root_headset_trackingSpace_local-choice-.name": "Local (Headset centered)",
        "_root_headset_trackingSpace_stage-choice-.name": "Stage (Room centered)",
        // Connection tab
        "_root_connection_tab.name": "Connection",
        "_root_connection_autoTrustClients.name": "Trust clients automatically (not recommended)", // adv
        "_root_connection_webServerPort.name": "Web server port",
        "_root_connection_streamProtocol-choice-.name": "Streaming protocol",
        "_root_connection_streamProtocol-choice-.description": "Network protocol used to stream data between client and server. UDP works best at low bitrates (<30), Throttled UDP works best at medium bitrates (~100), TCP works at any bitrate.",
        "_root_connection_streamProtocol_udp-choice-.name": "UDP",
        "_root_connection_streamProtocol_throttledUdp-choice-.name": "Throttled UDP",
        "_root_connection_streamProtocol_tcp-choice-.name": "TCP",
        "_root_connection_streamPort.name": "Server streaming port", // adv
        "_root_connection_streamPort.description": "Port used by the server to receive packets.", // adv
        "_root_connection_aggressiveKeyframeResend.name": "Aggressive keyframe resend",
        "_root_connection_aggressiveKeyframeResend.description": "Decrease minimum interval between keyframes from 100 ms to 5 ms. \nUsed only when packet loss is detected. \nImproves experience on networks with packet loss.",
        "_root_connection_onConnectScript.name": "On connect script",
        "_root_connection_onConnectScript.description": "This script/executable will be run asynchronously when headset connects.\nEnvironment variable ACTION will be set to &#34;connect&#34; (without quotes).",
        "_root_connection_onDisconnectScript.name": "On disconnect script",
        "_root_connection_onDisconnectScript.description": "This script/executable will be run asynchronously when headset disconnects and on SteamVR shutdown.\nEnvironment variable ACTION will be set to &#34;disconnect&#34; (without quotes).",
        // Extra tab
        "_root_extra_tab.name": "Extra",
        "_root_extra_theme-choice-.name": "Theme",
        "_root_extra_theme-choice-.description": "Come to the Dark Side. \nWe have cookies.",
        "_root_extra_theme_systemDefault-choice-.name": "System",
        "_root_extra_theme_classic-choice-.name": "Classic",
        "_root_extra_theme_darkly-choice-.name": "Darkly",
        "_root_extra_clientDarkMode.name": "Client dark mode",
        "_root_extra_clientDarkMode.description": "Applied after connection and sleep-wake cycle",
        "_root_extra_revertConfirmDialog.name": "Confirm revert",
        "_root_extra_revertConfirmDialog.description": "Ask for confirmation before reverting settings to default value",
        "_root_extra_restartConfirmDialog.name": "Confirm SteamVR restart",
        "_root_extra_promptBeforeUpdate.name": "Prompt before update",
        "_root_extra_updateChannel-choice-.name": "Update channel",
        "_root_extra_updateChannel_noUpdates-choice-.name": "No updates",
        "_root_extra_updateChannel_stable-choice-.name": "Stable",
        "_root_extra_updateChannel_beta-choice-.name": "Beta",
        "_root_extra_updateChannel_nightly-choice-.name": "Nightly",
        "_root_extra_logToDisk.name": "Log to disk (session_log.txt)",
        "_root_extra_notificationLevel-choice-.name": "Notification level", // adv
        "_root_extra_notificationLevel-choice-.description": "At which level notification will be generated. From less details to all details: \n- Error \n- Warning \n- Informations \n- Debug", // adv
        "_root_extra_notificationLevel_error-choice-.name": "Error", // adv
        "_root_extra_notificationLevel_warning-choice-.name": "Warning", // adv
        "_root_extra_notificationLevel_info-choice-.name": "Information", // adv
        "_root_extra_notificationLevel_debug-choice-.name": "Debug", // adv
        "_root_extra_excludeNotificationsWithoutId.name": "Exclude notifications without identification", // adv
        "_root_extra_excludeNotificationsWithoutId.description": "Do not show notifications that do not contain the identification structure.", // adv
        // Others
        "steamVRRestartSuccess": "SteamVR successfully restarted",
        "audioDeviceError": "No audio devices found. Cannot stream audio or microphone",
    },
    "it": true,
    "sl": true,
    "es": true,
    "fr": true,
    "ja": true,
    "zh": true,
    "ru": true,
    "bg": true,
    "nl": true,
    "de": true,
    "ko": true,
});
