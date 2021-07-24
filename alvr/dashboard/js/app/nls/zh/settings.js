define({
    // Video tab
    "_root_video_tab.name": "视频",
    "_root_video_tab.description": "视频设置",
    "_root_video_adapterIndex.name": "显卡序号", // adv
    "_root_video_adapterIndex.description": "显卡序号", // adv
    "_root_video_displayRefreshRate.name": "刷新率",
    "_root_video_displayRefreshRate.description":
        "为SteamVR和头显设置刷新率。 即使头显不支持，SteamVR也会使用该刷新率。 较高的刷新率需要性能更高的电脑。 Quest初代的最大刷新率为72 Hz。",
    "_root_video_preferredFps.name": "刷新率", // adv
    "_root_video_preferredFps.description": "在头显上使用的刷新率", // adv
    "_root_video_resolutionDropdown.name": "视频分辨率",
    "_root_video_resolutionDropdown.description": `100％则是Oculus Quest的原始2880x1600分辨率。 \n设置分辨率可以改善视觉质量,但不建议这样做。 \n低于100％的分辨率可以减少延迟并提高网络性能。`,
    "_root_video_renderResolution-choice-.name": "视频分辨率", // adv
    "_root_video_renderResolution_scale-choice-.name": "使用视频缩放比例", // adv
    "_root_video_renderResolution_scale-choice-.description": "视频分辨率的比例因子", // adv
    "_root_video_renderResolution_absolute-choice-.name": "使用绝对视频分辨率", // adv
    "_root_video_renderResolution_absolute-choice-.description": "使用绝对视频分辨率", // adv
    "_root_video_renderResolution_scale.name": "比例尺", // adv
    "_root_video_renderResolution_scale.description": "视频比例尺", // adv
    "_root_video_recommendedTargetResolution-choice-.name": "目标帧分辨率", // adv
    "_root_video_recommendedTargetResolution-choice-.description":
        "已请求SteamVR进行渲染的帧分辨率", // adv
    "_root_video_recommendedTargetResolution_scale-choice-.name": "使用目标分辨率比例", // adv
    "_root_video_recommendedTargetResolution_absolute-choice-.name": "使用绝对目标分辨率", // adv
    "_root_video_recommendedTargetResolution_scale.name": "比例", // adv
    "_root_video_recommendedTargetResolution_scale.description": "比例尺", // adv
    "_root_video_secondsFromVsyncToPhotons.name": "从垂直同步到帧画面的秒数", // adv
    "_root_video_secondsFromVsyncToPhotons.description": "从垂直同步事件到显示器产生的帧画面的时间", // adv
    "_root_video_foveatedRendering.name": "集中渲染",
    // "_root_video_foveatedRendering.description": use "_root_video_foveatedRendering_enabled.description"
    "_root_video_foveatedRendering_enabled.description":
        "以高分辨率渲染图像的中心,而以较低的分辨率渲染郊区。\n导致需要通过网络传输的视频分辨率大大降低。\n在相同比特率的情况下,较小的视频可以保留更多细节,并同时降低延迟。\nFFR会在视图的边缘造成一些视觉伪像,这些伪像或多或少会根据设置和游戏而变得可见。",
    "_root_video_foveatedRendering_content_strength.name": "强度",
    "_root_video_foveatedRendering_content_strength.description":
        "值越高,意味着朝向帧边缘的细节越少,伪像越多",
    "_root_video_foveatedRendering_content_shape.name": "形状", // adv
    "_root_video_foveatedRendering_content_shape.description": "集中渲染的形状", // adv
    "_root_video_foveatedRendering_content_verticalOffset.name": "垂直偏移",
    "_root_video_foveatedRendering_content_verticalOffset.description":
        "较高的值表示高质量的帧区域进一步向下移动",
    "_root_video_colorCorrection.name": "色彩校正",
    // "_root_video_colorCorrection.description": use "_root_video_colorCorrection_enabled.description"
    "_root_video_colorCorrection_enabled.description":
        "按以下顺序应用颜色转换:锐化,伽玛,亮度,对比度,饱和度",
    "_root_video_colorCorrection_content_brightness.name": "亮度",
    "_root_video_colorCorrection_content_brightness.description":
        "亮度:范围[-1; 1],默认值为0。-1完全为黑色,1完全为白色",
    "_root_video_colorCorrection_content_contrast.name": "对比度",
    "_root_video_colorCorrection_content_contrast.description":
        "对比度:范围[-1; 1],默认值为0。-1完全是灰色的",
    "_root_video_colorCorrection_content_saturation.name": "饱和度",
    "_root_video_colorCorrection_content_saturation.description":
        "饱和度:范围[-1; 1],默认值为0。-1为黑白",
    "_root_video_colorCorrection_content_gamma.name": "伽马",
    "_root_video_colorCorrection_content_gamma.description":
        "伽玛:范围[0; 5],默认值为1。使用值2.2校正从sRGB到RGB空间的颜色",
    "_root_video_colorCorrection_content_sharpening.name": "锐化",
    "_root_video_colorCorrection_content_sharpening.description":
        "锐化:范围[-1; 5],默认为0。-1是最模糊的,5是最锐利的",
    "_root_video_codec-choice-.name": "视频编解码器",
    "_root_video_codec-choice-.description":
        "使用的视频编解码器 \n如果可能,请选择HEVC以在较低比特率下获得更好的视觉质量",
    "_root_video_codec_H264-choice-.name": "H.264（性能优先）",
    "_root_video_codec_HEVC-choice-.name": "HEVC（画质优先）",
    "_root_video_clientRequestRealtimeDecoder.name": "请求实时解码器优先级（客户端）", // adv
    "_root_video_encodeBitrateMbs.name": "视频比特率",
    "_root_video_encodeBitrateMbs.description": "视频流的比特率。建议30Mbps",
    // Audio tab
    "_root_audio_tab.name": "音频",
    "_root_audio_tab.description": "音频设置",
    "_root_audio_gameAudio.name": "串流电脑声音",
    // "_root_audio_gameAudio.description": use "_root_audio_gameAudio_enabled.description"
    "_root_audio_gameAudio_enabled.description": "启用则会将游戏音频流传输到头戴显示器",
    "_root_audio_gameAudio_content_deviceId-choice-.name": "选择 音频 设备",
    "_root_audio_gameAudio_content_deviceId-choice-.description": "用于 捕获 音频 的 音频 设备",
    // "_root_audio_gameAudio_content_muteWhenStreaming.name": "",
    // "_root_audio_gameAudio_content_muteWhenStreaming.description": "",
    "_root_audio_microphone.name": "麦克风串流",
    // "_root_audio_microphone.description": use "_root_audio_microphone_enabled.description"
    "_root_audio_microphone_enabled.description": "用于流式传输的麦克风",
    "_root_audio_microphone_content_deviceId-choice-.name": "选择 虚拟 麦克风 设备",
    "_root_audio_microphone_content_deviceId-choice-.description":
        "为 确保 麦克风 可以 正常 使用 ， 你 需要 安装 VB-Audio Virtual device 或者 其他 相似 的 软件。",
    // Headset tab
    "_root_headset_tab.name": "头部显示器",
    "_root_headset_headsetEmulationMode.name": "头戴显示器模拟模式",
    "_root_headset_headsetEmulationMode.description": "使用不同的头戴显示器以获得更好的兼容性",
    // "_root_headset_universeId.name": "", // adv
    "_root_headset_serialNumber.name": "序列号", // adv
    "_root_headset_serialNumber.description": "用于模拟头戴显示器的序列号", // adv
    "_root_headset_trackingSystemName.name": "跟踪系统名称", // adv
    "_root_headset_trackingSystemName.description": "模拟的跟踪系统的名称", // adv
    "_root_headset_modelNumber.name": "型号", // adv
    "_root_headset_modelNumber.description": "模拟头戴显示器的型号", // adv
    "_root_headset_driverVersion.name": "驱动程序版本", // adv
    "_root_headset_driverVersion.description": "模拟头戴显示器的驱动程序版本", // adv
    "_root_headset_manufacturerName.name": "制造商名称", // adv
    "_root_headset_manufacturerName.description": "模拟头戴显示器的制造商名称", // adv
    "_root_headset_renderModelName.name": "渲染模型名称", // adv
    "_root_headset_renderModelName.description": "使用的渲染模块的名称", // adv
    "_root_headset_registeredDeviceType.name": "注册设备类型", // adv
    "_root_headset_registeredDeviceType.description": "已注册设备的类型", // adv
    "_root_headset_trackingFrameOffset.name": "跟踪帧偏移",
    "_root_headset_trackingFrameOffset.description": "姿势预测算法的偏移量",
    "_root_headset_positionOffset.name": "头戴显示器位置偏移", // adv
    "_root_headset_positionOffset.description": "报告位置的偏移量", // adv
    "_root_headset_positionOffset_0.name": "x", // adv
    "_root_headset_positionOffset_0.description": "X偏移量", // adv
    "_root_headset_positionOffset_1.name": "y", // adv
    "_root_headset_positionOffset_1.description": "Y偏移量", // adv
    "_root_headset_positionOffset_2.name": "z", // adv
    "_root_headset_positionOffset_2.description": "Z偏移", // adv
    "_root_headset_force3dof.name": "强制3自由度",
    "_root_headset_force3dof.description": "强制3个自由度模式（例如Oculus Go）",
    "_root_headset_controllers.name": "控制器",
    // "_root_headset_controllers.description": use "_root_headset_controllers_enabled.description"
    "_root_headset_controllers_enabled.description": "启用控制器的使用",
    "_root_headset_controllers_content_controllerMode.name": "控制器仿真模式",
    "_root_headset_controllers_content_controllerMode.description":
        "使用不同的控制器以获得更好的兼容性或启用手部跟踪",
    "_root_headset_controllers_content_modeIdx.name": "模式", // adv
    "_root_headset_controllers_content_modeIdx.description": "控制器模式索引", // adv
    "_root_headset_controllers_content_trackingSystemName.name": "跟踪系统名称", // adv
    "_root_headset_controllers_content_trackingSystemName.description": "模拟跟踪系统的名称", // adv
    "_root_headset_controllers_content_manufacturerName.name": "制造商名称", // adv
    "_root_headset_controllers_content_manufacturerName.description": "控制器制造商的名称", // adv
    "_root_headset_controllers_content_modelNumber.name": "型号", // adv
    "_root_headset_controllers_content_modelNumber.description": "控制器型号", // adv
    "_root_headset_controllers_content_renderModelNameLeft.name": "渲染模型名称（左）", // adv
    "_root_headset_controllers_content_renderModelNameLeft.description": "左控制器的渲染模型的名称", // adv
    "_root_headset_controllers_content_renderModelNameRight.name": "渲染模型名称（右）", // adv
    "_root_headset_controllers_content_renderModelNameRight.description":
        "右控制器的渲染模型的名称", // adv
    "_root_headset_controllers_content_serialNumber.name": "序列号", // adv
    "_root_headset_controllers_content_serialNumber.description": "控制器的序列号", // adv
    "_root_headset_controllers_content_registeredDeviceType.name": "设备类型名称", // adv
    "_root_headset_controllers_content_registeredDeviceType.description": "模拟设备类型的名称", // adv
    "_root_headset_controllers_content_inputProfilePath.name": "配置文件路径", // adv
    "_root_headset_controllers_content_inputProfilePath.description": "输入配置文件的路径", // adv
    "_root_headset_controllers_content_trackingSpeed.name": "追踪速度",
    "_root_headset_controllers_content_trackingSpeed.description":
        "像需要快速运动的游戏比如《光剑》，选择中速或快速。 运动比较慢的游戏比如《Skyrim》，使用正常即可。",
    "_root_headset_controllers_content_poseTimeOffset.name": "姿势时间偏移", // adv
    "_root_headset_controllers_content_poseTimeOffset.description": "姿势预测算法的偏移量", // adv
    "_root_headset_controllers_content_positionOffsetLeft.name": "位置偏移", // adv
    "_root_headset_controllers_content_positionOffsetLeft.description":
        "左侧控制器的位置偏移（以米为单位）。\n对于右侧控制器,x值被镜像", // adv
    "_root_headset_controllers_content_positionOffsetLeft_0.name": "x", // adv
    "_root_headset_controllers_content_positionOffsetLeft_0.description": "X偏移", // adv
    "_root_headset_controllers_content_positionOffsetLeft_1.name": "y", // adv
    "_root_headset_controllers_content_positionOffsetLeft_1.description": "Y偏移量", // adv
    "_root_headset_controllers_content_positionOffsetLeft_2.name": "z", // adv
    "_root_headset_controllers_content_positionOffsetLeft_2.description": "Z偏移", // adv
    "_root_headset_controllers_content_rotationOffsetLeft.name": "旋转偏移", // adv
    "_root_headset_controllers_content_rotationOffsetLeft.description":
        "左控制器的旋转偏移量,以度为单位。\n对于右控制器,沿Y和Z轴的旋转是镜像的", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_0.name": "x", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_0.description": "Y旋转", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_1.name": "y", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_1.description": "Y旋转", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_2.name": "z", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_2.description": "Z向旋转", // adv
    "_root_headset_controllers_content_hapticsIntensity.name": "触觉强度",
    "_root_headset_controllers_content_hapticsIntensity.description": "增加触觉反馈的因素",
    "_root_headset_trackingSpace-choice-.name": "安全区中心",
    "_root_headset_trackingSpace-choice-.description":
        "设置头显使用什么作为跟踪的参考，以及如何定义空间中心。",
    "_root_headset_trackingSpace_local-choice-.name": "头显所在的位置",
    "_root_headset_trackingSpace_stage-choice-.name": "安全区中心点所在的位置",
    // Connection tab
    "_root_connection_tab.name": "连接",
    // "_root_connection_autoTrustClients.name": "", // adv
    // "_root_connection_webServerPort.name": "",
    "_root_connection_streamPort.name": "服务器端口", // adv
    "_root_connection_streamPort.description": "服务器监听端口", // adv
    "_root_connection_aggressiveKeyframeResend.name": "重发关键帧",
    "_root_connection_aggressiveKeyframeResend.description":
        "将关键帧之间的最小间隔从100毫秒减小到5毫秒。\n仅在检测到数据包丢失时使用。可改善数据包丢失的网络体验。",
    // Extra tab
    "_root_extra_tab.name": "其他",
    "_root_extra_theme-choice-.name": "界面主题",
    "_root_extra_theme-choice-.description": "切换界面主题风格。",
    "_root_extra_theme_systemDefault-choice-.name": "系统",
    "_root_extra_theme_classic-choice-.name": "亮白",
    "_root_extra_theme_darkly-choice-.name": "暗黑",
    // "_root_extra_clientDarkMode.name": "",
    // "_root_extra_clientDarkMode.description": "",
    "_root_extra_revertConfirmDialog.name": "确认还原",
    "_root_extra_revertConfirmDialog.description": "在将设置恢复为默认值之前弹出用户确认对话框。",
    "_root_extra_restartConfirmDialog.name": "确认重启",
    "_root_extra_restartConfirmDialog.description": "在重新启动SteamVR之前弹出用户确认对话框。",
    // "_root_extra_promptBeforeUpdate.name": "",
    // "_root_extra_updateChannel-choice-.name": "",
    // "_root_extra_updateChannel_noUpdates-choice-.name": "",
    // "_root_extra_updateChannel_stable-choice-.name": "",
    // "_root_extra_updateChannel_beta-choice-.name": "",
    // "_root_extra_updateChannel_nightly-choice-.name": "",
    // "_root_extra_logToDisk.name": "",
    "_root_extra_notificationLevel-choice-.name": "通知级别", // adv
    "_root_extra_notificationLevel-choice-.description": "将触发gui通知的日志记录级别", // adv
    "_root_extra_notificationLevel_error-choice-.name": "错误", // adv
    "_root_extra_notificationLevel_warning-choice-.name": "警告", // adv
    "_root_extra_notificationLevel_info-choice-.name": "信息", // adv
    "_root_extra_notificationLevel_debug-choice-.name": "调试", // adv
    "_root_extra_excludeNotificationsWithoutId.name": "排除没有ID的通知", // adv
    "_root_extra_excludeNotificationsWithoutId.description": "不显示没有ID的通知", // adv
    // Others
    steamVRRestartSuccess: "SteamVR成功重启",
    // "audioDeviceError": "",
});
