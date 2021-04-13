import "bootstrap"
import "bootstrap/dist/css/bootstrap.min.css"
import "../resources/style.css"
import i18next from "i18next"
import I18nextBrowserLanguageDetector from "i18next-browser-languagedetector"
import I18NextHttpBackend from "i18next-http-backend"
import { library, dom } from "@fortawesome/fontawesome-svg-core"
import { faPlus, faMinus } from "@fortawesome/free-solid-svg-icons"

library.add(faPlus, faMinus)
dom.watch()

i18next
    .use(I18NextHttpBackend)
    .use(I18nextBrowserLanguageDetector)
    .init({
        fallbackLng: "en",
        debug: true,
        returnObjects: false,
        detection: { order: ["navigator", "localStorage"] },
        interpolation: { escapeValue: false },
        backend: { loadPath: "/languages/{{lng}}.json" },
    })

// Exports for WASM:
window.trans_key_exists = i18next.exists
window.t = i18next.t
window.change_language = async code => {
    window.t = await i18next.changeLanguage(code)
}

// WASM entry point
import("../pkg/index.js").catch(console.error)
