define({
    // Video tab
    "_root_video_tab.name": "映像",
    "_root_video_tab.description": "映像設定",
    // "_root_video_adapterIndex.name": "", // adv
    // "_root_video_displayRefreshRate.name": "",
    // "_root_video_displayRefreshRate.description": "",
    // "_root_video_preferredFps.name": "", // adv
    "_root_video_resolutionDropdown.name": "解像度",
    "_root_video_resolutionDropdown.description":
        "100%に設定するとOculus Questのネイティブ解像度である2880x1600になる.\n解像度を設定すると, 見た目の品質が多少向上する可能性があるが非推奨.\n100%より低く設定するとレイテンシが低下し, ネットワークパフォーマンスが向上する可能性がある",
    "_root_video_renderResolution-choice-.name": "解像度", // adv
    // "_root_video_renderResolution_scale-choice-.name": "", // adv
    // "_root_video_renderResolution_absolute-choice-.name": "", // adv
    // "_root_video_renderResolution_scale.name": "", // adv
    // "_root_video_recommendedTargetResolution-choice-.name": "", // adv
    // "_root_video_recommendedTargetResolution_scale-choice-.name": "", // adv
    // "_root_video_recommendedTargetResolution_absolute-choice-.name": "", // adv
    // "_root_video_recommendedTargetResolution_scale.name": "", // adv
    // "_root_video_secondsFromVsyncToPhotons.name": "", // adv
    "// _root_video_secondsFromVsyncToPhotons.description": "", // adv
    "_root_video_foveatedRendering.name": "フォービエイテッドレンダリング",
    // "_root_video_foveatedRendering.description": use "_root_video_foveatedRendering_enabled.description"
    "_root_video_foveatedRendering_enabled.description": `画像の中心部を高解像度で描画し, 周辺部を低解像度で描画する技術. これにより, ネットワーク経由で送信するビデオ解像度が大幅に減少する. 同ビットレートにおいて, 解像度が低い方がよりディテールを保持できると同時にレイテンシを低下させることができる. しかし, 設定やゲームによっては表示領域の端で多かれ少なかれ視覚的な異常を引き起こす`,
    "_root_video_foveatedRendering_content_strength.name": "強度",
    "_root_video_foveatedRendering_content_strength.description":
        "値が高いほど, より画像端付近の解像度が低くなり, 歪みが多くなる",
    // "_root_video_foveatedRendering_content_shape.name": "", // adv
    // "_root_video_foveatedRendering_content_shape.description": "", // adv
    "_root_video_foveatedRendering_content_verticalOffset.name": "垂直オフセット",
    "_root_video_foveatedRendering_content_verticalOffset.description":
        "値が高いほど, 高解像度の領域が下へ移動する",
    "_root_video_colorCorrection.name": "色補正",
    // "_root_video_colorCorrection.description": use "_root_video_colorCorrection_enabled.description"
    "_root_video_colorCorrection_enabled.description":
        "色補正はシャープネス, ガンマ, 明度, コントラスト, 彩度の順に適用",
    "_root_video_colorCorrection_content_brightness.name": "明度",
    "_root_video_colorCorrection_content_brightness.description":
        "明るさ. -1設定時は完全に黒, 1設定時は完全に白となる",
    "_root_video_colorCorrection_content_contrast.name": "コントラスト",
    "_root_video_colorCorrection_content_contrast.description":
        "コントラスト. -1設定時はグレースケールとなる",
    "_root_video_colorCorrection_content_saturation.name": "彩度",
    "_root_video_colorCorrection_content_saturation.description":
        "鮮やかさ. -1設定時は白と黒となる",
    "_root_video_colorCorrection_content_gamma.name": "ガンマ",
    "_root_video_colorCorrection_content_gamma.description":
        "ガンマ値. sRGBからRGB空間に補正する場合は2.2を指定",
    "_root_video_colorCorrection_content_sharpening.name": "シャープネス",
    "_root_video_colorCorrection_content_sharpening.description":
        "鮮明化度. -1設定時は最も不鮮明で, 5設定時は最も鮮明化される",
    "_root_video_codec-choice-.name": "ビデオコーデック",
    "_root_video_codec-choice-.description":
        "使用されるビデオコーデック. 可能であれば, 低いビットレートでより見た目の品質が向上するH.265を推奨",
    "_root_video_codec_H264-choice-.name": "H.264",
    "_root_video_codec_H264-choice-.description": "H.264コーデックを使用する",
    "_root_video_codec_HEVC-choice-.name": "HEVC (H.265)",
    "_root_video_codec_HEVC-choice-.description": "HEVC (H.265)コーデックを使用する",
    // "_root_video_clientRequestRealtimeDecoder.name": "", // adv
    "_root_video_encodeBitrateMbs.name": "映像ビットレート",
    "_root_video_encodeBitrateMbs.description":
        "映像ストリーミングのビットレート. 30Mbpsを推奨. \nビットレートを高くすると画質が良くなるが, レイテンシと通信量が多くなる",
    // Audio tab
    "_root_audio_tab.name": "音声",
    "_root_audio_tab.description": "音声設定",
    "_root_audio_gameAudio.name": "ゲーム音声のストリーミング",
    // "_root_audio_gameAudio.description": use "_root_audio_gameAudio_enabled.description"
    "_root_audio_gameAudio_enabled.description":
        "ヘッドセットへのゲーム音声のストリーミングを有効化する",
    "_root_audio_gameAudio_content_deviceId-choice-.name": "オーディオ機器の選択",
    "_root_audio_gameAudio_content_deviceId-choice-.description":
        "音声キャプチャのために使用されるオーディオ機器",
    // "_root_audio_gameAudio_content_muteWhenStreaming.name": "",
    // "_root_audio_gameAudio_content_muteWhenStreaming.description": "",
    "_root_audio_microphone.name": "マイクのストリーミング",
    // "_root_audio_microphone.description": use "_root_audio_microphone_enabled.description"
    "_root_audio_microphone_enabled.description": "ヘッドセットのマイクをストリーミングする",
    // "_root_audio_microphone_content_deviceId-choice-.name": "",
    // "_root_audio_microphone_content_deviceId-choice-.description": "",
    // Headset tab
    "_root_headset_tab.name": "ヘッドセット",
    "_root_headset_headsetEmulationMode.name": "ヘッドセットエミュレーション",
    "_root_headset_headsetEmulationMode.description":
        "互換性向上のために様々なヘッドセットをエミュレートする",
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
    "_root_headset_trackingFrameOffset.name": "トラッキングフレームオフセット",
    "_root_headset_trackingFrameOffset.description": "姿勢予測アルゴリズム用のオフセット値",
    // "_root_headset_positionOffset.name": "", // adv
    // "_root_headset_positionOffset.description": "", // adv
    // "_root_headset_positionOffset_0.name": "", // adv
    // "_root_headset_positionOffset_1.name": "", // adv
    // "_root_headset_positionOffset_2.name": "", // adv
    "_root_headset_force3dof.name": "3DoFに強制",
    "_root_headset_force3dof.description": "強制的に3軸自由度モードにする",
    "_root_headset_controllers.name": "コントローラ",
    // "_root_headset_controllers.description": use "_root_headset_controllers_enabled.description"
    "_root_headset_controllers_enabled.description": "コントローラの使用を有効にする",
    "_root_headset_controllers_content_controllerMode.name": "コントローラエミュレーション",
    "_root_headset_controllers_content_controllerMode.description":
        "互換性向上のために様々なコントローラをエミュレートしたり, ハンドトラッキングを有効にする",
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
    "_root_headset_controllers_content_poseTimeOffset.name": "ポーズタイムオフセット", // adv
    "_root_headset_controllers_content_poseTimeOffset.description":
        "姿勢予測アルゴリズム用のオフセット値", // adv
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
    "_root_headset_controllers_content_hapticsIntensity.name": "振動の強さ",
    "_root_headset_controllers_content_hapticsIntensity.description": "触覚フィードバックの強度",
    // "_root_headset_trackingSpace-choice-.name": "",
    // "_root_headset_trackingSpace-choice-.description": "",
    // "_root_headset_trackingSpace_local-choice-.name": "",
    // "_root_headset_trackingSpace_stage-choice-.name": "",
    // Connection tab
    "_root_connection_tab.name": "接続",
    // "_root_connection_autoTrustClients.name": "", // adv
    // "_root_connection_webServerPort.name": "",
    // "_root_connection_streamPort.name": "", // adv
    // "_root_connection_streamPort.description": "", // adv
    "_root_connection_aggressiveKeyframeResend.name": "積極的キーフレーム再送",
    "_root_connection_aggressiveKeyframeResend.description": `キーフレーム間の最小間隔を100msから5msに減少させる. パケットロスが検出された場合にのみ使用され, パケットロスのあるネットワークでの使用感を改善する`,
    // Extra tab
    "_root_extra_tab.name": "その他",
    // "_root_extra_theme-choice-.name": "",
    // "_root_extra_theme-choice-.description": "",
    // "_root_extra_theme_systemDefault-choice-.name": "",
    // "_root_extra_theme_classic-choice-.name": "",
    // "_root_extra_theme_darkly-choice-.name": "",
    // "_root_extra_clientDarkMode.name": "",
    // "_root_extra_clientDarkMode.description": "",
    "_root_extra_revertConfirmDialog.name": "初期化の確認ダイアログ",
    "_root_extra_revertConfirmDialog.description": "設定値をデフォルトの値に戻す前に確認を求める",
    "_root_extra_restartConfirmDialog.name": "再起動の確認ダイアログ",
    "_root_extra_restartConfirmDialog.description": "SteamVRを再起動する前に確認求める",
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
