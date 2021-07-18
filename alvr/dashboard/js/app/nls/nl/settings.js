define({
    // Video tab
    "_root_video_tab.name": "Video",
    "_root_video_displayRefreshRate.name": "Verversingssnelheid",
    "_root_video_displayRefreshRate.description":
        "Verversingssnelheid is ingesteld voor SteamVR en de headset. Hogere waarden hebben een snellere pc nodig. 72 Hz is het limiet voor de Quest 1.",
    "_root_video_resolutionDropdown.name": "Video resolutie",
    "_root_video_resolutionDropdown.description":
        "100% resulteert in de oorspronkelijke resolutie van de Oculus Quest. \nDe resolutie instellen kan een betere visuele kwaliteit geven, maar het is niet aanbevolen. \nEen resolutie lager dan 100% kan latency verminderen en netwerkprestaties verhogen",
    customVideoScale: "Custom",
    "_root_video_foveatedRendering.name": "Foveated encoding",
    // "_root_video_foveatedRendering.description": use "_root_video_foveatedRendering_enabled.description"
    "_root_video_foveatedRendering_enabled.description":
        "Rendering techniek dat de resolutie van het beeld vermindert aan de periferie van het zicht om de rekenbelasting van de GPU te verminderen. Dit resulteert in een veel lagere videoresolutie die via het netwerk moet worden verzonden.",
    "_root_video_foveatedRendering_content_strength.name": "Sterkte",
    "_root_video_foveatedRendering_content_strength.description":
        "Een hogere waarde betekent minder detail aan de randen van het frame en meer artefacten",
    "_root_video_foveatedRendering_content_verticalOffset.name": "Verticale offset",
    "_root_video_foveatedRendering_content_verticalOffset.description":
        "Een hogere waarde betekent het framegebied van hoge kwaliteit wordt verder naar beneden bewogen",
    "_root_video_colorCorrection.name": "Kleur correctie",
    // "_root_video_colorCorrection.description": use "_root_video_colorCorrection_enabled.description"
    "_root_video_colorCorrection_enabled.description":
        "Kleur correctie wordt toegepast in de volgende volgorde: verscherping, gamma, helderheid, contrast en verzadiging.",
    "_root_video_colorCorrection_content_brightness.name": "Helderheid",
    "_root_video_colorCorrection_content_brightness.description":
        "Helderheid: -1 betekent volledig zwart en 1 betekent volledig wit.",
    "_root_video_colorCorrection_content_contrast.name": "Contrast",
    "_root_video_colorCorrection_content_contrast.description":
        "Contrast: -1 betekent volledig grijs.",
    "_root_video_colorCorrection_content_saturation.name": "verzadiging",
    "_root_video_colorCorrection_content_saturation.description":
        "verzadiging: -1 betekent dat het beeld wit en zwart is.",
    "_root_video_colorCorrection_content_gamma.name": "Gamma",
    "_root_video_colorCorrection_content_gamma.description":
        "Gamut: Gebruik een waarde van 2,2 om de kleur van sRGB naar RGB te corrigeren. Dit regelt de helderheid maar houdt zwart in zwart en wit in wit",
    "_root_video_colorCorrection_content_sharpening.name": "Verscherping",
    "_root_video_colorCorrection_content_sharpening.description":
        "Scherpte: benadrukt de randen van het beeld.",
    "_root_video_codec-choice-.name": "Video codering",
    "_root_video_codec-choice-.description":
        "HEVC is de voorkeur om een ​​betere visuele kwaliteit te bereiken bij lagere bitrates. AMD videokaarten werkt het best met HEVC.",
    "_root_video_codec_H264-choice-.name": "h264",
    "_root_video_codec_HEVC-choice-.name": "HEVC (h265)",
    "_root_video_encodeBitrateMbs.name": "Video Bitrate",
    "_root_video_encodeBitrateMbs.description":
        "Bitrate van video streaming. 30Mbps is aangeraden. \nHogere bitrates zorgen voor een beter beeld maar verhoogd de latency en netwerk verkeer",
    // Audio tab
    "_root_audio_tab.name": "Audio",
    "_root_audio_gameAudio.name": "Stream game audio",
    // "_root_audio_gameAudio.description": use "_root_audio_gameAudio_enabled.description"
    "_root_audio_gameAudio_enabled.description": "Stream de game audio naar de headset",
    "_root_audio_gameAudio_content_deviceId-choice-.name": "Audio apparaat",
    "_root_audio_gameAudio_content_deviceId-choice-.description":
        "Selecteer het audio apparaat gebruikt om audio vast te leggen",
    "_root_audio_gameAudio_content_muteWhenStreaming.name": "Demp output tijdens het streamen",
    "_root_audio_gameAudio_content_muteWhenStreaming.description":
        "Dempt het audio apparaat (speakers/headphones) wanneer je streamt naar de headset. Alleen de fysieke output is gedempt (om dubbele audio te voorkomen), streamen naar de headset en andere opnamesoftware worden niet beïnvloed.",
    "_root_audio_microphone.name": "Stream de microfoon van de headset",
    // "_root_audio_microphone.description": use "_root_audio_microphone_enabled.description"
    "_root_audio_microphone_enabled.description":
        "Streamt de microfoon van de headset naar SteamVR. \nOm de microfoon te laten werken moet je VB-Audio Virtual device of een gelijksoortige software instaleren",
    "_root_audio_microphone_content_deviceId-choice-.name": "Virtuele microfoon input",
    "_root_audio_microphone_content_deviceId-choice-.description":
        "Selecteer de virtuele microfoon input gebruikt om audio vast te leggen. \nOm de microfoon te laten werken moet je VB-Audio Virtual device of een gelijksoortige software instaleren",
    // Headset tab
    "_root_headset_tab.name": "Headset",
    "_root_headset_headsetEmulationMode.name": "Headset emulatie modus",
    "_root_headset_headsetEmulationMode.description":
        "Emuleert verschillende headsets voor een betere compatibiliteit",
    "_root_headset_trackingFrameOffset.name": "Tracking frame-compensatie",
    "_root_headset_trackingFrameOffset.description":
        "Compensatie voor het pose-voorspellingsalgoritme",
    "_root_headset_force3dof.name": "Forceer 3Dof",
    "_root_headset_force3dof.description":
        "Forceert the 3 degrees of freedom modus (zoals Oculus Go)",
    "_root_headset_controllers.name": "Controllers",
    // "_root_headset_controllers.description": use "_root_headset_controllers_enabled.description"
    "_root_headset_controllers_enabled.description": "Laat het gebruik van controllers toe",
    "_root_headset_controllers_content_controllerMode.name": "Controller emulatie modus",
    "_root_headset_controllers_content_controllerMode.description":
        "Emuleert verschillende controllers voor betere compatibiliteit of maakt handvolging mogelijk",
    "_root_headset_controllers_content_trackingSpeed.name": "Tracking snelheid",
    "_root_headset_controllers_content_trackingSpeed.description":
        "Voor spellen met snelle bewegingen zoals Beatsaber, kies voor medium of snel. For tragere spellen zoals Skyrim kies normaal.\nOculus voorspellingen betekent dat de positie van de controllers voospelt worden via de headset en niet op de PC via SteamVR.",
    "_root_headset_controllers_content_hapticsIntensity.name": "Haptische intensiteit",
    "_root_headset_controllers_content_hapticsIntensity.description":
        "Factor die de intensiteit van de vibratie van de controllers vermedert of vermindert.",
    "_root_headset_controllers_content_useHeadsetTrackingSystem.name":
        "Use Headset Tracking System",
    "_root_headset_controllers_content_useHeadsetTrackingSystem.description":
        "Overrides the current controller profile's tracking system name with the current ALVR HMD's tracking system. Enable this in cases such as space calibration with OpenVR space calibrator.",
    "_root_headset_trackingSpace-choice-.name": "Tracking ruimte",
    "_root_headset_trackingSpace-choice-.description":
        "Stelt in wat de headset gebruikt als referentie voor tracking en hoe het midden van de ruimte wordt gedefinieerd. Stationair-trackingruimte gedraagt ​​zich als een bedrade headset: het midden van de ruimte blijft op één plek nadat de headset opnieuw is geplaatst. Dit moet worden ingesteld als u Vive-trackers wilt gebruiken.",
    "_root_headset_trackingSpace_local-choice-.name": "Lokaal (Headset gecentreerd)",
    "_root_headset_trackingSpace_stage-choice-.name": "Stationair (Kamer gecentreerd)",
    // Connection tab
    "_root_connection_tab.name": "Connectie",
    "_root_connection_webServerPort.name": "Web server poort",
    "_root_connection_aggressiveKeyframeResend.name": "Aggressief keyframe opnieuw verzenden",
    "_root_connection_aggressiveKeyframeResend.description":
        "Verlaagt het minimuminterval tussen keyframes van 100 ms naar 5 ms. \nAlleen gebruikt wanneer er pakketten verloren worden. \nVerbetert de ervaring op netwerken met pakketverlies.",
    // Extra tab
    "_root_extra_tab.name": "Extra",
    "_root_extra_theme-choice-.name": "Thema",
    "_root_extra_theme-choice-.description": "Sluit aan bij de donker kant.\n We hebben koekjes.",
    "_root_extra_theme_systemDefault-choice-.name": "Systeem",
    "_root_extra_theme_classic-choice-.name": "Klassiek",
    "_root_extra_theme_darkly-choice-.name": "Donker",
    "_root_extra_clientDarkMode.name": "Client donkere modus",
    "_root_extra_clientDarkMode.description":
        "Toegepast nadat de headset opnieuw verbonden en uit slaap modus gehaald wordt",
    "_root_extra_revertConfirmDialog.name": "Bevestig terugzetten instellingen",
    "_root_extra_revertConfirmDialog.description":
        "Vraag bevestiging voordat instellingen naar hun standaardwaarden teruggezet worden.",
    "_root_extra_restartConfirmDialog.name": "Bevestig het herstarten van SteamVR",
    "_root_extra_promptBeforeUpdate.name": "Bevestig voor updates",
    "_root_extra_updateChannel-choice-.name": "Update kanaal",
    "_root_extra_updateChannel_noUpdates-choice-.name": "Geen updates",
    "_root_extra_updateChannel_stable-choice-.name": "Stabiel",
    "_root_extra_updateChannel_beta-choice-.name": "Beta",
    "_root_extra_updateChannel_nightly-choice-.name": "Nightly",
    "_root_extra_logToDisk.name": "Log op schijf (session_log.txt)",
    // Others
    steamVRRestartSuccess: "SteamVR succesvol herstart",
    audioDeviceError:
        "Geen audio apparaten gevonden. Men kan de audio of de microfoon niet streamen",
});
