define({
    // Video tab
    "_root_video_tab.name": "Video",
    // "_root_video_adapterIndex.name": "", // adv
    // "_root_video_displayRefreshRate.name": "",
    // "_root_video_displayRefreshRate.description": "",
    // "_root_video_preferredFps.name": "", // adv
    "_root_video_resolutionDropdown.name": "Résolution video",
    "_root_video_resolutionDropdown.description":
        "100% correspond a la résolution native de 2880x1600 de l'Oculus Quest.\nAugmenter la résolution peut améliorer la qualité visuelle, mais ce n'est pas recommandé.\nUne résolution en dessous de 100% peut réduire la latence et améliorer la performance du reseau ",
    "_root_video_renderResolution-choice-.name": "Résolution video", // adv
    // "_root_video_renderResolution_scale-choice-.name": "", // adv
    // "_root_video_renderResolution_absolute-choice-.name": "", // adv
    // "_root_video_renderResolution_scale.name": "", // adv
    // "_root_video_recommendedTargetResolution-choice-.name": "", // adv
    // "_root_video_recommendedTargetResolution_scale-choice-.name": "", // adv
    // "_root_video_recommendedTargetResolution_absolute-choice-.name": "", // adv
    // "_root_video_recommendedTargetResolution_scale.name": "", // adv
    // "_root_video_secondsFromVsyncToPhotons.name": "", // adv
    // "_root_video_secondsFromVsyncToPhotons.description": "", // adv
    "_root_video_foveatedRendering.name": "Rendu fovéal",
    // "_root_video_foveatedRendering.description": use "_root_video_foveatedRendering_enabled.description"
    "_root_video_foveatedRendering_enabled.description":
        "Technique de rendu qui réduit la résolution de la vision périphérique. cela permet de réduire la résolution de la vidéo transmise par le réseau pour réduire la latence. en fonction des paramètres il y aura plus ou moins d'artéfacts sur le bord de l'image ",
    "_root_video_foveatedRendering_content_strength.name": "Intensité",
    "_root_video_foveatedRendering_content_strength.description":
        "Une intensité plus élevé donne moins de détails sur les bords de l'image et plus d'artéfacts",
    // "_root_video_foveatedRendering_content_shape.name": "", // adv
    // "_root_video_foveatedRendering_content_shape.description": "", // adv
    "_root_video_foveatedRendering_content_verticalOffset.name": "Décalage vertical",
    "_root_video_foveatedRendering_content_verticalOffset.description":
        "une plus haute valeaur décale verticalement la zone ou la qualité sera plus haute.",
    "_root_video_colorCorrection.name": "Correction de couleurs",
    // "_root_video_colorCorrection.description": use "_root_video_colorCorrection_enabled.description"
    "_root_video_colorCorrection_enabled.description":
        "Les changements sont appliqués dans cet ordre : Sharpening, Gamma, Luminosité, Contraste, Saturation. ",
    "_root_video_colorCorrection_content_brightness.name": "Luminosité",
    "_root_video_colorCorrection_content_brightness.description":
        "Luminosité: -1 donne une image noire et 1 donne une image blanche",
    "_root_video_colorCorrection_content_contrast.name": "Contraste",
    "_root_video_colorCorrection_content_contrast.description":
        "Contraste: -1 donne une image grise",
    "_root_video_colorCorrection_content_saturation.name": "Saturation",
    "_root_video_colorCorrection_content_saturation.description":
        "Saturation: -1 donne une image en noir et blanc",
    "_root_video_colorCorrection_content_gamma.name": "Gamma",
    "_root_video_colorCorrection_content_gamma.description": "Gamma",
    "_root_video_colorCorrection_content_sharpening.name": "Sharpening",
    "_root_video_colorCorrection_content_sharpening.description":
        "Sharpening: -1 est le plus flou et 5 est le plus net",
    "_root_video_codec-choice-.name": "Codec video",
    "_root_video_codec-choice-.description":
        "Si posible utilisez h265 pour une meilleure qualité visuel avec un bitrate plus bas",
    "_root_video_codec_H264-choice-.name": "h264",
    "_root_video_codec_H264-choice-.description": "Utiliser le codec h264",
    "_root_video_codec_HEVC-choice-.name": "HEVC (h265)",
    "_root_video_codec_HEVC-choice-.description": "Utiliser le codec HEVC (h265)",
    // "_root_video_clientRequestRealtimeDecoder.name": "", // adv
    "_root_video_encodeBitrateMbs.name": "Bitrate video",
    "_root_video_encodeBitrateMbs.description":
        "Bitrate du streaming video.30Mbps est recommandé. \nUn bitrate plus élevé donne une meilleure image mais rajoute de la latence ainsi que du traffic réseau.",
    // Audio tab
    "_root_audio_tab.name": "Audio",
    "_root_audio_gameAudio.name": "Transmettre l'audio au casque",
    // "_root_audio_gameAudio.description": use "_root_audio_gameAudio_enabled.description"
    "_root_audio_gameAudio_enabled.description": "Permet de streamer l'audio du jeu sur le casque",
    "_root_audio_gameAudio_content_deviceId-choice-.name": "Choisisez la source audio",
    "_root_audio_gameAudio_content_deviceId-choice-.description":
        "Source audio utilisé pour la capture",
    // "_root_audio_gameAudio_content_muteWhenStreaming.name": "",
    // "_root_audio_gameAudio_content_muteWhenStreaming.description": "",
    "_root_audio_microphone.name": "Transmettre le microphone",
    // "_root_audio_microphone.description": use "_root_audio_microphone_enabled.description"
    "_root_audio_microphone_enabled.description":
        "Permet d'utiliser le microphone du casque sur l'ordinateur",
    // "_root_audio_microphone_content_deviceId-choice-.name": "",
    // "_root_audio_microphone_content_deviceId-choice-.description": "",
    // Headset tab
    "_root_headset_tab.name": "Casque",
    "_root_headset_headsetEmulationMode.name": "Mode d'émulation du casque",
    "_root_headset_headsetEmulationMode.description":
        "Permet de choisir quel casque émuler pour plus de compatibilité",
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
    "_root_headset_trackingFrameOffset.name": "Décalage temporel du tracking",
    "_root_headset_trackingFrameOffset.description":
        "Décalage temporel du tracking pour l'algorithme de prédiction",
    // "_root_headset_positionOffset.name": "", // adv
    // "_root_headset_positionOffset.description": "", // adv
    // "_root_headset_positionOffset_0.name": "", // adv
    // "_root_headset_positionOffset_1.name": "", // adv
    // "_root_headset_positionOffset_2.name": "", // adv
    "_root_headset_force3dof.name": "Forcer le tracking 3DOF",
    "_root_headset_force3dof.description": "Force le tracking sur trois axes",
    "_root_headset_controllers.name": "Controleurs",
    // "_root_headset_controllers.description": use "_root_headset_controllers_enabled.description"
    "_root_headset_controllers_enabled.description": "Activer l'utilisation des controleurs",
    "_root_headset_controllers_content_controllerMode.name": "Mode d'émulation des controleurs",
    "_root_headset_controllers_content_controllerMode.description":
        "Permet de choisir quel controleurs émuler pour plus de compatibilité ou pour activer le suivi des mains",
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
    "_root_headset_controllers_content_poseTimeOffset.name": "Pose time offset", // adv
    "_root_headset_controllers_content_poseTimeOffset.description":
        "Décalage pour l'algorithme de prédiction de pose", // adv
    // "_root_headset_controllers_content_positionOffsetLeft.name": "", // adv
    // "_root_headset_controllers_content_positionOffsetLeft.description": "", // adv
    // "_root_headset_controllers_content_positionOffsetLeft_0.name": "", // adv
    // "_root_headset_controllers_content_positionOffsetLeft_1.name": "", // adv
    // "_root_headset_controllers_content_positionOffsetLeft_2.name": "", // adv
    // "_root_headset_controllers_content_rotationOffsetLeft.name": "", // adv
    // "_root_headset_controllers_content_rotationOffsetLeft.description": "", // adv
    // "_root_headset_controllers_content_rotationOffsetLeft_0.name": "", // adv
    // "_root_headset_controllers_content_rotationOffsetLeft_1.name": "", // adv
    // "_root_headset_controllers_content_rotationOffsetLeft_2.name": "", // adv
    "_root_headset_controllers_content_hapticsIntensity.name": "Intensité des vibrations",
    "_root_headset_controllers_content_hapticsIntensity.description":
        "Facteur d'intensité du retour haptique",
    // "_root_headset_trackingSpace-choice-.name": "",
    // "_root_headset_trackingSpace-choice-.description": "",
    // "_root_headset_trackingSpace_local-choice-.name": "",
    // "_root_headset_trackingSpace_stage-choice-.name": "",
    // Connection tab
    "_root_connection_tab.name": "Connexion",
    // "_root_connection_autoTrustClients.name": "", // adv
    // "_root_connection_webServerPort.name": "",
    // "_root_connection_streamPort.name": "", // adv
    // "_root_connection_streamPort.description": "", // adv
    "_root_connection_aggressiveKeyframeResend.name": "Renvoi de keyframes aggresif",
    "_root_connection_aggressiveKeyframeResend.description": `Réduit l'intervale minimum entre les keyframes de 100ms a 5ms, utilisé seulment quand des pertes de paquets sont détéctés. améliore l'experience sur des réseaux avec pertes de paquets`,
    // Extra tab
    "_root_extra_tab.name": "Extra",
    // "_root_extra_theme-choice-.name": "",
    // "_root_extra_theme-choice-.description": "",
    // "_root_extra_theme_systemDefault-choice-.name": "",
    // "_root_extra_theme_classic-choice-.name": "",
    // "_root_extra_theme_darkly-choice-.name": "",
    // "_root_extra_clientDarkMode.name": "",
    // "_root_extra_clientDarkMode.description": "",
    "_root_extra_revertConfirmDialog.name": "Confirmer remise a zéro",
    "_root_extra_revertConfirmDialog.description":
        "Demande une confirmations avant de remettre une valeur a zéro",
    "_root_extra_restartConfirmDialog.name": "Confirmer redémarrer SteamVR",
    "_root_extra_restartConfirmDialog.description":
        "Demande une confirmation avant de redémarrer SteamVR.",
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
    // "steamVRRestartSuccess": "",
    // "audioDeviceError": "",
});
