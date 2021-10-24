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
    "app/languageSelector",
    "json!../../api/session/load",
    "text!../../api/version",
    // eslint-disable-next-line requirejs/no-js-extension
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
    languageSelector,
    session,
    version,
) {
    $(function () {
        const compiledTemplate = _.template(mainTemplate);
        const template = compiledTemplate(i18n);

        function checkForUpdate(settings, delay) {
            session = settings.getSession();
            const updateType =
                session.sessionSettings.extra.updateChannel.variant;

            let url = "";
            if (updateType === "stable") {
                url =
                    "https://api.github.com/repos/alvr-org/ALVR/releases/latest";
            } else if (updateType === "beta") {
                url =
                    "https://api.github.com/repos/alvr-org/ALVR/releases?per_page=1";
            } else if (updateType === "nightly") {
                url =
                    "https://api.github.com/repos/alvr-org/ALVR-nightly/releases/latest";
            } else {
                return;
            }

            $.get(url, (data) => {
                if (updateType === "beta") {
                    data = data[0];
                }

                if (data.tag_name === "v" + version) {
                    Lobibox.notify("success", {
                        size: "mini",
                        rounded: true,
                        delayIndicator: false,
                        sound: false,
                        iconSource: "fontAwesome",
                        msg: i18n.noNeedForUpdate,
                    });
                    return;
                }

                if (session.sessionSettings.extra.promptBeforeUpdate) {
                    Lobibox.notify("warning", {
                        size: "mini",
                        rounded: true,
                        delay: delay,
                        delayIndicator: delay !== -1,
                        sound: false,
                        iconSource: "fontAwesome",
                        msg: i18n.needUpdateClickForMore,
                        closable: true,
                        onClick: () => showUpdatePopupDialog(data),
                    });
                } else {
                    triggerUpdate(data);
                }
            });
        }

        function showUpdatePopupDialog(data) {
            const compiledTemplate = _.template(updatePopup);
            const template = compiledTemplate(i18n);
            $("#confirmModal").remove();
            $("body").append(template);

            const _data = data;
            // this call need const variable unless you want them overwriten by the next call.
            $(document).ready(() => {
                $("#releaseTitle").text(_data.name);
                $("#releaseNote").text(_data.body);

                $("#confirmModal").modal({
                    backdrop: "static",
                    keyboard: false,
                });
                $("#cancelUpdateButton").click(() => {
                    $("#confirmModal").modal("hide");
                    $("#confirmModal").remove();
                });
                $("#okUpdateButton").click(() => {
                    $("#confirmModal").modal("hide");
                    $("#confirmModal").remove();
                    triggerUpdate(_data);
                });
                $("#moreUpdateButton").click(() => {
                    $.ajax({
                        headers: {
                            Accept: "application/json",
                            "Content-Type": "application/json",
                        },
                        type: "POST",
                        url: "/api/open",
                        // eslint-disable-next-line xss/no-mixed-html
                        data: JSON.stringify(_data.html_url),
                        dataType: "JSON",
                    });
                });
            });
        }

        function triggerUpdate(data) {
            let url = "";
            let size = 0;
            data.assets.forEach((asset) => {
                if (asset.name.startsWith("ALVR_Installer")) {
                    url = asset.browser_download_url;
                    size = asset.size;
                }
            });
            if (url === "") {
                return;
            }

            $("#setupWizard").modal("hide");
            $("#bodyContent").hide();
            $("#updating").show();

            const elem = document.getElementById("progressBar");

            // Create WebSocket connection.
            const webSocket = new WebSocket(
                "ws://" + window.location.host + "/api/events",
            );

            $.ajax({
                type: "POST",
                url: "/api/update",
                contentType: "application/json;charset=UTF-8",
                data: JSON.stringify(url),
                success: function (res) {
                    if (res === "") {
                        console.log("Success");
                    } else {
                        console.log("Info: ", res);
                        webSocket.close();
                        $("#bodyContent").show();
                        $("#updating").hide();
                    }
                },
                error: function (res) {
                    console.log("Error: ", res);
                    webSocket.close();
                    $("#bodyContent").show();
                    $("#updating").hide();
                },
            });

            if (webSocket !== null && typeof webSocket !== undefined) {
                webSocket.onmessage = function (event) {
                    try {
                        const dataJSON = JSON.parse(event.data);
                        if (dataJSON.id === "UpdateDownloadedBytesCount") {
                            const BtoMB = 1.0 / (1024 * 1024);
                            const sizeMb = size * BtoMB;
                            const downloadProgress = (
                                dataJSON.data * BtoMB
                            ).toFixed(2);
                            document.getElementById(
                                "downloadProgress",
                            ).innerText =
                                downloadProgress +
                                "MB" +
                                " / " +
                                sizeMb.toFixed(2) +
                                "MB";
                            const progress = (
                                (100.0 * dataJSON.data) /
                                size
                            ).toFixed(2);
                            elem.style.width = progress + "%";
                            elem.innerText = progress + "%";
                        }
                    } catch (error) {
                        console.log("Error with message: ", event);
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
                };
            }
        }

        $("#bodyContent").append(template);
        $(document).ready(() => {
            $("#loading").remove();
            let settings = null;
            let wizard = null;
            let monitor = null;
            let language = null;
            try {
                settings = new Settings();
                checkForUpdate(settings, -1);
                wizard = new SetupWizard(settings);
                monitor = new Monitor(settings);
                language = new languageSelector(settings);
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
            const sessionLocale = session.locale;

            language.addLanguageSelector("localeSelector", sessionLocale);

            language.addLanguageSelector("localeSelectorV", sessionLocale);

            let storedLocale = localStorage.getItem("locale");
            if (sessionLocale !== storedLocale && sessionLocale !== "system") {
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

            $("#version").text("v" + version);

            driverList.fillDriverList("registeredDriversInst");

            uploadPreset.addUploadPreset(
                "settingUploadPreset",
                settings.getWebClientId(),
            );
        });
    });
});
