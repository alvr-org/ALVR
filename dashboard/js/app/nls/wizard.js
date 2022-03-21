define({
    root: {
        // Banner
        title: "Welcome to ALVR",
        subtitle: "This setup wizard will guide you to the basic setup of ALVR",
        // Hardware page
        titleHardwareReq: "Hardware requirements",
        textHardwareReq:
            "ALVR requires a dedicated and recent graphics card. <br/> <br/> Make sure you have at least one output audio device. <br/> <br/> ",
        YourGPUIs: "Your GPU:",
        GPUSupported: " \nGreat! This GPU is probably supported!",
        GPUUnsupported:
            " \nWe are sorry, but this card may be unsupported. You can try ALVR anyway and see if it works",
        // Software page
        titleSoftwareReq: "Software requirements",
        textSoftwareReq:
            "To stream the Quest microphone you need to install the <a target= '_blank' href='https://www.vb-audio.com/Cable/'>VB-Audio Virtual Cable</a>.",
        // Firewall page
        titleFirewall: "Firewall",
        textFirewall:
            "To communicate with the headset, some firewall rules need to be set. <br/> <b>This requires administrator rights!</b>",
        buttonFirewall: "Add firewall rules",
        firewallFailed: "Setting firewall rules failed",
        firewallSuccess: "Firewall rules successfully set",
        // Tracking page
        titleTracking: "Tracking",
        textTracking:
            "How should the tracking of the controller be handled. Recommended to use adaptive Oculus or SteamVR prediction. If you want to use fixed tracking speeds: Medium or fast for fast paced games like Beatsaber, normal for slower games like Skyrim. <br/> <br/> Oculus prediction means controller position is predicted on the headset instead of on the PC through SteamVR.",
        oculusTracking: "Oculus",
        steamvrTracking: "SteamVR",
        normalTracking: "Normal",
        mediumTracking: "Medium",
        fastTracking: "Fast",
        // Performance page
        titlePerformance: "Performance preset",
        textPerformance:
            "Please choose preset that fits your setup. This will adjust some settings for you.",
        compatPerformance: "Compatibility",
        qualityPerformance: "Visual quality",
        // Import page
        titleImport: "Import ALVR preset",
        textImport:
            "You can import settings or preset files (.json): <ul><li> Presets for a specific headset. This is recommended for the <b>Oculus Go</b> (search for <code>oculus_go.json</code> in the installation folder).<li> Settings from a previous ALVR installation (<code>session.json</code>).<ul>",
        // End page
        titleFinished: "Finished",
        textFinished:
            'You can always restart this setup wizard from the "Installation" tab on the left',
        buttonBack: "Back",
        buttonNext: "Next",
        buttonClose: "Close",
    },
    it: true,
    sl: true,
    es: true,
    fr: true,
    ja: true,
    zh: true,
    ru: true,
    bg: true,
    de: true,
    nl: true,
    ko: true,
});
