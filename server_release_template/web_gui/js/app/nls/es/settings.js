define({
    // Video tab
    "_root_video_tab.name": "Video",
    "_root_video_tab.description": "Ajustes de video",
    "_root_video_adapterIndex.name": "Índice de GPU", // adv
    "_root_video_adapterIndex.description": "El índice que identifica la GPU", // adv
    // "_root_video_displayRefreshRate.name": "",
    // "_root_video_displayRefreshRate.description": "",
    "_root_video_preferredFps.name": "FPS", // adv
    "_root_video_preferredFps.description": "FPS del visor", // adv
    "_root_video_resolutionDropdown.name": "Resolución de video",
    "_root_video_resolutionDropdown.description": "El 100% corresponde a la resolución nativa de 2880x1600 de la Oculus Quest.\nEl ajuste de la resolución puede mejorar ligeramente la calidad de la imagen, pero no se recomienda.\nUna resolución inferior al 100% puede reducir la latencia y mejorar la calidad de la transmisión.",
    "_root_video_renderResolution-choice-.name": "Resolución de video", // adv
    "_root_video_renderResolution_scale-choice-.name": "Usar el factor de escala", // adv
    "_root_video_renderResolution_scale-choice-.description": "Factor de escala de la resolución de video", // adv
    "_root_video_renderResolution_absolute-choice-.name": "Usar valor absoluto", // adv
    "_root_video_renderResolution_absolute-choice-.description": "Usar valor absoluto para la resolución de video", // adv
    "_root_video_renderResolution_scale.name": "Escala", // adv
    "_root_video_renderResolution_scale.description": "Factor de escala de video", // adv
    "_root_video_recommendedTargetResolution-choice-.name": "Resolución de cuadro preferida", // adv
    "_root_video_recommendedTargetResolution-choice-.description": "Resolución requerida por SteamVR para la representación", // adv
    "_root_video_recommendedTargetResolution_scale-choice-.name": "Usar factor de escala", // adv
    "_root_video_recommendedTargetResolution_absolute-choice-.name": "Usar valor absoluto", // adv
    "_root_video_recommendedTargetResolution_scale.name": "Escala", // adv
    "_root_video_recommendedTargetResolution_scale.description": "Escala", // adv
    "_root_video_secondsFromVsyncToPhotons.name": "Segundos desde el VSync a la imagen del visor", // adv
    "_root_video_secondsFromVsyncToPhotons.description": "El tiempo transcurrido desde el VSync virtual hasta que la imagen es visible en la pantalla del visor", // adv
    "_root_video_foveatedRendering.name": "Foveated rendering",
    // "_root_video_foveatedRendering.description": use "_root_video_foveatedRendering_enabled.description"
    "_root_video_foveatedRendering_enabled.description": "Técnica de renderizado que reduce la resolución de la\nimagen en la periferia de la visión para reducir la carga computacional\nde la GPU, la cantidad de datos a transmitir y la latencia.\nEste ajuste puede causar distorsión de la imagen en los bordes.",
    "_root_video_foveatedRendering_content_strength.name": "Intensidad",
    "_root_video_foveatedRendering_content_strength.description": "Los valores más altos corresponden a menos detalles en los bordes de la imagen y más artefactos.",
    "_root_video_foveatedRendering_content_shape.name": "Relación de forma", // adv
    "_root_video_foveatedRendering_content_shape.description": "La forma del rectángulo central a la resolución original", // adv
    "_root_video_foveatedRendering_content_verticalOffset.name": "Desplazamiento vertical", // adv
    "_root_video_foveatedRendering_content_verticalOffset.description": "Desplazamiento vertical del rectángulo central con la resolución original. Los valores positivos corresponden a un desplazamiento hacia abajo.", // adv
    "_root_video_colorCorrection.name": "Corrección de color",
    // "_root_video_colorCorrection.description": use "_root_video_colorCorrection_enabled.description"
    "_root_video_colorCorrection_enabled.description": "Las transformaciones de color se aplican en el siguiente orden: nitidez, gama, brillo, contraste y saturación.",
    "_root_video_colorCorrection_content_brightness.name": "Brillo",
    "_root_video_colorCorrection_content_brightness.description": "Brillo: -1 significa completamente negro y 1 significa completamente blanco.",
    "_root_video_colorCorrection_content_contrast.name": "Contraste",
    "_root_video_colorCorrection_content_contrast.description": "Contraste: -1 significa completamente gris.",
    "_root_video_colorCorrection_content_saturation.name": "Saturación",
    "_root_video_colorCorrection_content_saturation.description": "Saturación: -1 significa que la imagen está en blanco y negro.",
    "_root_video_colorCorrection_content_gamma.name": "Gama",
    "_root_video_colorCorrection_content_gamma.description": "Gama: Utilizar un valor de 2.2 para corregir el color de sRGB a RGB.",
    "_root_video_colorCorrection_content_sharpening.name": "Nitidez",
    "_root_video_colorCorrection_content_sharpening.description": "Nitidez: resalta los bordes de la imagen.",
    "_root_video_codec-choice-.name": "Códec de vídeo",
    "_root_video_codec-choice-.description": "Utilizar h265 si es posible para una mejor calidad de vídeo a velocidades de bits más bajas.",
    "_root_video_codec_H264-choice-.name": "h264",
    "_root_video_codec_H264-choice-.description": "Usar el códec h264",
    "_root_video_codec_HEVC-choice-.name": "HEVC (h265)",
    "_root_video_codec_HEVC-choice-.description": "Usar el códec HEVC (h265)",
    // "_root_video_clientRequestRealtimeDecoder.name": "", // adv
    "_root_video_encodeBitrateMbs.name": "Bitrate de video",
    "_root_video_encodeBitrateMbs.description": "Transmisión de video a velocidad de bits. Se recomiendan 30Mbps.\nUna mayor tasa de bits resulta en una mejor calidad de imagen pero a costa de una mayor latencia y tráfico de red.",
    // Audio tab
    "_root_audio_tab.name": "Audio",
    "_root_audio_tab.description": "Ajustes de audio",
    "_root_audio_gameAudio.name": "Transmitir el audio del juego",
    // "_root_audio_gameAudio.description": use "_root_audio_gameAudio_enabled.description"
    "_root_audio_gameAudio_enabled.description": "Permite la transmisión del audio del juego al visor",
    "_root_audio_gameAudio_content_deviceDropdown.name": "Elija su dispositivo de audio",
    "_root_audio_gameAudio_content_deviceDropdown.description": "Dispositivo utilizado para capturar el audio del juego",
    "_root_audio_gameAudio_content_device.name": "Código de dispositivo", // adv
    "_root_audio_gameAudio_content_device.description": "Dispositivo utilizado para capturar el audio del juego", // adv
    // "_root_audio_gameAudio_content_muteWhenStreaming.name": "",
    // "_root_audio_gameAudio_content_muteWhenStreaming.description": "",
    "_root_audio_microphone.name": "Micrófono de transmisión",
    // "_root_audio_microphone.description": use "_root_audio_microphone_enabled.description"
    "_root_audio_microphone_enabled.description": "Transmitir el audio del micrófono del visor al PC",
    "_root_audio_microphone_content_deviceDropdown.name": "Seleccionar la entrada del micrófono virtual",
    "_root_audio_microphone_content_deviceDropdown.description": "Para que el micrófono funcione correctamente, debe instalar VB-Audio Virtual u otro software equivalente.",
    // "_root_audio_microphone_content_device.name": "", // adv
    // "_root_audio_microphone_content_device.description": "", // adv
    // Headset tab
    "_root_headset_tab.name": "Visor",
    "_root_headset_headsetEmulationMode.name": "Modo de emulación del visor",
    "_root_headset_headsetEmulationMode.description": "Elija el modo de emulación del visor para mejorar la compatibilidad con algunos juegos.",
    // "_root_headset_universeId.name": "", // adv
    "_root_headset_serialNumber.name": "Número de serie", // adv
    "_root_headset_serialNumber.description": "Número de serie del visor simulado", // adv
    "_root_headset_trackingSystemName.name": "Nombre del sistema de rastreo", // adv
    "_root_headset_trackingSystemName.description": "Nombre del sistema de rastreo", // adv
    "_root_headset_modelNumber.name": "Número de modelo", // adv
    "_root_headset_modelNumber.description": "Número de modelo del visor simulado", // adv
    "_root_headset_driverVersion.name": "Versión del controlador", // adv
    "_root_headset_driverVersion.description": "Versión del controlador simulado", // adv
    "_root_headset_manufacturerName.name": "Nombre de la empresa fabricante", // adv
    "_root_headset_manufacturerName.description": "Nombre de la empresa fabricante del visor simulado", // adv
    "_root_headset_renderModelName.name": "Nombre del modelo", // adv
    "_root_headset_renderModelName.description": "Nombre del modelo del visor simulado", // adv
    "_root_headset_registeredDeviceType.name": "Tipo de dispositivo registrado", // adv
    "_root_headset_registeredDeviceType.description": "Tipo de dispositivo registrado", // adv
    "_root_headset_trackingFrameOffset.name": "Offset de seguimiento",
    "_root_headset_trackingFrameOffset.description": "Offset de seguimiento del visor utilizado por el algoritmo de predicción de posición.",
    "_root_headset_positionOffset.name": "Offset espacial del visor", // adv
    "_root_headset_positionOffset.description": "Offset espacial del visor", // adv
    "_root_headset_positionOffset_0.name": "x", // adv
    "_root_headset_positionOffset_0.description": "Offset X", // adv
    "_root_headset_positionOffset_1.name": "y", // adv
    "_root_headset_positionOffset_1.description": "Offset Y", // adv
    "_root_headset_positionOffset_2.name": "z", // adv
    "_root_headset_positionOffset_2.description": "Offset Z", // adv
    "_root_headset_force3dof.name": "Modo 3DOF",
    "_root_headset_force3dof.description": "Forzar modo de sólo 3 grados de libertad para el visor (sólo rotación)",
    "_root_headset_controllers.name": "Mandos",
    // "_root_headset_controllers.description": use "_root_headset_controllers_enabled.description"
    "_root_headset_controllers_enabled.description": "Permitir el uso de mandos",
    "_root_headset_controllers_content_controllerMode.name": "Modo de emulación del controlador",
    "_root_headset_controllers_content_controllerMode.description": "Elija el modo de emulación del controlador para mejorar la compatibilidad con ciertos juegos, y elija si desea activar la emulación del disparador con el seguimiento de la mano.",
    "_root_headset_controllers_content_modeIdx.name": "Modo", // adv
    "_root_headset_controllers_content_modeIdx.description": "Índice de modo del mando", // adv
    "_root_headset_controllers_content_trackingSystemName.name": "Nombre del sistema de rastreo", // adv
    "_root_headset_controllers_content_trackingSystemName.description": "Nombre del sistema de rastreo", // adv
    "_root_headset_controllers_content_manufacturerName.name": "Nombre de la empresa fabricante", // adv
    "_root_headset_controllers_content_manufacturerName.description": "Nombre de la empresa fabricante de los mandos simulados", // adv
    "_root_headset_controllers_content_modelNumber.name": "Número de modelo", // adv
    "_root_headset_controllers_content_modelNumber.description": "Número de modelo de los mandos simulados", // adv
    "_root_headset_controllers_content_renderModelNameLeft.name": "Nombre del modelo (mando izquierdo)", // adv
    "_root_headset_controllers_content_renderModelNameLeft.description": "Nombre del modelo de la representación visual del mando izquierdo", // adv
    "_root_headset_controllers_content_renderModelNameRight.name": "Nombre del modelo (mando derecho)", // adv
    "_root_headset_controllers_content_renderModelNameRight.description": "Nombre del modelo de la representación visual del mando derecho", // adv
    "_root_headset_controllers_content_serialNumber.name": "Número de serie", // adv
    "_root_headset_controllers_content_serialNumber.description": "Número de serie de los mandos simulados", // adv
    "_root_headset_controllers_content_ctrlType.name": "Tipo de mando", // adv
    "_root_headset_controllers_content_ctrlType.description": "Tipo de los mandos simulados", // adv
    "_root_headset_controllers_content_registeredDeviceType.name": "Tipo de dispositivo registrado", // adv
    "_root_headset_controllers_content_registeredDeviceType.description": "Nombre de los mandos simulados", // adv
    "_root_headset_controllers_content_inputProfilePath.name": "Ruta de perfil de entrada", // adv
    "_root_headset_controllers_content_inputProfilePath.description": "Ruta del archivo de perfil para la entrada del mando", // adv
    // "_root_headset_controllers_content_trackingSpeed.name": "",
    // "_root_headset_controllers_content_trackingSpeed.description": "",
    "_root_headset_controllers_content_poseTimeOffset.name": "Offset de predicción de mandos", // adv
    "_root_headset_controllers_content_poseTimeOffset.description": "Offset utilizado por los mandos para el algoritmo de predicción.", // adv
    "_root_headset_controllers_content_positionOffsetLeft.name": "Offset de posición", // adv
    "_root_headset_controllers_content_positionOffsetLeft.description": "Compensación de la posición (en metros) del mando izquierdo. \nPara el mando derecho, se utiliza el opuesto del valor X.", // adv
    "_root_headset_controllers_content_positionOffsetLeft_0.name": "x", // adv
    "_root_headset_controllers_content_positionOffsetLeft_0.description": "Offset X", // adv
    "_root_headset_controllers_content_positionOffsetLeft_1.name": "y", // adv
    "_root_headset_controllers_content_positionOffsetLeft_1.description": "offset Y", // adv
    "_root_headset_controllers_content_positionOffsetLeft_2.name": "z", // adv
    "_root_headset_controllers_content_positionOffsetLeft_2.description": "Offset Z", // adv
    "_root_headset_controllers_content_rotationOffsetLeft.name": "Offset de rotación", // adv
    "_root_headset_controllers_content_rotationOffsetLeft.description": "Desplazamiento de la rotación en grados para el mando izquierdo. \nPara el mando derecho, las rotaciones a lo largo de los ejes Y y Z se invierten.", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_0.name": "x", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_0.description": "Y rotación", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_1.name": "y", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_1.description": "Y rotación", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_2.name": "z", // adv
    "_root_headset_controllers_content_rotationOffsetLeft_2.description": "Z rotación", // adv
    "_root_headset_controllers_content_hapticsIntensity.name": "Intensidad de la retroalimentación táctil",
    "_root_headset_controllers_content_hapticsIntensity.description": "Factor para reducir o aumentar la intensidad de la vibración de los mandos.",
    // "_root_headset_trackingSpace-choice-.name": "",
    // "_root_headset_trackingSpace-choice-.description": "",
    // "_root_headset_trackingSpace_local-choice-.name": "",
    // "_root_headset_trackingSpace_stage-choice-.name": "",
    // Connection tab
    "_root_connection_tab.name": "Conexión",
    "_root_connection_disableThrottling.name": "Deshabilitar los límites de envío",
    "_root_connection_disableThrottling.description": "Deshabilitar los límites de tiempo para el envío de paquetes.",
    "_root_connection_bufferOffset.name": "Offset de buffer",
    "_root_connection_bufferOffset.description": "Offset utilizado para aumentar o disminuir el tamaño del buffer calculado para el cliente. El tamaño del buffer resultante nunca será negativo.",
    // "_root_connection_autoTrustClients.name": "", // adv
    // "_root_connection_webServerPort.name": "",
    "_root_connection_listenPort.name": "Puerto del servidor", // adv
    "_root_connection_listenPort.description": "Puerto utilizado por el servidor para recibir paquetes.", // adv
    "_root_connection_throttlingBitrateBits.name": "Limitación de la tasa de bits", // adv
    "_root_connection_throttlingBitrateBits.description": "Velocidad de bitrate máxima permitida en bits.", // adv
    "_root_connection_clientRecvBufferSize.name": "Tamaño del buffer para el cliente", // adv
    "_root_connection_clientRecvBufferSize.description": "Tamaño del buffer para el cliente. \nDepende de la tasa de bits. \nSe recomienda dejar el valor calculado. Si tiene problemas de pérdida de paquetes, aumente el valor.", // adv
    "_root_connection_aggressiveKeyframeResend.name": "Reenvía los fotogramas clave de forma agresiva",
    "_root_connection_aggressiveKeyframeResend.description": "Reducir el intervalo de reenvío de cuadros de tipo \"I\" (key frames) de 100ms a 5ms.\nSólo se utiliza cuando se detecta una pérdida de paquetes. Mejora la experiencia visual en caso de pérdida de paquetes.",
    // Extra tab
    "_root_extra_tab.name": "Extra",
    "_root_extra_theme-choice-.name": "Estilo",
    "_root_extra_theme-choice-.description": "Ven al Lado Oscuro. \nTenemos galletas.",
    "_root_extra_theme_systemDefault-choice-.name": "Sistema",
    "_root_extra_theme_classic-choice-.name": "Clasico",
    "_root_extra_theme_darkly-choice-.name": "Oscuro",
    // "_root_extra_clientDarkMode.name": "",
    // "_root_extra_clientDarkMode.description": "",
    "_root_extra_revertConfirmDialog.name": "Confirmar los valores de restablecimiento",
    "_root_extra_revertConfirmDialog.description": "Pedir confirmación antes de restablecer los ajustes al valor predeterminado.",
    "_root_extra_restartConfirmDialog.name": "Confirmación de reinicio del SteamVR",
    "_root_extra_restartConfirmDialog.description": "Pedir confirmación antes de reiniciar el SteamVR.",
    // "_root_extra_promptBeforeUpdate.name": "",
    // "_root_extra_updateChannel-choice-.name": "",
    // "_root_extra_updateChannel_noUpdates-choice-.name": "",
    // "_root_extra_updateChannel_stable-choice-.name": "",
    // "_root_extra_updateChannel_beta-choice-.name": "",
    // "_root_extra_updateChannel_nightly-choice-.name": "",
    // "_root_extra_logToDisk.name": "",
    "_root_extra_notificationLevel-choice-.name": "Grado de notificaciones", // adv
    "_root_extra_notificationLevel-choice-.description": "Grado de registro con el que se genera una notificación.", // adv
    "_root_extra_notificationLevel_error-choice-.name": "Error", // adv
    "_root_extra_notificationLevel_warning-choice-.name": "Aviso", // adv
    "_root_extra_notificationLevel_info-choice-.name": "Información", // adv
    "_root_extra_notificationLevel_debug-choice-.name": "Debug", // adv
    "_root_extra_excludeNotificationsWithoutId.name": "Excluir las notificaciones sin identificación", // adv
    "_root_extra_excludeNotificationsWithoutId.description": "No mostrar notificaciones que no contengan la estructura de identificación.", // adv
    // Others
    "steamVRRestartSuccess": "SteamVR reiniciado con éxito",
    "audioDeviceError": "No se encontraron dispositivos de audio. No se puede transmitir audio o micrófono.",
});