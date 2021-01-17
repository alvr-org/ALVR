define([
    "jquery",
    "lib/bootstrap.bundle.min",
    "lib/lodash",
    "text!app/templates/main.html",
    "i18n!app/nls/main",
    "app/settings",
    "app/setupWizard",
    "text!app/templates/updatePopup.html",
    "app/monitor",
    "app/driverList",
    "app/uploadPreset",
    "json!../../session",
    "text!../../version",
    "js/lib/lobibox.min.js",
    "css!js/lib/lobibox.min.css",
], function (
    $,
    bootstrap,
    _,
    mainTemplate,
    i18n,
    Settings,
    SetupWizard,
    updatePopup,
    Monitor,
    driverList,
    uploadPreset,
    session,
    version,
) {
    $(function () {
        var compiledTemplate = _.template(mainTemplate);
        var template = compiledTemplate(i18n);

        function checkForUpdate(settings, delay) {
            session = settings.getSession();
            let updateType =
                session.sessionSettings.extra.updateChannel.variant;
            if (updateType !== "noUpdates") {
                let url =
                    "https://api.github.com/repos/alvr-org/ALVR/releases/latest";
                if (updateType === "beta") {
                    url = "https://api.github.com/repos/alvr-org/ALVR/releases";
                } else if (updateType === "nightly") {
                    url =
                        "https://api.github.com/repos/alvr-org/ALVR-nightly/releases/latest";
                }
                $.get(url, (data) => {
                    if (updateType === "beta") {
                        data = data[0];
                    }
                    const currentVersion = "v" + version;
                    // const releaseVersion = data.tag_name.match(/\d+.\d+.\d+/,)[0];
                    const releaseVersion = data.tag_name;
                    if (currentVersion === releaseVersion) {
                        Lobibox.notify("success", {
                            size: "mini",
                            rounded: true,
                            delayIndicator: false,
                            sound: false,
                            iconSource: "fontAwesome",
                            msg: i18n.noNeedForUpdate,
                        });
                    } else {
                        Lobibox.notify("warning", {
                            size: "mini",
                            rounded: true,
                            delay: delay,
                            delayIndicator: delay !== -1,
                            sound: false,
                            iconSource: "fontAwesome",
                            msg: i18n.needUpdateClickForMore,
                            closable: true,
                            onClick: function () {
                                const releaseNote = data.body;
                                const infoURL = data.html_url;
                                showUpdatePopupDialog(
                                    releaseVersion,
                                    releaseNote,
                                    infoURL,
                                ).then((res) => {
                                    if (res) {
                                        let url = "";
                                        let size = 0;
                                        data.assets.forEach((asset) => {
                                            const found = asset.name.match(".*\.exe$");
                                            if (found) {
                                                url =
                                                    asset.browser_download_url;
                                                size = asset.size;
                                            }
                                        });
                                        if (url !== "") {
                                            triggerUpdate(url);
                                        }
                                    }
                                });
                            },
                        });
                    }
                });
            }
        }

        function showUpdatePopupDialog(releaseVersion, releaseNote, infoURL) {
            return new Promise((resolve) => {
                var compiledTemplate = _.template(updatePopup);
                var template = compiledTemplate(i18n);
                $("#confirmModal").remove();
                $("body").append(template);
                $(document).ready(() => {
                    $("#releaseVersion").text(releaseVersion);
                    $("#releaseNote").text(releaseNote);

                    $("#confirmModal").modal({
                        backdrop: "static",
                        keyboard: false,
                    });
                    $("#confirmModal").on("hidden.bs.modal", (e) => {
                        resolve(false);
                    });
                    $("#cancelUpdateButton").click(() => {
                        resolve(false);
                        $("#confirmModal").modal("hide");
                        $("#confirmModal").remove();
                    });
                    $("#okUpdateButton").click(() => {
                        resolve(true);
                        $("#confirmModal").modal("hide");
                        $("#confirmModal").remove();
                    });
                    $("#moreUpdateButton").click(() => {
                        $.ajax({
                            headers: {
                                Accept: "application/json",
                                "Content-Type": "application/json",
                            },
                            type: "POST",
                            url: "/open",
                            data: JSON.stringify(infoURL),
                            dataType: "JSON",
                        });
                    });
                });
            });
        }

        function triggerUpdate(url) {
            url = url.replace("https", "http");
            $.ajax({
                type: "POST",
                url: "/update",
                contentType: "application/json;charset=UTF-8",
                data: JSON.stringify(url),
            });
            Lobibox.notify("success", {
                size: "mini",
                rounded: true,
                delayIndicator: false,
                sound: false,
                iconSource: "fontAwesome",
                msg: i18n.noNeedForUpdate,
            });
        }

        $("#bodyContent").append(template);
        $(document).ready(() => {
            $("#loading").remove();
            try {
                var settings = new Settings();
                checkForUpdate(settings, -1);
                var wizard = new SetupWizard(settings);
                var monitor = new Monitor(settings);
            } catch (error) {
                Lobibox.notify("error", {
                    rounded: true,
                    delay: -1,
                    delayIndicator: false,
                    sound: false,
                    position: "bottom left",
                    iconSource: "fontAwesome",
                    msg: error.stack,
                    closable: true,
                    messageHeight: 250,
                });
            }

            // update the current language on startup
            let sessionLocale = session.locale;
            $("#localeChange").val(sessionLocale);
            let storedLocale = localStorage.getItem("locale");
            if (
                sessionLocale !== storedLocale &&
                storedLocale !== null &&
                sessionLocale !== "system"
            ) {
                storedLocale = sessionLocale;
                localStorage.setItem("locale", storedLocale);
                window.location.reload();
            }

            $("#bodyContent").fadeIn(function () {
                if (session.setupWizard) {
                    setTimeout(() => {
                        wizard.showWizard();
                    }, 500);
                }
            });

            $("#runSetupWizard").click(() => {
                wizard.showWizard();
            });

            $("#addFirewallRules").click(() => {
                $.get("firewall-rules/add", undefined, (res) => {
                    if (res == 0) {
                        Lobibox.notify("success", {
                            size: "mini",
                            rounded: true,
                            delayIndicator: false,
                            sound: false,
                            msg: i18n.firewallSuccess,
                        });
                    }
                });
            });

            $("#removeFirewallRules").click(() => {
                $.get("firewall-rules/remove", undefined, (res) => {
                    if (res == 0) {
                        Lobibox.notify("success", {
                            size: "mini",
                            rounded: true,
                            delayIndicator: false,
                            sound: false,
                            msg: i18n.firewallSuccess,
                        });
                    }
                });
            });

            $("#checkForUpdates").click(() => {
                checkForUpdate(settings, 5000);
            });

            $("#localeChange").change(() => {
                storedLocale = $("#localeChange").val();
                session.locale = storedLocale;
                settings.updateSession(session);
                settings.storeSession("other");
                if (storedLocale === "system") {
                    if (localStorage.getItem("locale") !== null) {
                        localStorage.removeItem("locale");
                    }
                } else {
                    localStorage.setItem("locale", storedLocale);
                }
                window.location.reload();
            });

            $("#version").text("v" + version);

            driverList.fillDriverList("registeredDriversInst");

            uploadPreset.addUploadPreset(
                "restartSteamVR",
                settings.getWebClientId(),
            );
        });
    });
});
