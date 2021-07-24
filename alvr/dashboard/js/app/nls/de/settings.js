define({
    // Video tab
    "_root_video_tab.name": "Video",
    "_root_video_adapterIndex.name": "GPU index", // adv
    "_root_video_displayRefreshRate.name": "Bildwiederholrate",
    "_root_video_displayRefreshRate.description":
        "Bildwiederholrate für das Headset und SteamVR. Höhere Werte benötigen einen besseren PC. 72 Hz ist das Maximum für die Quest 1.",
    "_root_video_preferredFps.name": "Benutzerdefinierte Bildwiederholrate", // adv
    "_root_video_resolutionDropdown.name": "Videoauflösung",
    "_root_video_resolutionDropdown.description":
        "100% ergibt die native Auflösung der Oculus Quest. \nDie Auflösung zu erhöhen steigert die Bildqualität, ist aber nicht empfohlen. \nEine niedrigere Auflösung kann Latenz und Netzwerkleistung verbessern",
    "_root_video_renderResolution-choice-.name": "Videoauflösungsbasis", // adv
    // I am not very knowledgeable about this stuff here. A lot of this is guesswork. If you know more than me, please fix my mistakes!
    "_root_video_renderResolution_scale-choice-.name": "Relativ", // adv
    "_root_video_renderResolution_absolute-choice-.name": "Absolut", // adv
    "_root_video_renderResolution_scale.name": "Relativ", // adv
    "_root_video_recommendedTargetResolution-choice-.name": "Vorgezogene Spiel-Auflösung", // adv
    "_root_video_recommendedTargetResolution_scale-choice-.name": "Relativ", // adv
    "_root_video_recommendedTargetResolution_absolute-choice-.name": "Absolut", // adv
    "_root_video_recommendedTargetResolution_scale.name": "Relativ", // adv
    "_root_video_secondsFromVsyncToPhotons.name": "Sekunden zwischen VSync und Anzeige", // adv
    "_root_video_secondsFromVsyncToPhotons.description":
        "Die Zeit zwischen dem virtuellen VSync und dem tatsächlichen Anzeigen des Bildes", // adv
    // from here my translations should be ok again
    "_root_video_foveatedRendering.name": "'Foveated rendering'",
    // "_root_video_foveatedRendering.description": use "_root_video_foveatedRendering_enabled.description"
    "_root_video_foveatedRendering_enabled.description":
        "Eine Render-taktik die die Auflösung an den Rändern des Sichtfelds senkt, um die GPU zu entlasten. Führt zu weniger Durgangsrate über das Netzwerk",
    "_root_video_foveatedRendering_content_strength.name": "Stärke",
    "_root_video_foveatedRendering_content_strength.description":
        "Ein höherer Wert führt zu weniger Detail am Rand des Bildes, und mehr Artefakten",
    "_root_video_foveatedRendering_content_shape.name": "Form", // adv
    "_root_video_foveatedRendering_content_shape.description": "Die Form des 'foveated rendering'", // adv
    "_root_video_foveatedRendering_content_verticalOffset.name": "Vertikale Verschiebung",
    "_root_video_foveatedRendering_content_verticalOffset.description":
        "Ein höherer Wert verschiebt die 'scharfe' Bildregion weiter nach unten",
    "_root_video_colorCorrection.name": "Farbkorrektur",
    // "_root_video_colorCorrection.description": use "_root_video_colorCorrection_enabled.description"
    "_root_video_colorCorrection.description":
        "Farbkorrektur läuft in der Folgenden Reihenfolge ab: Schärfen, Gamma, Helligkeit, Kontrast, und Färbung.",
    "_root_video_colorCorrection_content_brightness.name": "Helligkeit",
    "_root_video_colorCorrection_content_brightness.description":
        "Helligkeit: -1 bedeutet komplett Schwarz und 1 bedeutet komplett Weiß.",
    "_root_video_colorCorrection_content_contrast.name": "Kontrast",
    "_root_video_colorCorrection_content_contrast.description":
        "Kontrast: -1 bedeutet komplett Grau.",
    "_root_video_colorCorrection_content_saturation.name": "Färbung",
    "_root_video_colorCorrection_content_saturation.description":
        "Färbung: -1 bedeutet ein Schwarz-Weiß Bild.",
    "_root_video_colorCorrection_content_gamma.name": "Gamma",
    // Don't know how to translate "Gamut" | original English text below
    // "_root_video_colorCorrection_content_gamma.description": "Gamut: Use a value of 2.2 to correct the color from sRGB to RGB. This controls the brightness but keeps blacks to black and whites to white",
    "_root_video_colorCorrection_content_gamma.description":
        "Kontrolliert die Helligkeit, belässt aber Schwarz bei Schwarz und Weiß bei Weiß",
    "_root_video_colorCorrection_content_sharpening.name": "Schärfen",
    "_root_video_colorCorrection_content_sharpening.description":
        "Schärfe: Verstärkt die Kanten des Bildes",
    "_root_video_codec-choice-.name": "Video codec",
    "_root_video_codec-choice-.description":
        "HEVC wird aufgrund besserer Videoqualität bei niedrigen Datenraten bevorzugt. AMD GPUs funktionieren am besten mit HEVC.",
    "_root_video_codec_H264-choice-.name": "h264",
    "_root_video_codec_HEVC-choice-.name": "HEVC (h265)",
    "_root_video_clientRequestRealtimeDecoder.name": "Echtzeit-decoder-Berechtigungen anfordern", // adv
    "_root_video_encodeBitrateMbs.name": "Video Bitrate",
    "_root_video_encodeBitrateMbs.description":
        "Streaming-Bitrate. 30Mbps ist empfohlen. \nHöhere Bitraten ergeben ein besseres Bild aber auch mehr Latenz und Netzwerkauslastung",
    // Audio tab
    "_root_audio_tab.name": "Audio",
    "_root_audio_gameAudio.name": "PC-Audio streamen",
    // "_root_audio_gameAudio.description": use "_root_audio_gameAudio_enabled.description"
    // "_root_audio_gameAudio_enabled.description": "",
    "_root_audio_gameAudio_content_deviceId-choice-.name": "Audiogerät",
    "_root_audio_gameAudio_content_deviceId-choice-.description":
        "Wähle das Audiogerät aus, dessen Ton zum Headset gespiegelt werden soll",
    "_root_audio_gameAudio_content_muteWhenStreaming.name":
        "Audiogerät während des Streamens Stummschalten",
    "_root_audio_gameAudio_content_muteWhenStreaming.description":
        "Deaktiviert den Tonausgang deines Audiogeräts, wenn zum Headset gestreamt wird. Nur der physikalsche Ausgang (Lautsprecher/Kopfhörer) wird stummgeschalten, der Stream zum Headset oder anderen Aufnahme-Programmen is unangetastet.",
    "_root_audio_microphone.name": "Headset-Mikrofon streamen",
    // "_root_audio_microphone.description": use "_root_audio_microphone_enabled.description"
    "_root_audio_microphone_enabled.description":
        "Stream den Ton deines Headset-Mikrofons zum PC \nUm dies zu benutzen benötigst du 'VB-Audio Virtual device' oder ein ähnliches Programm",
    "_root_audio_microphone_content_deviceId-choice-.name": "Virtueller Mikrofoneingang",
    "_root_audio_microphone_content_deviceId-choice-.description":
        "Wähle den Eingang des Virtuellen Mikrofons aus \nUm dies zu benutzen benötigst du 'VB-Audio Virtual device' oder ein ähnliches Programm",
    // Headset tab
    "_root_headset_tab.name": "Headset",
    "_root_headset_headsetEmulationMode.name": "Zu Simulierendes Headset",
    "_root_headset_headsetEmulationMode.description":
        "Simuliert verschiedene Headsets, um Kompatibilität zu erhöhen",
    // What is a Universe ID?
    "_root_headset_universeId.name": "Universe ID", // adv
    "_root_headset_serialNumber.name": "Seriennummer", // adv
    "_root_headset_serialNumber.description": "Seriennummer des simulierten Headsets", // adv
    "_root_headset_trackingSystemName.name": "Name des Tracking-Systems", // adv
    "_root_headset_trackingSystemName.description": "Name des simulierten Tracking-Systems", // adv
    "_root_headset_modelNumber.name": "Modelnummer", // adv
    "_root_headset_modelNumber.description": "Modelnummer des simulierten Headsets", // adv
    "_root_headset_driverVersion.name": "Treiberversion", // adv
    "_root_headset_driverVersion.description": "Treiberversion des simulierten Headsets", // adv
    "_root_headset_manufacturerName.name": "Herstellername", // adv
    "_root_headset_manufacturerName.description": "Herstellername des simulierten Headsets", // adv
    "_root_headset_renderModelName.name": "Rendermodell-Name", // adv
    "_root_headset_renderModelName.description": "Rendermodell-Name des simulierten Headsets", // adv
    "_root_headset_registeredDeviceType.name": "Registrierter Geräte-Typ", // adv
    "_root_headset_registeredDeviceType.description":
        "Registrierter Geräte-Typ des simulierten Headsets", // adv
    "_root_headset_trackingFrameOffset.name": "Tracking Offset",
    "_root_headset_trackingFrameOffset.description":
        "Tracking Offset für den Positions-Vorhersage-Algorythmus",
    "_root_headset_positionOffset.name": "Headsetpositionsoffset", // adv
    "_root_headset_positionOffset.description":
        "Headsetpositionsoffset für den Positions-Vorhersage-Algorythmus", // adv
    "_root_headset_positionOffset_0.name": "X", // adv
    "_root_headset_positionOffset_1.name": "Y", // adv
    "_root_headset_positionOffset_2.name": "Z", // adv
    "_root_headset_force3dof.name": "Erzwinge 3Dof",
    "_root_headset_force3dof.description":
        "Erzwingt den '3 degrees of freedom' Modus (wie die Oculus Go)",
    "_root_headset_controllers.name": "Controller",
    // "_root_headset_controllers.description": use "_root_headset_controllers_enabled.description"
    "_root_headset_controllers_enabled.description": "Erlaube die Nutzung von Controllern",
    "_root_headset_controllers_content_controllerMode.name": "Controller Simulationsmodus",
    "_root_headset_controllers_content_controllerMode.description":
        "Simuliert verschiedene Controller oder aktiviert Handtracking",
    "_root_headset_controllers_content_modeIdx.name": "Indexmodus", // adv
    "_root_headset_controllers_content_modeIdx.description":
        "Indexmodus des simulierten Controllers", // adv
    "_root_headset_controllers_content_trackingSystemName.name": "Name des Tracking-Systems", // adv
    "_root_headset_controllers_content_trackingSystemName.description":
        "Name des simulierten Controllers", // adv
    "_root_headset_controllers_content_manufacturerName.name": "Herstellername", // adv
    "_root_headset_controllers_content_manufacturerName.description":
        "Herstellername des simulierten Controllers", // adv
    "_root_headset_controllers_content_modelNumber.name": "Modellnummer", // adv
    "_root_headset_controllers_content_modelNumber.description":
        "Modellnummer des simulierten Controllers", // adv
    "_root_headset_controllers_content_renderModelNameLeft.name": "Modellnummer (linke Hand)", // adv
    "_root_headset_controllers_content_renderModelNameLeft.description":
        "Modelnummer des linken simulierten Controllers", // adv
    "_root_headset_controllers_content_renderModelNameRight.name": "Modellnummer (rechte Hand)", // adv
    "_root_headset_controllers_content_renderModelNameRight.description":
        "Modellnummer des rechten simulierten Controllers", // adv
    "_root_headset_controllers_content_serialNumber.name": "Seriennummer", // adv
    "_root_headset_controllers_content_serialNumber.description":
        "Seriennummer des simulierten Controllers", // adv
    "_root_headset_controllers_content_registeredDeviceType.name": "Registrierter Geräte-Typ", // adv
    "_root_headset_controllers_content_registeredDeviceType.description":
        "Registrierter Geräte-Typ des simulierten Controllers", // adv
    "_root_headset_controllers_content_inputProfilePath.name": "Eingangsprofilpfad", // adv
    "_root_headset_controllers_content_inputProfilePath.description":
        "Eingangsprofilpfad jedes simulierten Controllers", // adv
    "_root_headset_controllers_content_trackingSpeed.name": "Trackinggeschwindigkeit",
    "_root_headset_controllers_content_trackingSpeed.description":
        "Wähle Medium oder Schnell für schnelle Spiele wie BeatSaber. Lass es auf normal für langsamere Spiele wie Skyrim.\nOculus Vorhersage-Algorithmus bedeutet dass die Vorhersage auf dem Headset anstatt des PCs geschiet.",
    // "_root_headset_controllers_content_poseTimeOffset.name": "", // adv
    // "_root_headset_controllers_content_poseTimeOffset.description": "", // adv
    "_root_headset_controllers_content_positionOffsetLeft.name": "Position offset", // adv
    "_root_headset_controllers_content_positionOffsetLeft.description":
        "Positions Offset für den linken Controller in Metern \n X ist gespiegelt für den rechten Controller", // adv
    "_root_headset_controllers_content_positionOffsetLeft_0.name": "X", // adv
    "_root_headset_controllers_content_positionOffsetLeft_1.name": "Y", // adv
    "_root_headset_controllers_content_positionOffsetLeft_2.name": "Z", // adv
    "_root_headset_controllers_content_rotationOffsetLeft.name": "Rotation offset", // adv
    "_root_headset_controllers_content_rotationOffsetLeft.description":
        "Rotations Offset für den linken Controller.\nY und Z sind gespiegelt für den rechten Controller", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_0.name": "X", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_1.name": "Y", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_2.name": "Z", // adv
    "_root_headset_controllers_content_hapticsIntensity.name": "Vibrationsstärke",
    "_root_headset_controllers_content_hapticsIntensity.description":
        "Faktor zum verstärken/schwächen des Haptischen Feedbacks",
    "_root_headset_trackingSpace-choice-.name": "Trackingzone",
    "_root_headset_trackingSpace-choice-.description":
        "Definiert die Zone, die das Headset verwendet um zu tracken und die Mitte des Raumes zu definieren. Bühnen-Tracking verhält sich wie ein verkabeltes Headset, und muss verwendet werden wenn Vive-Tracker verwendet werden",
    "_root_headset_trackingSpace_local-choice-.name": "Lokal (Zentriert am Headset)",
    "_root_headset_trackingSpace_stage-choice-.name": "Bühne (Zentriert am Raum)",
    // Connection tab
    "_root_connection_tab.name": "Verbindung",
    "_root_connection_autoTrustClients.name":
        "Neuen Clients automatisch Vertrauen (nicht empfohlen)", // adv
    "_root_connection_webServerPort.name": "Webserver Port",
    "_root_connection_streamPort.name": "Server streaming port", // adv
    "_root_connection_streamPort.description": "Port, der vom Server zum Streamen verwendet wird", // adv
    "_root_connection_aggressiveKeyframeResend.name": "Aggressiver 'keyframe resend'", //FIXME: "keyframe resend"
    "_root_connection_aggressiveKeyframeResend.description":
        "Senkt das minimale Intervall zwischen Keyframes von 100ms zu 5ms. \nWird nur verwendet, falls Packetverlust erkannt wird. \nVerbessert die Leistung in Netzwerken mit Packetverlust.",
    // Extra tab
    "_root_extra_tab.name": "Extras",
    "_root_extra_theme-choice-.name": "Themen",
    "_root_extra_theme-choice-.description": "Komm auf die dunkle Seite.\n Wir haben Kekse!",
    "_root_extra_theme_systemDefault-choice-.name": "System",
    "_root_extra_theme_classic-choice-.name": "Hell",
    "_root_extra_theme_darkly-choice-.name": "Dunkel",
    "_root_extra_clientDarkMode.name": "Client Nachtmodus",
    "_root_extra_clientDarkMode.description": "Wird nach der Verbindung angewendet",
    "_root_extra_revertConfirmDialog.name": "Reset bestätigen",
    "_root_extra_revertConfirmDialog.description":
        "Nach einer Bestätigung fragen, bevor alle Werte auf den Standard zurückgesetzt werden",
    "_root_extra_restartConfirmDialog.name": "SteamVR-Neustart bestätigen",
    "_root_extra_promptBeforeUpdate.name": "Vor einem Update benachrichtigen",
    "_root_extra_updateChannel-choice-.name": "Updatekanal",
    "_root_extra_updateChannel_noUpdates-choice-.name": "Keine Updates",
    "_root_extra_updateChannel_stable-choice-.name": "Stabil",
    "_root_extra_updateChannel_beta-choice-.name": "Beta",
    "_root_extra_updateChannel_nightly-choice-.name": "Nightly",
    "_root_extra_logToDisk.name": "lokalen Log erstellen (session_log.txt)",
    "_root_extra_notificationLevel-choice-.name": "Benachrichtigungslevel", // adv
    "_root_extra_notificationLevel-choice-.description":
        "Ab welchem Level sollen Benachrichtigungen generiert werden? Von viel zu wenig Benachrichtigungen: \n - Fehler \n - Warnung \n - Information \n - Debug", // adv
    "_root_extra_notificationLevel_error-choice-.name": "Fehler", // adv
    "_root_extra_notificationLevel_warning-choice-.name": "Warnung", // adv
    "_root_extra_notificationLevel_info-choice-.name": "Information", // adv
    "_root_extra_notificationLevel_debug-choice-.name": "Debug", // adv
    "_root_extra_excludeNotificationsWithoutId.name":
        "Benachrichtigungen ohne Identifikationsstruktur auslassen", // adv
    "_root_extra_excludeNotificationsWithoutId.description":
        "Benachrichtigungen ignorieren, die nicht in die Identifikationsstruktur passen", // adv
    // Others
    steamVRRestartSuccess: "SteamVR erfolgreich Neugestartet",
    audioDeviceError: "Keine Audiogeräte gefunden!",
});
