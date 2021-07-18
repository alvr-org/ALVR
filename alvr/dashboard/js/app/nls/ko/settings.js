define({
    // Video tab
    "_root_video_tab.name": "비디오",
    "_root_video_adapterIndex.name": "GPU 번호", // adv
    "_root_video_displayRefreshRate.name": "재생 속도",
    "_root_video_displayRefreshRate.description":
        "SteamVr과 헤드셋 재생 속도를 설정합니다. 높은 재생속도는 빠른 PC를 요구합니다. 72 Hz는 퀘스트 1의 최대값입니다.",
    "_root_video_preferredFps.name": "커스텀 재생 속도", // adv
    "_root_video_resolutionDropdown.name": "비디오 해상도",
    "_root_video_resolutionDropdown.description":
        "100%는 오큘러스의 실제 해상도와 동일합니다. \n해상도를 높이는 것은 품질향상에 도움이 되지만 권장하지 않습니다. \n해상도를 낮추는 것은 지연시간을 줄이고 네트워크 성능에 도움이 됩니다.",
    "_root_video_renderResolution-choice-.name": "비디오 인코딩 해상도", // adv
    "_root_video_renderResolution_scale-choice-.name": "상대값", // adv
    "_root_video_renderResolution_absolute-choice-.name": "절대값", // adv
    "_root_video_renderResolution_scale.name": "상대값", // adv
    "_root_video_recommendedTargetResolution-choice-.name": "게임 렌더링 해상도", // adv
    "_root_video_recommendedTargetResolution_scale-choice-.name": "상대값", // adv
    "_root_video_recommendedTargetResolution_absolute-choice-.name": "절대값", // adv
    "_root_video_recommendedTargetResolution_scale.name": "상대값", // adv
    "_root_video_secondsFromVsyncToPhotons.name": "수직 동기화 지연시간", // adv
    "_root_video_secondsFromVsyncToPhotons.description":
        "이미지가 표시될 때까지 가상 수직 동기화에서 대기하는 시간", // adv
    "_root_video_foveatedRendering.name": "Foveated 렌더링",
    // "_root_video_foveatedRendering.description": use "_root_video_foveatedRendering_enabled.description"
    "_root_video_foveatedRendering_enabled.description":
        "GPU 부하를 줄이기 위해 시야 바깥에 있는 이미지의 해상도를 줄이는 렌더링 기술.  네트워크를 통해 전송해야 하는 비디오 해상도가 낮아집니다.",
    "_root_video_foveatedRendering_content_strength.name": "강도",
    "_root_video_foveatedRendering_content_strength.description":
        "값이 높으면 시야 바깥이 흐려지고 잡음을 유발합니다",
    "_root_video_foveatedRendering_content_shape.name": "선명도", // adv
    "_root_video_foveatedRendering_content_shape.description": "Foveated 렌더링 선명도", // adv
    "_root_video_foveatedRendering_content_verticalOffset.name": "수직 오프셋",
    "_root_video_foveatedRendering_content_verticalOffset.description":
        "값이 높으면 고품질 프레임 영역이 아래로 이동됩니다.",
    "_root_video_colorCorrection.name": "색 보정",
    // "_root_video_colorCorrection.description": use "_root_video_colorCorrection_enabled.description"
    "_root_video_colorCorrection_enabled.description":
        "색상 보정은 다음 순서로 적용됩니다. 선명도, 감마, 밝기, 대비 및 채도입니다.",
    "_root_video_colorCorrection_content_brightness.name": "밝기",
    "_root_video_colorCorrection_content_brightness.description":
        "밝기: -1은 완전 검은색, 1은 완전 흰색입니다.",
    "_root_video_colorCorrection_content_contrast.name": "대비",
    "_root_video_colorCorrection_content_contrast.description": "대비: -1은 완전히 회색입니다.",
    "_root_video_colorCorrection_content_saturation.name": "채도",
    "_root_video_colorCorrection_content_saturation.description": "채도: -1은 흑백입니다.",
    "_root_video_colorCorrection_content_gamma.name": "감마",
    "_root_video_colorCorrection_content_gamma.description":
        "감마: 2.2의 값을 사용하여 sRGB에서 RGB로 색상을 수정합니다. 이것은 밝기를 조절하지만 검은색은 검은색, 흰색은 흰색으로 유지합니다.",
    "_root_video_colorCorrection_content_sharpening.name": "선명도",
    "_root_video_colorCorrection_content_sharpening.description":
        "선명도: 이미지의 경계를 강조합니다.",
    "_root_video_codec-choice-.name": "비디오 코덱",
    "_root_video_codec-choice-.description":
        "HEVC은 낮은 비트레이트에서 좋은 품질을 유지합니다. AMD 그래픽카드는 HEVC와 알맞습니다.",
    "_root_video_codec_H264-choice-.name": "h264",
    "_root_video_codec_HEVC-choice-.name": "HEVC (h265)",
    "_root_video_clientRequestRealtimeDecoder.name": "최우선 실시간 디코더 요청 (클라이언트)", // adv
    "_root_video_use10bitEncoder.name": "10비트 인코더 사용하기 (최신 엔비디아 그래픽 카드만)", // adv
    "_root_video_use10bitEncoder.description": "컬러 밴딩을 줄여 비디오 품질을 향상시킵니다",
    "_root_video_encodeBitrateMbs.name": "비디오 비트레이트",
    "_root_video_encodeBitrateMbs.description":
        "비디오 전송 비트레이트 입니다. 30Mbps를 권장합니다. \n높은 비트레이트는 품질향상에 좋지만 지연시간 증가와 네트워크 성능에 악영향을 줍니다. ",
    // Audio tab
    "_root_audio_tab.name": "오디오",
    "_root_audio_gameAudio.name": "게임 오디오 전송",
    // "_root_audio_gameAudio.description": use "_root_audio_gameAudio_enabled.description"
    "_root_audio_gameAudio_enabled.description": "게임 오디오를 헤드셋으로 전송합니다.",
    "_root_audio_gameAudio_content_deviceDropdown.name": "오디오 장치를 선택하세요",
    "_root_audio_gameAudio_content_deviceId-choice-.name": "오디오 장치",
    "_root_audio_gameAudio_content_deviceId_default-choice-.name": "기본",
    "_root_audio_gameAudio_content_deviceId_name-choice-.name": "이름으로",
    "_root_audio_gameAudio_content_deviceId_index-choice-.name": "번호로",
    "_root_audio_gameAudio_content_muteWhenStreaming.name": "전송시 외부장치 음소거하기",
    "_root_audio_gameAudio_content_muteWhenStreaming.description":
        "헤드셋으로 전송할 때 오디오 출력(스피커/헤드폰)을 음소거합니다. 물리적 출력만 음소거되고(하울링 방지), 헤드셋 및 기타 캡처 소프트웨어는 영향을 받지 않습니다.",
    "_root_audio_gameAudio_content_config.name": "설정",
    "_root_audio_gameAudio_content_config_averageBufferingMs.name": "지연시간 (ms)",
    "_root_audio_gameAudio_content_config_averageBufferingMs.description":
        "지연시간이 길수록 오디오 깨짐이 줄어듭니다.",
    "_root_audio_microphone.name": "헤드셋 마이크 전송",
    // "_root_audio_microphone.description": use "_root_audio_microphone_enabled.description"
    "_root_audio_microphone_enabled.description":
        "헤드셋 마이크를 SteamVr로 전송합니다. \nVB-CABLE Virtual Audio Device 혹은 유사한 가상 오디오 장치가 필요합니다.\nThe virtual microphone input is the recording device, the virtual microphone output is the audio rendering device, which is used to configure SteamVR microphone.",
    "_root_audio_microphone_content_inputDeviceDropdown.name": "가상 마이크 입력 장치를 선택",
    "_root_audio_microphone_content_inputDeviceDropdown.description":
        "Output device used to render the microphone audio.",
    "_root_audio_microphone_content_inputDeviceId-choice-.name": "가상 마이크 입력",
    "_root_audio_microphone_content_inputDeviceId_default-choice-.name": "기본",
    "_root_audio_microphone_content_inputDeviceId_name-choice-.name": "이름으로",
    "_root_audio_microphone_content_inputDeviceId_index-choice-.name": "번호로",
    "_root_audio_microphone_content_outputDeviceDropdown.name": "가상 마이크 출력 장치를 선택",
    "_root_audio_microphone_content_outputDeviceDropdown.description":
        "Input device used as microphone. This is used to configure SteamVR microphone.",
    "_root_audio_microphone_content_outputDeviceId-choice-.name": "가상 마이크 출력",
    "_root_audio_microphone_content_outputDeviceId_default-choice-.name": "기본",
    "_root_audio_microphone_content_outputDeviceId_name-choice-.name": "이름으로",
    "_root_audio_microphone_content_outputDeviceId_index-choice-.name": "번호로",
    "_root_audio_microphone_content_config.name": "설정",
    "_root_audio_microphone_content_config_averageBufferingMs.name": "지연시간 (ms)",
    "_root_audio_microphone_content_config_averageBufferingMs.description":
        "지연시간이 길수록 오디오 깨짐이 줄어듭니다.",
    // Headset tab
    "_root_headset_tab.name": "헤드셋",
    "_root_headset_headsetEmulationMode.name": "헤드셋 에뮬레이션 방식",
    "_root_headset_headsetEmulationMode.description":
        "더 좋은 호환성을 위해 다른 헤드셋을 에뮬레이션합니다.",
    "_root_headset_universeId.name": "Universe ID", // adv
    "_root_headset_serialNumber.name": "시리얼 숫자", // adv
    "_root_headset_serialNumber.description": "에뮬레이션 헤드셋의 시리얼 숫자", // adv
    "_root_headset_trackingSystemName.name": "트래킹 시스템 이름", // adv
    "_root_headset_trackingSystemName.description": "에뮬레이션 트래킹 시스템의 이름", // adv
    "_root_headset_modelNumber.name": "모델숫자", // adv
    "_root_headset_modelNumber.description": "에뮬레이션 헤드셋의 모델숫자", // adv
    "_root_headset_driverVersion.name": "드라이버 버전", // adv
    "_root_headset_driverVersion.description": "에뮬레이션 헤드셋의 드라이버 버전", // adv
    "_root_headset_manufacturerName.name": "제조사", // adv
    "_root_headset_manufacturerName.description": "에뮬레이션 헤드셋의 제조사", // adv
    "_root_headset_renderModelName.name": "렌더 모델 이름", // adv
    "_root_headset_renderModelName.description": "에뮬레이션 헤드셋의 렌더 모델 이름", // adv
    "_root_headset_registeredDeviceType.name": "등록된 기기 타입", // adv
    "_root_headset_registeredDeviceType.description": "에뮬레이션 헤드셋의 등록된 기기 타입", // adv
    "_root_headset_trackingFrameOffset.name": "트래킹 프레임 오프셋",
    "_root_headset_trackingFrameOffset.description": "포즈 예측 알고리즘 오프셋",
    "_root_headset_positionOffset.name": "헤드셋 위치 오프셋", // adv
    "_root_headset_positionOffset.description": "헤드셋 위치 예측 알고리즘 오프셋", // adv
    "_root_headset_positionOffset_0.name": "X", // adv
    "_root_headset_positionOffset_1.name": "Y", // adv
    "_root_headset_positionOffset_2.name": "Z", // adv
    "_root_headset_force3dof.name": "강제 3Dof(3축)",
    "_root_headset_force3dof.description": "강제 3축모드를 사용합니다.(Oculus Go와 유사)",
    "_root_headset_controllers.name": "컨트롤러",
    // "_root_headset_controllers.description": use "_root_headset_controllers_enabled.description"
    "_root_headset_controllers_enabled.description": "컨트롤러를 사용합니다.",
    "_root_headset_controllers_content_controllerMode.name": "컨트롤러 에뮬레이션 방식",
    "_root_headset_controllers_content_controllerMode.description":
        "호환성을 위해 다른 컨트롤러를 에뮬레이션 하거나 핸드트래킹을 이용할 수 있습니다.",
    "_root_headset_controllers_content_modeIdx.name": "Mode Index", // adv
    "_root_headset_controllers_content_modeIdx.description": "에뮬레이터 컨트롤러의 Mode index", // adv
    "_root_headset_controllers_content_trackingSystemName.name": "트래킹 시스템 이름", // adv
    "_root_headset_controllers_content_trackingSystemName.description":
        "에뮬레이터 컨트롤러 트래킹 시스템의 이름", // adv
    "_root_headset_controllers_content_manufacturerName.name": "제조사 이름", // adv
    "_root_headset_controllers_content_manufacturerName.description":
        "에뮬레이터 컨트롤러의 제조사 이름", // adv
    "_root_headset_controllers_content_modelNumber.name": "모델 숫자", // adv
    "_root_headset_controllers_content_modelNumber.description": "에뮬레이터 컨트롤러의 모델 숫자", // adv
    "_root_headset_controllers_content_renderModelNameLeft.name": "모델 숫자 (왼손)", // adv
    "_root_headset_controllers_content_renderModelNameLeft.description":
        "왼손 에뮬레이터 컨트롤러의 모델 숫자", // adv
    "_root_headset_controllers_content_renderModelNameRight.name": "모델 숫자 (오른손)", // adv
    "_root_headset_controllers_content_renderModelNameRight.description":
        "오른손 에뮬레이터 컨트롤러의 모델 숫자", // adv
    "_root_headset_controllers_content_serialNumber.name": "시리얼 숫자", // adv
    "_root_headset_controllers_content_serialNumber.description":
        "에뮬레이터 컨트롤러의 시리얼 숫자", // adv
    "_root_headset_controllers_content_registeredDeviceType.name": "등록된 디바이스 타입", // adv
    "_root_headset_controllers_content_registeredDeviceType.description":
        "에뮬레이터 컨트롤러의 등록된 디바이스 타입", // adv
    "_root_headset_controllers_content_inputProfilePath.name": "입력 프로파일 경로", // adv
    "_root_headset_controllers_content_inputProfilePath.description":
        "에뮬레이터 컨트롤러의 입력 프로파일 경로", // adv
    "_root_headset_controllers_content_trackingSpeed.name": "트래킹 속도",
    "_root_headset_controllers_content_trackingSpeed.description":
        "비트세이버처럼 빠른 속도의 게임을 플레이하신다면 보통 혹은 중간을 선택하세요. 스카이림처럼 느린 게임을 플레이하신다면 보통을 선택하세요. \n 오큘러스 예측은 SteamVR 대신 헤드셋에서 컨트롤러 위치를 예측합니다.",
    "_root_headset_controllers_content_poseTimeOffset.name": "포즈 타임 오프셋", // adv
    "_root_headset_controllers_content_poseTimeOffset.description": "포즈 예측 알고리즘 오프셋", // adv
    "_root_headset_controllers_content_positionOffsetLeft.name": "위치 오프셋", // adv
    "_root_headset_controllers_content_positionOffsetLeft.description":
        "왼쪽 컨트롤러의 위치 오프셋(미터)입니다. \n오른쪽 컨트롤러와 x 값이 반대입니다.", // adv
    "_root_headset_controllers_content_positionOffsetLeft_0.name": "X", // adv
    "_root_headset_controllers_content_positionOffsetLeft_1.name": "Y", // adv
    "_root_headset_controllers_content_positionOffsetLeft_2.name": "Z", // adv
    "_root_headset_controllers_content_rotationOffsetLeft.name": "회전 오프셋", // adv
    "_root_headset_controllers_content_rotationOffsetLeft.description":
        "왼쪽 컨트롤러의 회전각도 오프셋(도)입니다. \n오른쪽 컨트롤러와  Y Z 축이 반대입니다.", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_0.name": "X", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_1.name": "Y", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_2.name": "Z", // adv
    "_root_headset_controllers_content_hapticsIntensity.name": "진동 세기",
    "_root_headset_controllers_content_hapticsIntensity.description": "진동 세기를 조절합니다.",
    "_root_headset_trackingSpace-choice-.name": "트래킹 공간",
    "_root_headset_trackingSpace-choice-.description":
        "헤드셋이 추적을 위해 사용하는 항목과 공간 중심이 정의되는 방법을 설정합니다. Stage 추적은 유선 헤드셋과 같이 동작합니다. 헤드셋 recentering 후 공간의 중심이 일정하게 유지됩니다. Vive 추적기를 사용하려면 Stage 설정을 해야 합니다.",
    "_root_headset_trackingSpace_local-choice-.name": "Local (헤드셋 중심)",
    "_root_headset_trackingSpace_stage-choice-.name": "Stage (방 중심)",
    // Connection tab
    "_root_connection_tab.name": "연결",
    "_root_connection_autoTrustClients.name": "자동으로 클라이언트 신뢰(권장하지 않음)", // adv
    "_root_connection_webServerPort.name": "웹 서버 포트",
    "_root_connection_streamProtocol-choice-.name": "전송 프로토콜",
    "_root_connection_streamProtocol-choice-.description":
        "네트워크 프로토콜은 서버와 클라이언트 사이의 데이터 전송 방식입니다. UDP는 낮은 비트레이트에서 좋습니다. (<30), Throttled UDP 중간 비트레이트에서 좋습니다. (~100), TCP 어느 비트레이트에서나 작동합니다.",
    "_root_connection_streamProtocol_udp-choice-.name": "UDP",
    "_root_connection_streamProtocol_throttledUdp-choice-.name": "Throttled UDP",
    "_root_connection_streamProtocol_tcp-choice-.name": "TCP",
    "_root_connection_streamPort.name": "서버 전송 포트", // adv
    "_root_connection_streamPort.description": "서버가 패킷을 수신하는 데 사용하는 포트입니다.", // adv
    "_root_connection_aggressiveKeyframeResend.name": "적극적인 키 프레임 재전송",
    "_root_connection_aggressiveKeyframeResend.description":
        "키 프레임 사이의 최소 간격을 100ms에서 5ms로 감소. \n패킷 손실이 감지될 때만 사용됩니다. \n패킷 손실이 있는 네트워크에 효과가 있습니다.",
    "_root_connection_onConnectScript.name": "연결 스크립트 사용",
    "_root_connection_onConnectScript.description":
        "이 스크립트/실행 파일은 헤드셋이 연결될 때 비동기식으로 실행됩니다.\nEnvironment variable ACTION will be set to &#34;connect&#34; (without quotes).",
    "_root_connection_onDisconnectScript.name": "연결 끊기 스크립트 사용",
    "_root_connection_onDisconnectScript.description":
        "이 스크립트/실행 파일은 헤드셋의 연결이 끊기거나 SteamVr이 종료될 때 비동기식으로 실행됩니다..\nEnvironment variable ACTION will be set to &#34;disconnect&#34; (without quotes).",
    // Extra tab
    "_root_extra_tab.name": "기타",
    "_root_extra_theme-choice-.name": "테마",
    "_root_extra_theme-choice-.description": "Come to the Dark Side. \nWe have cookies.",
    "_root_extra_theme_systemDefault-choice-.name": "시스템",
    "_root_extra_theme_classic-choice-.name": "클래식",
    "_root_extra_theme_darkly-choice-.name": "다크",
    "_root_extra_clientDarkMode.name": "클라이언트 다크 모드",
    "_root_extra_clientDarkMode.description": "Applied after connection and sleep-wake cycle",
    "_root_extra_revertConfirmDialog.name": "초기화 확인",
    "_root_extra_revertConfirmDialog.description": "기본값으로 초기화할 때 다시 한번 확인합니다.",
    "_root_extra_restartConfirmDialog.name": "SteamVr 재시작 확인",
    "_root_extra_promptBeforeUpdate.name": "업데이트 전 확인 팝업 표시",
    "_root_extra_updateChannel-choice-.name": "업데이트 채널",
    "_root_extra_updateChannel_noUpdates-choice-.name": "업데이트 없음",
    "_root_extra_updateChannel_stable-choice-.name": "안정",
    "_root_extra_updateChannel_beta-choice-.name": "베타",
    "_root_extra_updateChannel_nightly-choice-.name": "나이틀리",
    "_root_extra_logToDisk.name": "로그 저장 (session_log.txt)",
    "_root_extra_notificationLevel-choice-.name": "알림 수준", // adv
    "_root_extra_notificationLevel-choice-.description":
        "알림이 생성되는 수준입니다. 대략적 정보에서 모든 세부 정보까지: \n- Error \n- Warning \n- Informations \n- Debug", // adv
    "_root_extra_notificationLevel_error-choice-.name": "Error", // adv
    "_root_extra_notificationLevel_warning-choice-.name": "Warning", // adv
    "_root_extra_notificationLevel_info-choice-.name": "Information", // adv
    "_root_extra_notificationLevel_debug-choice-.name": "Debug", // adv
    "_root_extra_excludeNotificationsWithoutId.name": "식별 없이 알림 제외", // adv
    "_root_extra_excludeNotificationsWithoutId.description":
        "식별 구조가 포함되지 않은 알람은 표시하지 않습니다.", // adv
    // Others
    steamVRRestartSuccess: "SteamVr 재시작 성공",
    audioDeviceError: "오디오 장치가 확인되지 않았습니다. 오디오와 마이크를 사용할 수 없습니다.",
});
