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
            let updateType = session.sessionSettings.extra.updateChannel.variant;

            let url = "";
            if (updateType === "noUpdates") {
                return;
            } else if (updateType === "nightly") {
                url = "https://api.github.com/repos/alvr-org/ALVR-nightly/releases/latest";
            } else { // stable and beta
                url = "https://api.github.com/repos/alvr-org/ALVR/releases/latest";
            }

            $.get(url, (data) => {
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
                        onClick: () => showUpdatePopupDialog(data)
                    });
                } else {
                    triggerUpdate(data);
                }
            });
        }

        function showUpdatePopupDialog(data) {
            var compiledTemplate = _.template(updatePopup);
            var template = compiledTemplate(i18n);
            $("#confirmModal").remove();
            $("body").append(template);
            $(document).ready(() => {
                $("#releaseTitle").text(data.name);
                $("#releaseNote").text(data.body);

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
                    triggerUpdate(data);
                });
                $("#moreUpdateButton").click(() => {
                    $.ajax({
                        headers: {
                            Accept: "application/json",
                            "Content-Type": "application/json",
                        },
                        type: "POST",
                        url: "/open",
                        data: JSON.stringify(data.html_url),
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
                return
            }

            $("#setupWizard").modal("hide");
            $("#bodyContent").hide();
            $("#updating").show();

            const elem = document.getElementById("progressBar");

            // Create WebSocket connection.
            const webSocket = new WebSocket("ws://" + window.location.host + "/events");

            $.ajax({
                type: "POST",
                url: "/update",
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
                        if (dataJSON.id === "updateDownloadedBytesCount") {
                            const BtoMB = 1.0 / (1024 * 1024);
                            const sizeMb = size * BtoMB;
                            const downloadProgress = (dataJSON.data * BtoMB).toFixed(2);
                            document.getElementById("downloadProgress").innerHTML = downloadProgress + "MB" + " / " + sizeMb.toFixed(2) + "MB";
                            const progress = (100.0 * dataJSON.data / size).toFixed(2);
                            elem.style.width = progress + "%";
                            elem.innerHTML = progress + "%";
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
