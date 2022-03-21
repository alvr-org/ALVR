define({
    // Banner
    title: "Welkom bij ALVR",
    subtitle: "Deze instalatiewizard leidt u doorheen de basisconfiguratie van ALVR",
    // Hardware page
    titleHardwareReq: "Systeemvereisten",
    textHardwareReq:
        "ALVR heeft een discrete en recente grafische kaart nodig. <br/> <br/> Zorg ervoor dat je minstens één audio uitvoerapparaat hebt. <br/> <br/> ",
    YourGPUIs: "Jouw GPU:",
    GPUSupported: "\nGeweldig! Deze GPU is waarschijnlijk ondersteund!", //Yes, '\n' must be in translation
    GPUUnsupported:
        "\nHet spijt ons, deze GPU is niet ondersteund. Je kan nog steeds ALVR proberen om te kijken ofdat het toch werkt", //Yes, '\n' must be in translation
    // Software page
    titleSoftwareReq: "Softwarevereisten",
    textSoftwareReq:
        "Om de microfoon van de Quest te streamen moet je <a target= '_blank' href='https://www.vb-audio.com/Cable/'>VB-Audio Virtual Cable</a> installeren.",
    // Firewall page
    titleFirewall: "Firewall",
    textFirewall:
        "Om te communiceren met de headset, zullen er een aantal Firewall regels toegevoegd worden. <br/> <b>Dit vereist administrator rechten!</b>",
    buttonFirewall: "Firewall regels toevoegen",
    firewallFailed: "Firewall regels zijn niet succesfol toegevoegd",
    firewallSuccess: "Firewall regels zijn succesfol toegevoegd",
    // Tracking page
    titleTracking: "Tracking",
    textTracking:
        "Hoe moet het volgen van de controller worden afgehandeld?. Als je een spel speelt waarbij er snelle bewegingen nodig zijn zoals: Beatsaber, kies medium of snel. Voor spellen met tragere beweging zoals Skyrim, laat het dan op normaal staan.\n\nOculus voorspelling betekent dat de controller positie voorspelt word op headset in de plaats van op de PC via SteamVR.",
    oculusTracking: "Oculus voorspelling",
    normalTracking: "Normaal",
    mediumTracking: "Medium",
    fastTracking: "Snel",
    // Performance page
    titlePerformance: "Performance voorinstelling",
    textPerformance:
        "Kies een voorinstelling die bij uw setup past. Hierdoor worden er enkele instellingen voor u aangepast.",
    compatPerformance: "Compatibiliteit",
    qualityPerformance: "Visuele kwaliteit",
    // Import page
    titleImport: "Importeer ALVR voorinstelling",
    textImport: `Je kan instellingen of voorinstellingen bestanden importeren (.json):
    <ul>
        <li> Voorinstellingen voor een specifieke headset. Dit is aanbeloven voor de <b>Oculus Go</b> (zoek voor <code>oculus_go_preset.json</code> in de installatie map).
        <li> Instellingen van een vorige ALVR installatie (<code>session.json</code>).
    <ul>`,
    // End page
    titleFinished: "Voltooid",
    textFinished:
        'Je kan altijd deze installatie wizard herstarten via de "Installation" tablad aan je linkerkant',
    buttonNext: "Volgende",
    buttonClose: "Sluiten",
});
