define({
    root: {
        // Banner
        title: "Welcome to ALVR",
        subtitle: "This setup wizard will guide you to the basic setup of ALVR",
        // Hardware page
        titleHardwareReq: "Hardware requirements",
        textHardwareReq:
            "ALVR requires a dedicated and recent graphics card. <br/> <br/> Make sure you have at least one output audio device. <br/> <br/> ",
        YourGPUIs: "Your GPU(s):",
        GPUSupported: " \nGreat! This GPU is probably supported!",
        GPUUnsupported:
            " \nWe are sorry, but this card may be unsupported. You can try ALVR anyway and see if it works",
        // Software page
        titleSoftwareReq: "Software requirements",
        textSoftwareReq:
            "To stream the Quest microphone on Windows you need to install <a target= '_blank' href='https://www.vb-audio.com/Cable/'>VB-Audio Virtual Cable</a>. <br> On Linux some feaures are not working and should be disabled (foveated encoding and color correction) and some need a proper environment setup to have them working (game audio and microphone streaming).",
        // Firewall page
        titleFirewall: "Firewall",
        textFirewall:
            "To communicate with the headset, some firewall rules need to be set. <br/> <b>This requires administrator rights!</b>",
        buttonFirewall: "Add firewall rules",
        firewallFailed: "Setting firewall rules failed",
        firewallSuccess: "Firewall rules successfully set",
        // Performance page
        titlePerformance: "Performance preset",
        textPerformance:
            "Please choose preset that fits your setup. This will adjust some settings for you.",
        compatPerformance: "Compatibility",
        qualityPerformance: "Visual quality",
        // Recommendations page
        titleRecommendations: "Recommendations",
        textRecommendations:
            "ALVR supports multiple types of PC hardware and headsets but not all work correctly with default settings. For example some AMD video cards work only with the HEVC codec and GearVR does not support foveated encoding. Please try tweaking different settings if your ALVR experience is broken or not optimal.",
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
