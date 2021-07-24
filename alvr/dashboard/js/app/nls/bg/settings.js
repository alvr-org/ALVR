define({
    // Video tab
    "_root_video_tab.name": "Видео",
    "_root_video_adapterIndex.name": "Индекс на GPU", // adv
    // "_root_video_displayRefreshRate.name": "",
    // "_root_video_displayRefreshRate.description": "",
    // "_root_video_preferredFps.name": "", // adv
    "_root_video_resolutionDropdown.name": "Разрешение на видео",
    "_root_video_resolutionDropdown.description":
        "100% води до естествената резолюция на Oculus Quest.\nЗадаването на разделителната способност може да донесе известно подобрение в качеството на зрението, но не се препоръчва.\nРазделителна способност, по-ниска от 100%, може да намали латентността и да увеличи производителността на мрежата",
    "_root_video_renderResolution-choice-.name": "Основа за резолюция на видео кодиране", // adv
    // "_root_video_renderResolution_scale-choice-.name": "", // adv
    // "_root_video_renderResolution_absolute-choice-.name": "", // adv
    // "_root_video_renderResolution_scale.name": "", // adv
    "_root_video_recommendedTargetResolution-choice-.name":
        "Предпочитана резолюция за изобразяване на играта", // adv
    // "_root_video_recommendedTargetResolution_scale-choice-.name": "", // adv
    // "_root_video_recommendedTargetResolution_absolute-choice-.name": "", // adv
    // "_root_video_recommendedTargetResolution_scale.name": "", // adv
    // "_root_video_secondsFromVsyncToPhotons.name": "", // adv
    // "_root_video_secondsFromVsyncToPhotons.description": "", // adv
    "_root_video_foveatedRendering.name": "Фовеатично изобразяване",
    // "_root_video_foveatedRendering.description": use "_root_video_foveatedRendering_enabled.description"
    "_root_video_foveatedRendering_enabled.description":
        "Техника, при която центърът на изображението се изобразява с висока разделителна способност, докато покрайнините се изобразяват с по-ниска разделителна способност.\nРезултатът е много по-ниска видеоразделителна способност, която трябва да се предава по мрежата.\nПо-малкият видеоклип при една и съща битрейт може да запази повече подробности и да намали латентността едновременно.\nFFR причинява някои визуални артефакти по краищата на изгледа, които са повече или по-малко видими в зависимост от настройките и играта",
    "_root_video_foveatedRendering_content_strength.name": "Сила",
    "_root_video_foveatedRendering_content_strength.description":
        "По-високата стойност означава по-малко детайли към ръбовете на рамката и повече артефакти",
    // "_root_video_foveatedRendering_content_shape.name": "", // adv
    "_root_video_foveatedRendering_content_shape.description": "Формата на фовеатен рендеринг", // adv
    "_root_video_foveatedRendering_content_verticalOffset.name": "Вертикално отместване",
    "_root_video_foveatedRendering_content_verticalOffset.description":
        "По-високата стойност означава, че висококачествената област на рамката се премества по-надолу.",
    "_root_video_colorCorrection.name": "Корекция на цветовете",
    // "_root_video_colorCorrection.description": use "_root_video_colorCorrection_enabled.description"
    // "_root_video_colorCorrection_enabled.description": "",
    "_root_video_colorCorrection_content_brightness.name": "Яркост",
    // "_root_video_colorCorrection_content_brightness.description": "",
    "_root_video_colorCorrection_content_contrast.name": "Контраст",
    // "_root_video_colorCorrection_content_contrast.description": "",
    "_root_video_colorCorrection_content_saturation.name": "Наситеност",
    // "_root_video_colorCorrection_content_saturation.description": "",
    "_root_video_colorCorrection_content_gamma.name": "Гама",
    "_root_video_colorCorrection_content_gamma.description":
        "Това контролира яркостта, но запазва черното в черно и бялото в бялото",
    "_root_video_colorCorrection_content_sharpening.name": "Заточване",
    // "_root_video_colorCorrection_content_sharpening.description": "",
    "_root_video_codec-choice-.name": "Видео кодек",
    "_root_video_codec-choice-.description":
        "HEVC се предпочита за постигане на по-добро визуално качество при по-ниски битрейтове. Видеокартите AMD работят най-добре с HEVC.",
    "_root_video_codec_H264-choice-.name": "h264",
    "_root_video_codec_HEVC-choice-.name": "HEVC (h265)",
    // "_root_video_clientRequestRealtimeDecoder.name": "", // adv
    "_root_video_encodeBitrateMbs.name": "Битрейт на видео",
    "_root_video_encodeBitrateMbs.description":
        "Битрейт на видео излъчване. Препоръчва се 30Mb/сек. \nПо-високата скорост на предаване осигурява по-добро качество на изображението, но увеличава латентността и мрежовия трафик.",
    // Audio tab
    "_root_audio_tab.name": "Аудио",
    "_root_audio_gameAudio.name": "Аудио предаване на игра",
    // "_root_audio_gameAudio.description": use "_root_audio_gameAudio_enabled.description"
    // "_root_audio_gameAudio_enabled.description": "",
    "_root_audio_gameAudio_content_deviceId-choice-.name": "Изберете аудио устройство",
    "_root_audio_gameAudio_content_deviceId-choice-.description":
        "Аудио устройство използвано за улавяне на звук",
    // "_root_audio_gameAudio_content_muteWhenStreaming.name": "",
    // "_root_audio_gameAudio_content_muteWhenStreaming.description": "",
    "_root_audio_microphone.name": "Излъчен микрофон",
    // "_root_audio_microphone.description": use "_root_audio_microphone_enabled.description"
    "_root_audio_microphone_enabled.description": "Предава сигнала на микрофона на слушалките.",
    "_root_audio_microphone_content_deviceId-choice-.name":
        "Изберете виртуално устройство за въвеждане",
    "_root_audio_microphone_content_deviceId-choice-.description":
        "За да работи микрофонът ви, трябва да инсталирате VB-CABLE Virtual Audio Device или подобен или подобен или подобен или подобен.",
    // Headset tab
    "_root_headset_tab.name": "Слушалки",
    "_root_headset_headsetEmulationMode.name": "Режим на емулация на слушалки",
    "_root_headset_headsetEmulationMode.description":
        "Емулира различни слушалки за по-добра съвместимост.",
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
    "_root_headset_trackingFrameOffset.name": "Изместване на рамката за проследяване",
    "_root_headset_trackingFrameOffset.description": "Офсет за алгоритъма за предсказване на поза",
    "_root_headset_positionOffset.name": "Офсетна позиция на слушалките", // adv
    // "_root_headset_positionOffset.description": "", // adv
    "_root_headset_positionOffset_0.name": "x", // adv
    "_root_headset_positionOffset_1.name": "y", // adv
    "_root_headset_positionOffset_2.name": "z", // adv
    "_root_headset_force3dof.name": "Принуден 3Dof",
    "_root_headset_force3dof.description":
        "Принуждава режима на 3 степени на свобода (като Oculus Go)",
    "_root_headset_controllers.name": "Контролери",
    // "_root_headset_controllers.description": use "_root_headset_controllers_enabled.description"
    // "_root_headset_controllers_enabled.description": "",
    "_root_headset_controllers_content_controllerMode.name": "Режим на емулация на контролер",
    "_root_headset_controllers_content_controllerMode.description":
        "Емулира различни контролери за по-добра съвместимост или ръчно проследяване.",
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
    "_root_headset_controllers_content_poseTimeOffset.name":
        "Забавяне на проследяване на позицията", // adv
    "_root_headset_controllers_content_poseTimeOffset.description":
        "Офсет за алгоритъма за предсказване на позицията.", // adv
    "_root_headset_controllers_content_positionOffsetLeft.name": "Изместване на позицията", // adv
    "_root_headset_controllers_content_positionOffsetLeft.description":
        "Изместване на позицията в метри за левия контролер. \nЗа десния контролер стойността за оста x се отразява.", // adv
    "_root_headset_controllers_content_positionOffsetLeft_0.name": "x", // adv
    "_root_headset_controllers_content_positionOffsetLeft_1.name": "y", // adv
    "_root_headset_controllers_content_positionOffsetLeft_2.name": "z", // adv
    "_root_headset_controllers_content_rotationOffsetLeft.name": "Изместване на въртенето", // adv
    "_root_headset_controllers_content_rotationOffsetLeft.description":
        "Изместване на въртенето в градуси за левия контролер. \nЗа десния контролер се отразяват завъртанията Y и Z.", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_0.name": "x", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_1.name": "y", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_2.name": "z", // adv
    "_root_headset_controllers_content_hapticsIntensity.name":
        "Интензивност на тактилна обратна връзка",
    // "_root_headset_controllers_content_hapticsIntensity.description": "",
    // "_root_headset_trackingSpace-choice-.name": "",
    // "_root_headset_trackingSpace-choice-.description": "",
    // "_root_headset_trackingSpace_local-choice-.name": "",
    // "_root_headset_trackingSpace_stage-choice-.name": "",
    // Connection tab
    "_root_connection_tab.name": "Връзка",
    // "_root_connection_autoTrustClients.name": "", // adv
    // "_root_connection_webServerPort.name": "",
    "_root_connection_streamPort.name": "Порт за излъчен сървър", // adv
    // "_root_connection_streamPort.description": "", // adv
    "_root_connection_aggressiveKeyframeResend.name": "Интензивен опит за повторен пакет",
    "_root_connection_aggressiveKeyframeResend.description":
        "Намалява минималния интервал между ключовите кадри от 100ms на 5ms.\nИзползва се само когато се наблюдава загуба на пакети. Подобрява производителността в мрежи с висока загуба на пакети.",
    // Extra tab
    "_root_extra_tab.name": "допълнително",
    "_root_extra_theme-choice-.name": "Тема",
    "_root_extra_theme-choice-.description": "Елате в Тъмната страна. \nИмаме бисквитки.",
    "_root_extra_theme_systemDefault-choice-.name": "Система",
    "_root_extra_theme_classic-choice-.name": "Класически",
    "_root_extra_theme_darkly-choice-.name": "Мрачно",
    // "_root_extra_clientDarkMode.name": "",
    // "_root_extra_clientDarkMode.description": "",
    "_root_extra_revertConfirmDialog.name": "Потвърдете възстановяването",
    "_root_extra_revertConfirmDialog.description":
        "Поискайте потвърждение, преди да върнете параметъра до стойността му по подразбиране.",
    "_root_extra_restartConfirmDialog.name": "Потвърдете рестартирането",
    "_root_extra_restartConfirmDialog.description":
        "Поискайте потвърждение, преди да рестартирате SteamVR.",
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
    steamVRRestartSuccess: "SteamVR се рестартира успешно",
    audioDeviceError:
        "Не са намерени устройства за възпроизвеждане. Не може да се излъчи аудио или микрофон",
});
