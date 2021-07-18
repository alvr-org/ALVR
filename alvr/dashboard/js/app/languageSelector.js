define([
    "lib/lodash",
    "text!app/templates/languageSelector.html",
    "i18n!app/nls/main",
    "json!../../api/session/load",
    "app/nls/main",
    "app/languageList",
], function (_, languageSelector, i18n, session, main, languageList) {
    return function (alvrSettings) {
        this.addLanguageSelector = function (elementId, sessionLocale) {
            const compiledTemplate = _.template(languageSelector);
            const template = compiledTemplate({ id: elementId, ...i18n });

            $("#" + elementId).empty();
            $("#" + elementId).append(template);

            const _elementId = elementId;
            // this call need const variable unless you want them overwriten by the next call.
            $(document).ready(() => {
                const selectElement = document.getElementById("localeChange_" + _elementId);
                // remove the "root": {}, from the main object
                delete main.root;
                // add english to the list
                Object.assign(main, { en: true });
                // keep only the key (languages codes) from the main object
                const availableLanguage = Object.keys(main).sort();
                // for each languages create the html element
                availableLanguage.forEach((element) => {
                    selectElement.options[selectElement.options.length] = new Option(
                        languageList[element].nativeName,
                        element
                    );
                });
                // select the current languages.
                selectElement.value = sessionLocale;
            });

            // change locale after selected a new one
            $(document).on("change", "#localeChange_" + elementId, () => {
                const storedLocale = document.getElementById("localeChange_" + elementId).value;
                session.locale = storedLocale;
                alvrSettings.updateSession(session);
                alvrSettings.storeSession("other");
                if (storedLocale === "system") {
                    if (localStorage.getItem("locale") !== null) {
                        localStorage.removeItem("locale");
                    }
                } else {
                    localStorage.setItem("locale", storedLocale);
                }
                window.location.reload();
            });
        };
    };
});
