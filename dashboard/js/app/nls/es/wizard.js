define({
    // Banner
    title: "Bienvenido a ALVR",
    subtitle: "Este asistente de configuración le guiará en la configuración básica de ALVR",
    // Hardware page
    titleHardwareReq: "Requisitos de hardware",
    textHardwareReq:
        "ALVR requiere una tarjeta gráfica dedicada reciente. <br/> <br/> Asegúrate de tener al menos un dispositivo de salida de audio. <br/> <br/> ",
    YourGPUIs: "Tu tarjeta gráfica es",
    GPUSupported: "<br/>Excelente! Esta tarjeta gráfica es soportada!",
    GPUUnsupported:
        "<br/>Lo sentimos, pero esta tarjeta gráfica podría no ser soportada. Puedes probar ALVR y ver si funciona.",
    // Software page
    titleSoftwareReq: "Requisitos de software",
    textSoftwareReq:
        "Para transmitir el audio desde el micrófono del visor necesitas instalar <a target= '_blank' href='https://www.vb-audio.com/Cable/'>VB-Audio Virtual Cable</a>.",
    // Firewall page
    titleFirewall: "Cortafuegos",
    textFirewall:
        "Para comunicarse con el visor se deben establecer ciertas excepciones en el cortafuegos. <br/> <b>Esto requiere privilegios de administrador.</b>",
    buttonFirewall: "Añadir excepción al cortafuegos",
    firewallFailed: "Error al añadir la excepción al cortafuegos",
    firewallSuccess: "Se ha añadido la excepción al cortafuegos con éxito",
    // Tracking page
    titleTracking: "Seguimiento",
    textTracking: `Elija el modo de seguimiento para los mandos. Los juegos frenéticos como BeatSaber requieren un modo "rápido". Para juegos más tranquilos como Skyrim, elige "medio" o "lento".  <br/> <br/> La predicción de Oculus significa que la posición del controlador se predice en el visor en lugar de en la PC a través del SteamVR.`,
    oculusTracking: "Predicción de Oculus",
    normalTracking: "Lento",
    mediumTracking: "Medio",
    fastTracking: "Rápido",
    // Performance page
    titlePerformance: "Ajustes de rendimiento",
    textPerformance:
        "Elija el modo que mejor se adapte a su PC. Algunos valores se ajustarán automáticamente.",
    compatPerformance: "Compatibilidad",
    qualityPerformance: "Calidad visual",
    // Import page
    titleImport: "Importar preconfiguraciones de ALVR",
    textImport:
        "Puedes importar configuracioneso o preconfiguraciones (.json): <ul><li> Preconfiguraciones para un visor específico. Se recomienda que para el visor de <b>Oculus Go</b> (utilizar <code>oculus_go_preset.json</code> de la carpeta de instalación).<li> Ajustes de una instalación previa de ALVR (<code>session.json</code>).<ul>",
    // End page
    titleFinished: "Listo!",
    textFinished: `Puedes reiniciar esta guía desde la sección "Instalación" en el menu la izquierda.`,
    buttonBack: "Atrás",
    buttonNext: "Siguiente",
    buttonClose: "Cerrar",
});
