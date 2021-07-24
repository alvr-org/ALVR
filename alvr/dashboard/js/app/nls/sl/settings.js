define({
    "_root_video_tab.name": "SLIKA",
    "_root_video_adapterIndex.name": "GPE kazalo", // adv
    "_root_video_displayRefreshRate.name": "Hitrost osveževanja",
    "_root_video_displayRefreshRate.description":
        "Hitrost osveževanja za SteamVR in napravo. SteamVR bo uporabil to hitrost tudi če je naprava ne podpira. Višje vsote zahtevajo zmogljivejši PC. 72 Hz je najhitrejše kar Quest 1 podpira.",
    "_root_video_preferredFps.name": "Osveževanje po meri", // adv
    "_root_video_resolutionDropdown.name": "Ločljivost slike",
    "_root_video_resolutionDropdown.description":
        "100% je mehanska ločljivost Oculus Quest-a. \nZvišanje ločljivosti lahko izbolša sliko, a ni priporočeno. \nLočljivost nižja od 100% lahko skrajša zaostanek.",
    "_root_video_renderResolution-choice-.name": "Osnovna ločljivost stisnjene slike", // adv
    "_root_video_renderResolution_scale-choice-.name": "Lestvica", // adv
    "_root_video_renderResolution_absolute-choice-.name": "Brezpogojno", // adv
    "_root_video_renderResolution_scale.name": "Lestvica", // adv
    "_root_video_recommendedTargetResolution-choice-.name": "Zaželjena ločljivost igre", // adv
    "_root_video_recommendedTargetResolution_scale-choice-.name": "Lestvica", // adv
    "_root_video_recommendedTargetResolution_absolute-choice-.name": "Brezpogojno", // adv
    "_root_video_recommendedTargetResolution_scale.name": "Lestvica", // adv
    "_root_video_secondsFromVsyncToPhotons.name": "Sekunde od VSync-a do slike", // adv
    "_root_video_secondsFromVsyncToPhotons.description":
        "Čas, ki preteče od navideznega VSync-a do vidne slike na zaslonu.", // adv
    "_root_video_foveatedRendering.name": "Sredinska prednost(Foveated Rendering)",
    // "_root_video_foveatedRendering.description": use "_root_video_foveatedRendering_enabled.description"
    "_root_video_foveatedRendering_enabled.description":
        "Upodobitven način, ki zamnjša ločljivost slike v obrobju, kar zamnjša obremenitev na GPE in omogoča bolj jasno sliko pri nižjem toku podatkov.",
    "_root_video_foveatedRendering_content_strength.name": "Moč",
    "_root_video_foveatedRendering_content_strength.description":
        "Višja vsota pomeni manjša ločljivost in več napak v obrobju.",
    "_root_video_foveatedRendering_content_shape.name": "Oblika", // adv
    "_root_video_foveatedRendering_content_shape.description": "Oblika ostrega predela slike.", // adv
    "_root_video_foveatedRendering_content_verticalOffset.name": "Pokončni odmik",
    "_root_video_foveatedRendering_content_verticalOffset.description":
        "Višja vsota premakne oster del slike nižje na samem zaslonu.",
    "_root_video_colorCorrection.name": "Prilagoditev slike",
    // "_root_video_colorCorrection.description": use "_root_video_colorCorrection_enabled.description"
    "_root_video_colorCorrection_enabled.description":
        "Prilagoditev je uveljavljena v sledečem vrstnem redu: Ostrina, Gamma, Svetlost, Kontrast in Barvitost.",
    "_root_video_colorCorrection_content_brightness.name": "Svetlost",
    "_root_video_colorCorrection_content_brightness.description":
        "Svetlost: Na -1 je slika popolnoma črna in na 1 čisto bela.",
    "_root_video_colorCorrection_content_contrast.name": "Kontrast",
    "_root_video_colorCorrection_content_contrast.description":
        "Kontrast: Na -1 bo slika popolnoma siva.",
    "_root_video_colorCorrection_content_saturation.name": "Barvitost",
    "_root_video_colorCorrection_content_saturation.description":
        "Barvitost: Na -1 je slika črnobela, na več kot 0 so lahko barve prenasičene.",
    "_root_video_colorCorrection_content_gamma.name": "Gamma",
    "_root_video_colorCorrection_content_gamma.description":
        "Gamma: To spreminja svetlost, ampak obdrži belo belo in črno črno.",
    "_root_video_colorCorrection_content_sharpening.name": "Ostrina",
    "_root_video_colorCorrection_content_sharpening.description":
        "Ostrina: Poudari kontrastne robove.",
    "_root_video_codec-choice-.name": "Način zlaganja/zmanjšanja",
    "_root_video_codec-choice-.description": "H.265 omogoča boljšo sliko z manjšim tokom podatkov.",
    "_root_video_codec_H264-choice-.name": "H.264",
    "_root_video_codec_HEVC-choice-.name": "H.265",
    "_root_video_clientRequestRealtimeDecoder.name":
        "Zahtevaj najhitrejšo različico razlagovalca (naprava)", // adv
    "_root_video_encodeBitrateMbs.name": "Podatkovni tok videa",
    "_root_video_encodeBitrateMbs.description":
        "Podatkovni tok video prenosa. Predlagana vsota je 10% zmožnosti brezžične povezave.(Naprava>Nastavitve>Omrežja>Trenutno omrežje>Podrobnosti) \nVišji tok izboljša sliko a podlaša zaostanek in obratno.",
    // Audio tab
    "_root_audio_tab.name": "ZVOK",
    "_root_audio_gameAudio.name": "Prenašaj zvok na napravo",
    // "_root_audio_gameAudio.description": use "_root_audio_gameAudio_enabled.description"
    "_root_audio_gameAudio_enabled.description": "Prenašaj zvok izbrane zvočne naprave.",
    "_root_audio_gameAudio_content_deviceId-choice-.name": "Zvočna naprava",
    "_root_audio_gameAudio_content_deviceId-choice-.description":
        "Izberi od katere naprave bo zvok zajet.",
    "_root_audio_gameAudio_content_muteWhenStreaming.name":
        "Izključi zvok strežnika med prenašanjem",
    "_root_audio_gameAudio_content_muteWhenStreaming.description":
        "Izključi zvok od izbrane zvočne naprave (zvočniki/slušalke) med prenašanjem na napravo. To nima vpliva na snemanje z ostalimi programi.",
    "_root_audio_microphone.name": "Prenesi mikrofon iz naprave",
    // "_root_audio_microphone.description": use "_root_audio_microphone_enabled.description"
    "_root_audio_microphone_enabled.description":
        "Prenese zvok iz mikrofona od naprave v SteamVR. \nDa bo ta nastavitev delovala je predlagano namestiti VB-Audio Virtual device ali podobno, če imaš proste vhode in izhode za zvok, lahko pa tudi uporabiš AUX žico, in povežeš izbran izhod na željen vhod.",
    "_root_audio_microphone_content_deviceId-choice-.name": "Izberi napravo za mikrofon",
    "_root_audio_microphone_content_deviceId-choice-.description":
        "Izberi zvočno napravo preko katere bo zvok mikrofona predvajan.",
    // Headset tab
    "_root_headset_tab.name": "NAPRAVA",
    "_root_headset_headsetEmulationMode.name": "Izberi kater HMD naj bo posneman",
    "_root_headset_headsetEmulationMode.description":
        "Posnema različne naprave za boljšo združljivost.",
    "_root_headset_universeId.name": "ID Sveta", // adv
    "_root_headset_serialNumber.name": "Zaporedna številka", // adv
    "_root_headset_serialNumber.description": "Zaporedna številka posnemane naprave.", // adv
    "_root_headset_trackingSystemName.name": "Tracking system name", // adv
    "_root_headset_trackingSystemName.description": "Name of the emulated headset tracking system.", // adv
    "_root_headset_modelNumber.name": "Model number", // adv
    "_root_headset_modelNumber.description": "Model number of the emulated headset", // adv
    "_root_headset_driverVersion.name": "Driver version", // adv
    "_root_headset_driverVersion.description": "Driver version of the emulated headset", // adv
    "_root_headset_manufacturerName.name": "Manufacturer name", // adv
    "_root_headset_manufacturerName.description": "Manufacturer name of the emulated headset", // adv
    "_root_headset_renderModelName.name": "Render model name", // adv
    "_root_headset_renderModelName.description": "Render model name of the emulated headset", // adv
    "_root_headset_registeredDeviceType.name": "Registered device type", // adv
    "_root_headset_registeredDeviceType.description":
        "Registered device type of the emulated headset", // adv
    "_root_headset_trackingFrameOffset.name": "Št. sličic zamika za sledenje ",
    "_root_headset_trackingFrameOffset.description": "Zamik za predvidevanje sledenja.",
    "_root_headset_positionOffset.name": "Headset position offset", // adv
    "_root_headset_positionOffset.description":
        "Headset position offset used by the position prediction algorithm.", // adv
    "_root_headset_positionOffset_0.name": "X", // adv
    "_root_headset_positionOffset_1.name": "Y", // adv
    "_root_headset_positionOffset_2.name": "Z", // adv
    "_root_headset_force3dof.name": "Vsili 3 DoF",
    "_root_headset_force3dof.description":
        "Vsili 3 stopnje svobode (Tako kot Oculus Go/Ostale VR naprave brez prostorskega sledenja).",
    "_root_headset_controllers.name": "Krmilniki",
    // "_root_headset_controllers.description": use "_root_headset_controllers_enabled.description"
    "_root_headset_controllers_enabled.description": "Dovoli uporabo krmilnikov.",
    "_root_headset_controllers_content_controllerMode.name": "Način posnemanja krmilnikov",
    "_root_headset_controllers_content_controllerMode.description":
        "Posnema različne krmilnike za bolšjo združljivost in sledenje prstom.",
    "_root_headset_controllers_content_modeIdx.name": "Mode Index", // adv
    "_root_headset_controllers_content_modeIdx.description":
        "Mode index of the emulated controller", // adv
    "_root_headset_controllers_content_trackingSystemName.name": "Tracking system name", // adv
    "_root_headset_controllers_content_trackingSystemName.description":
        "Name of the emulated controller tracking system", // adv
    "_root_headset_controllers_content_manufacturerName.name": "Manufacturer name", // adv
    "_root_headset_controllers_content_manufacturerName.description":
        "Manufacturer name of the emulated controller", // adv
    "_root_headset_controllers_content_modelNumber.name": "Model number", // adv
    "_root_headset_controllers_content_modelNumber.description":
        "Model number of the emulated controller", // adv
    "_root_headset_controllers_content_renderModelNameLeft.name": "Model number (Left hand)", // adv
    "_root_headset_controllers_content_renderModelNameLeft.description":
        "Model number of the emulated left hand controller", // adv
    "_root_headset_controllers_content_renderModelNameRight.name": "Model number (Right hand)", // adv
    "_root_headset_controllers_content_renderModelNameRight.description":
        "Model number of the emulated right hand controller", // adv
    "_root_headset_controllers_content_serialNumber.name": "Serial number", // adv
    "_root_headset_controllers_content_serialNumber.description":
        "Serial number of the emulated controller", // adv
    "_root_headset_controllers_content_registeredDeviceType.name": "Registered device type", // adv
    "_root_headset_controllers_content_registeredDeviceType.description":
        "Registered device type of the emulated controller", // adv
    "_root_headset_controllers_content_inputProfilePath.name": "Input profile path", // adv
    "_root_headset_controllers_content_inputProfilePath.description":
        "Input profile path of the emulated controller", // adv
    "_root_headset_controllers_content_trackingSpeed.name": "Hitrost sledenja",
    "_root_headset_controllers_content_trackingSpeed.description":
        "Za hitre igre kot Beatsaber, izberi SREDNJE ali HITRO. Za počasnejše igre kjer potrebuješ večjo natančnost uporabi NAVADNO.\nOCULUS predvidevanje pomeni, da je predvidevanje izvedeno na napravi, namesto na računalniku preko SteamVR.",
    "_root_headset_controllers_content_poseTimeOffset.name": "Pose time offset", // adv
    "_root_headset_controllers_content_poseTimeOffset.description":
        "Offset for the pose prediction algorithm", // adv
    "_root_headset_controllers_content_positionOffsetLeft.name": "Position offset", // adv
    "_root_headset_controllers_content_positionOffsetLeft.description":
        "Position offset in meters for the left controller. \n For the right controller, x value is mirrored", // adv
    "_root_headset_controllers_content_positionOffsetLeft_0.name": "X", // adv
    "_root_headset_controllers_content_positionOffsetLeft_1.name": "Y", // adv
    "_root_headset_controllers_content_positionOffsetLeft_2.name": "Z", // adv
    "_root_headset_controllers_content_rotationOffsetLeft.name": "Rotation offset", // adv
    "_root_headset_controllers_content_rotationOffsetLeft.description":
        "Rotation offset in degrees for the left controller.\nFor the right controller, rotations along the Y and Z axes are mirrored", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_0.name": "X", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_1.name": "Y", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_2.name": "Z", // adv
    "_root_headset_controllers_content_hapticsIntensity.name": "Sila povratnih tresljajov",
    "_root_headset_controllers_content_hapticsIntensity.description":
        "Povečaj ali zmanjšaj tresljaje v krmlinikih.",
    "_root_headset_trackingSpace-choice-.name": "Način prostora",
    "_root_headset_trackingSpace-choice-.description":
        "Nastavi kaj naprava uporabi za osnovo sledenja in kako je sredina določena. Podestno sledenje se obnaša kot naprave z žico: sredina prostora ostane na mestu, po ponastavitvi sredinske točke naprave, za ponastavitev uporabiš SteamVR. Z Okoliškim sledenjem lahko ponastaviš sredino prostora na napravi-tako kot v samostojnem načinu.",
    "_root_headset_trackingSpace_local-choice-.name": "Okoliško (Osredotočeno na napravo)",
    "_root_headset_trackingSpace_stage-choice-.name": "Podest (Osredotočeno na sobo/prostor)",
    // Connection tab
    "_root_connection_tab.name": "POVEZAVA",
    "_root_connection_autoTrustClients.name": "Samodejno zaupaj napravam (ni priporočeno)", // adv
    "_root_connection_webServerPort.name": "Pristanišče spletnega strežnika",
    "_root_connection_streamPort.name": "Pretočno pristanišče strežnika", // adv
    "_root_connection_streamPort.description":
        "Pristanišče, ki go bo strežnik uporabljal za prejemanje zabojčkov.", // adv
    "_root_connection_aggressiveKeyframeResend.name": "Vsiljeno pošiljanje celotnih sličic",
    "_root_connection_aggressiveKeyframeResend.description":
        "Zamnjšaj najmanjši čas med celotnimi slikicami iz 100 ms na 5 ms. \nUporabljeno samo ko je zaznana izguba zabojčkov. \nIzboljša izkušnjo na omrežjih, ki izgubljajo zabojčke, sicer pa poveča zamik in je podatkovno bolj potratno.",
    // Extra tab
    "_root_extra_tab.name": "DODATNO",
    "_root_extra_theme-choice-.name": "Izgled",
    "_root_extra_theme-choice-.description": "Pridi na temno stran.\n Mi imamo piškotke.",
    "_root_extra_theme_systemDefault-choice-.name": "SAMODEJEN",
    "_root_extra_theme_classic-choice-.name": "NAVADEN",
    "_root_extra_theme_darkly-choice-.name": "TEMEN",
    "_root_extra_clientDarkMode.name": "Temen izgled na napravi",
    "_root_extra_revertConfirmDialog.name": "Potrdi povrnitev",
    "_root_extra_revertConfirmDialog.description":
        "Vprašaj za potrditev, preden se nastavitev povrne.",
    "_root_extra_restartConfirmDialog.name": "Potrdi ponoven zagon SteamVR",
    "_root_extra_promptBeforeUpdate.name": "Opozori pred posodobitvijo",
    "_root_extra_updateChannel-choice-.name": "Posodobitvene različice",
    "_root_extra_updateChannel_noUpdates-choice-.name": "Brez posodobitev",
    "_root_extra_updateChannel_stable-choice-.name": "Trdna",
    "_root_extra_updateChannel_beta-choice-.name": "Preizkuzna",
    "_root_extra_updateChannel_nightly-choice-.name": "Dnevna-preizkusna",
    "_root_extra_logToDisk.name": "Beleži na pomnilnik (session_log.txt)",
    "_root_extra_notificationLevel-choice-.name": "Raven opozoril", // adv
    "_root_extra_notificationLevel-choice-.description":
        "Na kateri stopnji se bodo opozorila pojavila. Od manj do več podrobnosti: \n - Napaka \n - Opozorilo \n - podrobnosti \n - Razhrošč", // adv
    "_root_extra_notificationLevel_error-choice-.name": "Napaka", // adv
    "_root_extra_notificationLevel_warning-choice-.name": "Opozorilo", // adv
    "_root_extra_notificationLevel_info-choice-.name": "Podrobnost", // adv
    "_root_extra_notificationLevel_debug-choice-.name": "Razhroč", // adv
    "_root_extra_excludeNotificationsWithoutId.name": "Spreglej ne prepoznavna opozorila", // adv
    "_root_extra_excludeNotificationsWithoutId.description":
        "Do not show notifications that do not contain the identification structure.", // adv
    // Others
    steamVRRestartSuccess: "SteamVR se je vspešno zagnal",
    audioDeviceError: "Ni zaznanih zvočnih naprav, ni mogoče prenašanje zvoka in mikrofona",
});
