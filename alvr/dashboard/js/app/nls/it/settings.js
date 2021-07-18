define({
    // Video tab
    "_root_video_tab.name": "Video",
    "_root_video_adapterIndex.name": "Indice GPU", // adv
    "_root_video_displayRefreshRate.name": "FPS",
    "_root_video_displayRefreshRate.description":
        "Frequenza di refresh del visore. 72 Hz è il massimo per L'Oculus Quest 1.",
    // "_root_video_preferredFps.name": "", // adv
    "_root_video_resolutionDropdown.name": "Risoluzione video",
    "_root_video_resolutionDropdown.description":
        "100% corrisponde alla risoluzione nativa dell'Oculus Quest.\nImpostare la risoluzione può migliorare marginalmente la qualità dell'immagine ma non è consigliato.\nUna risoluzione minore di 100% può ridurre la latenza e migliorare la qualità di trasmissione",
    "_root_video_renderResolution-choice-.name": "Risoluzione codifica video", // adv
    // "_root_video_renderResolution_scale-choice-.name": "", // adv
    // "_root_video_renderResolution_absolute-choice-.name": "", // adv
    // "_root_video_renderResolution_scale.name": "", // adv
    "_root_video_recommendedTargetResolution-choice-.name": "Risoluzione di rendering preferita", // adv
    // "_root_video_recommendedTargetResolution_scale-choice-.name": "", // adv
    // "_root_video_recommendedTargetResolution_absolute-choice-.name": "", // adv
    // "_root_video_recommendedTargetResolution_scale.name": "", // adv
    // "_root_video_secondsFromVsyncToPhotons.name": "", // adv
    // "_root_video_secondsFromVsyncToPhotons.description": "", // adv
    "_root_video_foveatedRendering.name": "Foveated encoding",
    // "_root_video_foveatedRendering.description": use "_root_video_foveatedRendering_enabled.description"
    "_root_video_foveatedRendering_enabled.description": `Tecnica di rendering che riduce la risoluzione dell'immagine nella periferia della visione per ridurre il carico computazionale della GPU, la quantità di dati da trasmettere, e la latenza. Questa impostazione può provocare una distorsione dell'immagine ai bordi.`,
    "_root_video_foveatedRendering_content_strength.name": "Intensità",
    "_root_video_foveatedRendering_content_strength.description":
        "A valori più alti corrisponde meno dettaglio ai bordi dell'immagine e più artefatti",
    "_root_video_foveatedRendering_content_shape.name": "Rapporto di forma", // adv
    "_root_video_foveatedRendering_content_shape.description":
        "Forma del rettangolo centrale a risoluzione originale", // adv
    "_root_video_foveatedRendering_content_verticalOffset.name": "Spostamento verticale",
    "_root_video_foveatedRendering_content_verticalOffset.description":
        "Spostamento verticale del rettangolo centrale a risoluzione originale. Valori positivi corrispondono ad uno spostamento verso il basso.",
    "_root_video_colorCorrection.name": "Correzione del colore",
    // "_root_video_colorCorrection.description": use "_root_video_colorCorrection_enabled.description"
    "_root_video_colorCorrection.description": "Correzione del colore",
    "_root_video_colorCorrection_content_brightness.name": "Luminosità",
    // "_root_video_colorCorrection_content_brightness.description": "",
    "_root_video_colorCorrection_content_contrast.name": "Contrasto",
    // "_root_video_colorCorrection_content_contrast.description": "",
    "_root_video_colorCorrection_content_saturation.name": "Saturazione",
    // "_root_video_colorCorrection_content_saturation.description": "",
    "_root_video_colorCorrection_content_gamma.name": "Gamma",
    "_root_video_colorCorrection_content_gamma.description":
        "Controlla la luminosità ma tenendo i livelli del nero a nero e bianco a bianco",
    "_root_video_colorCorrection_content_sharpening.name": "Sharpening",
    "_root_video_colorCorrection_content_sharpening.description":
        "Sharpening: mette in risalto i bordi nell'immagine",
    "_root_video_codec-choice-.name": "Codec video",
    "_root_video_codec-choice-.description":
        "Usa h265 se possibile per una migliore qualità video a bitrate più bassi",
    "_root_video_codec_H264-choice-.name": "h264",
    "_root_video_codec_HEVC-choice-.name": "HEVC (h265)",
    // "_root_video_clientRequestRealtimeDecoder.name": "", // adv
    "_root_video_encodeBitrateMbs.name": "Bitrate per il video",
    "_root_video_encodeBitrateMbs.description":
        "Bitrate della trasmissione video. È consigliato 30Mbps. \nUn bitrate più alto comporta una qualità migliore dell'immagine ma al costo di una maggiore latenza e traffico di rete.",
    // Audio tab
    "_root_audio_tab.name": "Audio",
    "_root_audio_gameAudio.name": "Trasmetti l'audio del gioco",
    // "_root_audio_gameAudio.description": use "_root_audio_gameAudio_enabled.description"
    // "_root_audio_gameAudio_enabled.description": "",
    "_root_audio_gameAudio_content_deviceId-choice-.name": "Seleziona un dispositivo audio",
    "_root_audio_gameAudio_content_deviceId-choice-.description":
        "Dispositivo usato per catturare l'audio del gioco",
    "_root_audio_gameAudio_content_muteWhenStreaming.name": "Silenzia PC durante la trasmissione",
    "_root_audio_gameAudio_content_muteWhenStreaming.description":
        "Azzera il volume del dispositivo audio dal PC durante la trasmissione al visore. L'audio viene comunque transmesso al visore. Questo fa si di evitare l'eco dal PC",
    "_root_audio_microphone.name": "Trasmetti microfono",
    // "_root_audio_microphone.description": use "_root_audio_microphone_enabled.description"
    "_root_audio_microphone_enabled.description":
        "Trasmetti l'audio del microfono dal visore al PC",
    "_root_audio_microphone_content_deviceId-choice-.name": "Seleziona un microfono virtuale",
    "_root_audio_microphone_content_deviceId-choice-.description":
        "Per far funzionare il microfono, devi installare VB-Audio Virtual device o un altro software equivalente",
    // Headset tab
    "_root_headset_tab.name": "Visore",
    "_root_headset_headsetEmulationMode.name": "Modalità emulazione del visore",
    "_root_headset_headsetEmulationMode.description":
        "Scegli la modalità di emulazione del visore per migliorare la compatibilità con alcuni giochi",
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
    "_root_headset_trackingFrameOffset.name": "Offset temporale del tracking",
    "_root_headset_trackingFrameOffset.description":
        "Offset temporale del tracking del visore usato dall'algoritmo di predizione della posa",
    "_root_headset_positionOffset.name": "Offset spaziale del visore", // adv
    // "_root_headset_positionOffset.description": "", // adv
    "_root_headset_positionOffset_0.name": "x", // adv
    "_root_headset_positionOffset_1.name": "y", // adv
    "_root_headset_positionOffset_2.name": "z", // adv
    "_root_headset_force3dof.name": "Modalità 3DOF",
    "_root_headset_force3dof.description":
        "Forza solo 3 gradi di libertà per il visore (solo rotazione)",
    "_root_headset_controllers.name": "Controller",
    // "_root_headset_controllers.description": use "_root_headset_controllers_enabled.description"
    // "_root_headset_controllers_enabled.description": "",
    "_root_headset_controllers_content_controllerMode.name": "Modalità emulazione controller",
    "_root_headset_controllers_content_controllerMode.description":
        "Scegli la modalità di emulazione dei controller per migliorare la compatibilità con alcuni giochi, e scegli se attivare l'emulazione del grilletto con il tracking delle mani",
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
    "_root_headset_controllers_content_trackingSpeed.name": "Velocità di tracking",
    // "_root_headset_controllers_content_trackingSpeed.description": "",
    // "_root_headset_controllers_content_poseTimeOffset.name": "", // adv
    // "_root_headset_controllers_content_poseTimeOffset.description": "", // adv
    "_root_headset_controllers_content_positionOffsetLeft.name": "Offset spaziale", // adv
    "_root_headset_controllers_content_positionOffsetLeft.description":
        "Offset della posizione (in metri) del controller sinistro. \nPer il controller destro, viene usato l'opposto del valore x", // adv
    "_root_headset_controllers_content_positionOffsetLeft_0.name": "x", // adv
    "_root_headset_controllers_content_positionOffsetLeft_1.name": "y", // adv
    "_root_headset_controllers_content_positionOffsetLeft_2.name": "z", // adv
    "_root_headset_controllers_content_rotationOffsetLeft.name": "Offset di rotazione", // adv
    "_root_headset_controllers_content_rotationOffsetLeft.description":
        "Offset di rotazione in gradi per il controller sinistro. \nPer il controller destro, le rotazioni lungo l'asse Y e Z sono invertite", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_0.name": "x", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_1.name": "y", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_2.name": "z", // adv
    "_root_headset_controllers_content_hapticsIntensity.name": "Intensità feedback tattile",
    // "_root_headset_controllers_content_hapticsIntensity.description": "",
    "_root_headset_trackingSpace-choice-.name": "Ancora dello spazio di gioco",
    "_root_headset_trackingSpace-choice-.description":
        "Imposta come il visore viene tracciato all'avvio. La modalità room scale va impostata se si vuole giocare a giochi che richiedono camminare nello spazio di gioco o se si vuole usare tracker esterni come i Vive trackers.",
    "_root_headset_trackingSpace_local-choice-.name": "Mobile",
    "_root_headset_trackingSpace_stage-choice-.name": "Room scale",
    // Connection tab
    "_root_connection_tab.name": "Connessione",
    // "_root_connection_autoTrustClients.name": "", // adv
    // "_root_connection_webServerPort.name": "",
    // "_root_connection_streamPort.name": "", // adv
    // "_root_connection_streamPort.description": "", // adv
    // "_root_connection_aggressiveKeyframeResend.name": "",
    // "_root_connection_aggressiveKeyframeResend.description": "",
    // Extra tab
    "_root_extra_tab.name": "Extra",
    "_root_extra_theme-choice-.name": "Tema",
    // "_root_extra_theme-choice-.description": "",
    "_root_extra_theme_systemDefault-choice-.name": "Predefinito di sistema",
    "_root_extra_theme_classic-choice-.name": "Classico",
    "_root_extra_theme_darkly-choice-.name": "Darkly",
    "_root_extra_clientDarkMode.name": "Tema scuro per il client",
    "_root_extra_clientDarkMode.description":
        "Applicato dopo la connessione, sospensione e riaccensione del visore",
    "_root_extra_revertConfirmDialog.name": "Conferma reimpostazione valori",
    "_root_extra_revertConfirmDialog.description":
        "Chiedi conferma prima di reipostare i valori delle impostazioni al valore predefinito",
    "_root_extra_restartConfirmDialog.name": "Conferma riavvio SteamVR",
    "_root_extra_promptBeforeUpdate.name": "Chiedi prima di aggiornare",
    "_root_extra_updateChannel-choice-.name": "Canale di aggiornamento",
    "_root_extra_updateChannel_noUpdates-choice-.name": "Nessun aggiornamento",
    "_root_extra_updateChannel_stable-choice-.name": "Stabile",
    "_root_extra_updateChannel_beta-choice-.name": "Beta",
    "_root_extra_updateChannel_nightly-choice-.name": "Nightly",
    "_root_extra_logToDisk.name": "Salva log su disco (session_log.txt)",
    // "_root_extra_notificationLevel-choice-.name": "", // adv
    // "_root_extra_notificationLevel-choice-.description": "", // adv
    // "_root_extra_notificationLevel_error-choice-.name": "", // adv
    // "_root_extra_notificationLevel_warning-choice-.name": "", // adv
    // "_root_extra_notificationLevel_info-choice-.name": "", // adv
    // "_root_extra_notificationLevel_debug-choice-.name": "", // adv
    // "_root_extra_excludeNotificationsWithoutId.name": "", // adv
    // "_root_extra_excludeNotificationsWithoutId.description": "", // adv
    // Others
    steamVRRestartSuccess: "Riavvio in corso...",
    audioDeviceError:
        "Nessun dispositivo audio trovato. Non sarà possibile trasmettere l'audio del gioco o il microfono",
});
