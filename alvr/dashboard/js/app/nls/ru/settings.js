define({
    // Video tab
    "_root_video_tab.name": "Видео",
    "_root_video_adapterIndex.name": "Индекс GPU", // adv
    // "_root_video_displayRefreshRate.name": "",
    // "_root_video_displayRefreshRate.description": "",
    // "_root_video_preferredFps.name": "", // adv
    "_root_video_resolutionDropdown.name": "Разрешение видео",
    "_root_video_resolutionDropdown.description":
        "100% обеспечивает нативное разрешение Oculus Quest 2880x1600.\nИзменение разрешения не рекомендовано, однако оно может улучшить визуальные впечатления.\nРазрешение меньше 100% может уменьшить задержку и увеличить производительность сети.",
    "_root_video_renderResolution-choice-.name": "Изменение разрешения видео", // adv
    // "_root_video_renderResolution_scale-choice-.name": "", // adv
    // "_root_video_renderResolution_absolute-choice-.name": "", // adv
    // "_root_video_renderResolution_scale.name": "", // adv
    "_root_video_recommendedTargetResolution-choice-.name":
        "Предпочитаемое разрешение рендеринга игр", // adv
    // "_root_video_recommendedTargetResolution_scale-choice-.name": "", // adv
    // "_root_video_recommendedTargetResolution_absolute-choice-.name": "", // adv
    // "_root_video_recommendedTargetResolution_scale.name": "", // adv
    // "_root_video_secondsFromVsyncToPhotons.name": "", // adv
    // "_root_video_secondsFromVsyncToPhotons.description": "", // adv
    "_root_video_foveatedRendering.name": "Реновированный рендеринг",
    // "_root_video_foveatedRendering.description": use "_root_video_foveatedRendering_enabled.description"
    "_root_video_foveatedRendering_enabled.description":
        "Реновированный рендеринг (Foveated rendering) - техника рендеринга, при которой центр изображения обрабатывается в высоком разрешении, а периферийные участки - в значительно пониженном.\nОбеспечивает более низкое разрешение видео, передаваемого по сети.\nВидео меньшего объема при том же битрейте сохраняет больше деталей и, в то же время, уменьшает задержку.",
    "_root_video_foveatedRendering_content_strength.name": "Сила",
    "_root_video_foveatedRendering_content_strength.description":
        "Более высокое значение уменьшает кол-во деталей у краев поля видимости и увеличивает кол-во артефактов.",
    // "_root_video_foveatedRendering_content_shape.name": "", // adv
    "_root_video_foveatedRendering_content_shape.description": "Форма реновированного рендеринга.", // adv
    "_root_video_foveatedRendering_content_verticalOffset.name": "Вертикальный сдвиг",
    "_root_video_foveatedRendering_content_verticalOffset.description":
        "Более высокое значение обеспечивает смещение более качественной области кадра вниз.",
    "_root_video_colorCorrection.name": "Коррекция цвета",
    // "_root_video_colorCorrection.description": use "_root_video_colorCorrection_enabled.description"
    // "_root_video_colorCorrection_enabled.description": "",
    "_root_video_colorCorrection_content_brightness.name": "Яркость",
    // "_root_video_colorCorrection_content_brightness.description": "",
    "_root_video_colorCorrection_content_contrast.name": "Контраст",
    // "_root_video_colorCorrection_content_contrast.description": "",
    "_root_video_colorCorrection_content_saturation.name": "Насыщенность",
    // "_root_video_colorCorrection_content_saturation.description": "",
    "_root_video_colorCorrection_content_gamma.name": "Гамма",
    "_root_video_colorCorrection_content_gamma.description":
        "Также регулирует яркость, но сохраняет черный цвет черным, а белый - белым.",
    "_root_video_colorCorrection_content_sharpening.name": "Четкость",
    // "_root_video_colorCorrection_content_sharpening.description": "",
    "_root_video_codec-choice-.name": "Кодек видео",
    "_root_video_codec-choice-.description":
        "HEVC предпочитаем для получения более качественного изображения на низком битрейте. Лучше всего с HEVC с видеокартами AMD.",
    "_root_video_codec_H264-choice-.name": "h264",
    "_root_video_codec_HEVC-choice-.name": "HEVC (h265)",
    // "_root_video_clientRequestRealtimeDecoder.name": "", // adv
    "_root_video_encodeBitrateMbs.name": "Битрейт видео",
    "_root_video_encodeBitrateMbs.description":
        "Битрейт трансляции видео. 30Мб/с - рекомендуемый. \nБолее высокий битрейт обеспечивает более качественное изображение, но при этом повышается задержка и кол-во сетевого трафика.",
    // Audio tab
    "_root_audio_tab.name": "Аудио",
    "_root_audio_gameAudio.name": "Передача звука игры",
    // "_root_audio_gameAudio.description": use "_root_audio_gameAudio_enabled.description"
    // "_root_audio_gameAudio_enabled.description": "",
    "_root_audio_gameAudio_content_deviceId-choice-.name": "Выберите устройство воспроизведения",
    "_root_audio_gameAudio_content_deviceId-choice-.description":
        "Устройство воспроизведения используется для захвата звука.",
    // "_root_audio_gameAudio_content_muteWhenStreaming.name": "",
    // "_root_audio_gameAudio_content_muteWhenStreaming.description": "",
    "_root_audio_microphone.name": "Трансляция микрофона",
    // "_root_audio_microphone.description": use "_root_audio_microphone_enabled.description"
    "_root_audio_microphone.description": "Передает сигнал микрофона гарнитуры.",
    "_root_audio_microphone_content_deviceId-choice-.name": "Выберите виртуальное устройство ввода",
    "_root_audio_microphone_content_deviceId-choice-.description":
        "Чтобы ваш микрофон заработал, необходимо установить VB-CABLE Virtual Audio Device или другое аналогичное ПО.",
    // Headset tab
    "_root_headset_tab.name": "Гарнитура",
    "_root_headset_headsetEmulationMode.name": "Режим эмуляции гарнитуры",
    "_root_headset_headsetEmulationMode.description":
        "Эмулирует разные гарнитуры для лучшей совместимости.",
    // "_root_headset_universeId.name": "", // adv
    // "_root_headset_serialNumber.name": "", // adv
    // "_root_headset_serialNumber.description": "", // adv
    // "_root_headset_trackingSystemName.name": "", // adv
    // "_root_headset_trackingSystemName.description": "", // adv
    // "_root_headset_modelNumber.name": "", // adv
    // "_root_headset_modelNumber.description": "", // adv
    // "_root_headset_driverVersion.name": "", // adv
    // "_root_headset_driverVersion.description": "", // adv
    // "_root_headset_manufacturerName.name": "", // adv
    // "_root_headset_manufacturerName.description": "", // adv
    // "_root_headset_renderModelName.name": "", // adv
    // "_root_headset_renderModelName.description": "", // adv
    // "_root_headset_registeredDeviceType.name": "", // adv
    // "_root_headset_registeredDeviceType.description": "", // adv
    "_root_headset_trackingFrameOffset.name": "Задержка отслеживания",
    "_root_headset_trackingFrameOffset.description":
        "Задержка алгоритма предсказания расположения гарнитуры.",
    "_root_headset_positionOffset.name": "Смещение положения гарнитуры", // adv
    // "_root_headset_positionOffset.description": "", // adv
    "_root_headset_positionOffset_0.name": "x", // adv
    "_root_headset_positionOffset_1.name": "y", // adv
    "_root_headset_positionOffset_2.name": "z", // adv
    "_root_headset_force3dof.name": "Принудительное отслеживание 3Dof",
    "_root_headset_force3dof.description":
        "Принудительное отслеживание 3-х степеней свободы (как в Oculus Go).",
    "_root_headset_controllers.name": "Контроллеры",
    // "_root_headset_controllers.description": use "_root_headset_controllers_enabled.description"
    // "_root_headset_controllers_enabled.description": "",
    "_root_headset_controllers_content_controllerMode.name": "Режим эмуляции контроллеров",
    "_root_headset_controllers_content_controllerMode.description":
        "Эмулирует разные контроллеры для лучшей совместимости или использования отслеживания рук.",
    // "_root_headset_controllers_content_modeIdx.name": "", // adv
    // "_root_headset_controllers_content_modeIdx.description": "", // adv
    // "_root_headset_controllers_content_trackingSystemName.name": "", // adv
    // "_root_headset_controllers_content_trackingSystemName.description": "", // adv
    // "_root_headset_controllers_content_manufacturerName.name": "", // adv
    // "_root_headset_controllers_content_manufacturerName.description": "", // adv
    // "_root_headset_controllers_content_modelNumber.name": "", // adv
    // "_root_headset_controllers_content_modelNumber.description": "", // adv
    // "_root_headset_controllers_content_renderModelNameLeft.name": "", // adv
    // "_root_headset_controllers_content_renderModelNameLeft.description": "", // adv
    // "_root_headset_controllers_content_renderModelNameRight.name": "", // adv
    // "_root_headset_controllers_content_renderModelNameRight.description": "", // adv
    // "_root_headset_controllers_content_serialNumber.name": "", // adv
    // "_root_headset_controllers_content_serialNumber.description": "", // adv
    // "_root_headset_controllers_content_ctrlType.name": "", // adv
    // "_root_headset_controllers_content_ctrlType.description": "", // adv
    // "_root_headset_controllers_content_registeredDeviceType.name": "", // adv
    // "_root_headset_controllers_content_registeredDeviceType.description": "", // adv
    // "_root_headset_controllers_content_inputProfilePath.name": "", // adv
    // "_root_headset_controllers_content_inputProfilePath.description": "", // adv
    // "_root_headset_controllers_content_trackingSpeed.name": "",
    // "_root_headset_controllers_content_trackingSpeed.description": "",
    "_root_headset_controllers_content_poseTimeOffset.name": "Задержка отслеживания положения", // adv
    "_root_headset_controllers_content_poseTimeOffset.description":
        "Задержка алгоритма предсказания расположения контроллеров.", // adv
    "_root_headset_controllers_content_positionOffsetLeft.name": "Смещение положения", // adv
    "_root_headset_controllers_content_positionOffsetLeft.description":
        "Смещение положения в метрах для левого контроллера. \nДля правого контроллера, значение для оси x отражено.", // adv
    "_root_headset_controllers_content_positionOffsetLeft_0.name": "x", // adv
    "_root_headset_controllers_content_positionOffsetLeft_1.name": "y", // adv
    "_root_headset_controllers_content_positionOffsetLeft_2.name": "z", // adv
    "_root_headset_controllers_content_rotationOffsetLeft.name": "Смещение поворота", // adv
    "_root_headset_controllers_content_rotationOffsetLeft.description":
        "Смещение поворота в градусах для левого контроллера. \nДля правого контроллера, повороты по осям Y и Z отражены.", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_0.name": "x", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_1.name": "y", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_2.name": "z", // adv
    "_root_headset_controllers_content_hapticsIntensity.name": "Интенсивность тактильной отдачи",
    // "_root_headset_controllers_content_hapticsIntensity.description": "",
    // "_root_headset_trackingSpace-choice-.name": "",
    // "_root_headset_trackingSpace-choice-.description": "",
    // "_root_headset_trackingSpace_local-choice-.name": "",
    // "_root_headset_trackingSpace_stage-choice-.name": "",
    // Connection tab
    "_root_connection_tab.name": "Подключение",
    // "_root_connection_autoTrustClients.name": "", // adv
    // "_root_connection_webServerPort.name": "",
    "_root_connection_streamPort.name": "Порт сервера трансляции", // adv
    // "_root_connection_streamPort.description": "", // adv
    "_root_connection_aggressiveKeyframeResend.name": "Интенсивный повтор отправки пакетов",
    "_root_connection_aggressiveKeyframeResend.description":
        "Уменьшает минимальный интервал между ключевыми кадрами со 100мс до 5мс.\nИспользуется только когда наблюдается потеря пакетов. Улучшает работу в сетях с большой потерей пакетов.",
    // Extra tab
    "_root_extra_tab.name": "Дополнительно",
    "_root_extra_theme-choice-.name": "Оформление",
    "_root_extra_theme-choice-.description":
        "Переходи на темную сторону! \n...у нас тут есть печеньки.",
    "_root_extra_theme_systemDefault-choice-.name": "Системная",
    "_root_extra_theme_classic-choice-.name": "Классическая",
    "_root_extra_theme_darkly-choice-.name": "Темная",
    // "_root_extra_clientDarkMode.name": "",
    // "_root_extra_clientDarkMode.description": "",
    "_root_extra_revertConfirmDialog.name": "Подтверждать восстановление",
    "_root_extra_revertConfirmDialog.description":
        "Запрашивать подтверждение перед откатом параметра к стандартному значению.",
    "_root_extra_restartConfirmDialog.name": "Подтверждать перезапуск",
    "_root_extra_restartConfirmDialog.description":
        "Запрашивать подтверждение перед перезапуском SteamVR.",
    // "_root_extra_promptBeforeUpdate.name": "",
    // "_root_extra_updateChannel-choice-.name": "",
    // "_root_extra_updateChannel_noUpdates-choice-.name": "",
    // "_root_extra_updateChannel_stable-choice-.name": "",
    // "_root_extra_updateChannel_beta-choice-.name": "",
    // "_root_extra_updateChannel_nightly-choice-.name": "",
    // "_root_extra_logToDisk.name": "",
    // "_root_extra_notificationLevel-choice-.name": "", // adv
    // "_root_extra_notificationLevel-choice-.description": "", // adv
    // "_root_extra_notificationLevel_error-choice-.name": "", // adv
    // "_root_extra_notificationLevel_warning-choice-.name": "", // adv
    // "_root_extra_notificationLevel_info-choice-.name": "", // adv
    // "_root_extra_notificationLevel_debug-choice-.name": "", // adv
    // "_root_extra_excludeNotificationsWithoutId.name": "", // adv
    // "_root_extra_excludeNotificationsWithoutId.description": "", // adv
    // Others
    steamVRRestartSuccess: "SteamVR успешно перезапущен",
    audioDeviceError:
        "Не найдены устройства воспроизведения. Невозможно транслировать звук или микрофон",
});
