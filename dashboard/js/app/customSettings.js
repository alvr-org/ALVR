define([
    "i18n!app/nls/settings",
    "i18n!app/nls/wizard",
    "lib/selectal",
    "json!../../api/audio-devices",
    "json!app/resources/HTCVive.json",
    "json!app/resources/OculusRift.json",
    "json!app/resources/Quest2.json",
    "json!app/resources/OculusTouch.json",
    "json!app/resources/ValveIndex.json",
    "json!app/resources/HTCViveWand.json",
    "json!app/resources/Quest2Touch.json",
    "json!app/resources/HTCViveTracker.json",
], function (
    i18n,
    i18nWizard,
    select,
    audio_devices,
    vive,
    rifts,
    quest2,
    touch,
    index,
    vivewand,
    q2touch,
    vivetracker
) {
    return function (alvrSettings) {
        const self = this;
        const video_scales = [25, 50, 66, 75, 100, 125, 150, 200];

        self.setCustomSettings = function () {
            try {
                setAudioDeviceList();
                setVideoOptions();
                setRefreshRate();
                setHeadsetEmulation();
                setControllerEmulation();
                setTrackingSpeed();
                setTheme();
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
        };

        function setControllerEmulation() {
            let controller = $("#_root_headset_controllers_content_controllerMode");
            controller.unbind();
            controller.after(
                alvrSettings.getHelpReset(
                    "controllerMode",
                    "_root_headset_controllers_content",
                    6,
                    (postFix = ""),
                    "controllerMode",
                    "Oculus Quest 2"
                )
            );
            controller.parent().addClass("special");

            const controllerBase = "#_root_headset_controllers_content_";
            const controllerMode = $(controllerBase + "modeIdx");
            const controllerOptions = [
                touch,
                touch,
                index,
                index,
                vivewand,
                vivewand,
                q2touch,
                q2touch,
                vivetracker,
                vivetracker,
            ];

            controller.append(`<option value="1">Oculus Rift S</option>`);
            controller.append(`<option value="3">Valve Index</option>`);
            controller.append(`<option value="5">HTC Vive</option>`);
            controller.append(`<option value="7">Oculus Quest 2</option>`);
            controller.append(`<option value="9">HTC Vive Tracker</option>`);

            const select = new Selectal("#_root_headset_controllers_content_controllerMode");
            controller = $("#_root_headset_controllers_content_controllerMode");

            controller.val(controllerMode.val());
            controller.change();

            controller.change((ev) => {
                for (const key in controllerOptions[controller.val()]) {
                    const target = $(controllerBase + key);
                    target.val(controllerOptions[controller.val()][key]);
                    alvrSettings.storeParam(target, true);
                }
                controllerMode.val(controller.val());
                alvrSettings.storeParam(controllerMode, true);

                alvrSettings.storeSession("settings");
            });
        }

        function setHeadsetEmulation() {
            let headset = $("#_root_headset_headsetEmulationMode");
            headset.unbind();
            headset.after(
                alvrSettings.getHelpReset(
                    "headsetEmulationMode",
                    "_root_headset",
                    2,
                    (postFix = ""),
                    "headsetEmulationMode",
                    "Oculus Quest 2"
                )
            );
            headset.parent().addClass("special");

            const headsetBase = "#_root_headset_";
            const headsetMode = $(headsetBase + "modeIdx");
            const headsetOptions = [rifts, vive, quest2];

            headset.append(`<option value="0">Oculus Rift S</option>`);
            headset.append(`<option value="1">HTC Vive</option>`);
            headset.append(`<option value="2">Oculus Quest 2</option>`);

            const select = new Selectal("#_root_headset_headsetEmulationMode");
            headset = $("#_root_headset_headsetEmulationMode");

            headset.val(headsetMode.val());
            headset.change();

            headset.change((ev) => {
                for (const key in headsetOptions[headset.val()]) {
                    const target = $(headsetBase + key);
                    target.val(headsetOptions[headset.val()][key]);
                    alvrSettings.storeParam(target, true);
                }
                alvrSettings.storeSession("settings");
            });
        }

        function setVideoOptions() {
            let dropdown = $("#_root_video_resolutionDropdown");
            dropdown.after(alvrSettings.getHelpReset("resolutionDropdown", "_root_video", "100"));
            dropdown.parent().addClass("special");
            dropdown.unbind();

            const renderScale = $("#_root_video_renderResolution_scale");
            const targetScale = $("#_root_video_recommendedTargetResolution_scale");
            const renderScaleVariant = $("#_root_video_renderResolution_scale-choice-");
            const targetScaleVariant = $("#_root_video_recommendedTargetResolution_scale-choice-");

            video_scales.forEach((scale) => {
                dropdown.append(`<option value="${scale}"> ${scale}% </option>`);
            });

            const select = new Selectal("#_root_video_resolutionDropdown");
            dropdown = $("#_root_video_resolutionDropdown");

            let customRes = `<div style="display:inline;" id="customVideoScale"><b>${i18n.customVideoScale} </b></div>`;
            $("#_root_video_resolutionDropdown-selectal").after(customRes);
            customRes = $("#customVideoScale");
            customRes.hide();

            let update = false;

            const updateDropdown = function () {
                useScale = renderScaleVariant.prop("checked") && targetScaleVariant.prop("checked");
                sameScale = renderScale.val() == targetScale.val();
                if (useScale && sameScale) {
                    if (video_scales.indexOf(renderScale.val() * 100) != -1) {
                        dropdown.val(renderScale.val() * 100);
                        $("#_root_video_resolutionDropdown-selectal").show();
                        customRes.hide();
                    } else {
                        $("#_root_video_resolutionDropdown-selectal").hide();
                        customRes.show();
                    }
                } else {
                    $("#_root_video_resolutionDropdown-selectal").hide();
                    //always custom
                    customRes.show();
                }
                dropdown.change();
            };

            updateDropdown();

            $(
                "#_root_video_renderResolution_scale-choice-,#_root_video_recommendedTargetResolution_scale-choice-,#_root_video_renderResolution_scale,#_root_video_recommendedTargetResolution_scale"
            ).change((ev) => {
                if (update) {
                    return;
                }

                update = true;
                updateDropdown();
                update = false;
            });

            dropdown.change((ev) => {
                if (update) {
                    return;
                }

                update = true;

                const val = dropdown.val();
                renderScale.val(val / 100);
                targetScale.val(val / 100);

                alvrSettings.storeParam(renderScale, true);
                alvrSettings.storeParam(targetScale, true);

                //force scale mode
                renderScaleVariant.prop("checked", true);
                renderScaleVariant
                    .parent()
                    .parent()
                    .children()
                    .filter(".active")
                    .removeClass("active");
                alvrSettings.storeParam(renderScaleVariant, true);
                targetScaleVariant.prop("checked", true);
                targetScaleVariant
                    .parent()
                    .parent()
                    .children()
                    .filter(".active")
                    .removeClass("active");
                alvrSettings.storeParam(targetScaleVariant, true);
                alvrSettings.storeSession("settings");

                update = false;
            });
        }

        function setRefreshRate() {
            const el = $("#_root_video_displayRefreshRate");

            const preferredFps = $("#_root_video_preferredFps");

            const custom = i18n.customRefreshRate;

            const customButton = `<label id="displayRefreshRateCustomButton" class="btn btn-primary active">
            <input type="radio" name="displayRefreshRate"  autocomplete="off" value="custom" checked>
                ${custom}
            </label> `;

            function setRefreshRateRadio() {
                $("#displayRefreshRateCustomButton").remove();
                $("input:radio[name='displayRefreshRate']").parent().removeClass("active");

                switch (preferredFps.val()) {
                    case "120":
                    case "90":
                    case "80":
                    case "72":
                    case "60":
                        $(
                            "input:radio[name='displayRefreshRate'][value='" +
                                preferredFps.val() +
                                "']"
                        ).prop("checked", "true");
                        $(
                            "input:radio[name='displayRefreshRate'][value='" +
                                preferredFps.val() +
                                "']"
                        )
                            .parent()
                            .addClass("active");
                        break;

                    default:
                        console.log("custom refresh rate");
                        $("#displayRefreshRateButtons").append(customButton);

                        break;
                }
            }

            function setRefreshRateValue(val) {
                if (val !== "custom") {
                    preferredFps.val(val);
                }
                alvrSettings.storeParam(preferredFps);
                setRefreshRateRadio();
            }

            //move elements into better layout
            const text = el.parent().text().trim();
            el.parent().find("label").remove();

            const grp = `
                    <div class="card-title"> ${text}
                    ${alvrSettings.getHelpReset(
                        "displayRefreshRate",
                        "_root_video",
                        72,
                        (postFix = ""),
                        "displayRefreshRate",
                        "72 Hz"
                    )}
                    </div>
                    <div class="btn-group" data-toggle="buttons" id="displayRefreshRateButtons">
                        <label style="min-width:10%" class="btn btn-primary">
                            <input type="radio" name="displayRefreshRate"  autocomplete="off" value="60">
                            60 Hz
                        </label>
                        <label class="btn btn-primary">
                            <input type="radio" name="displayRefreshRate"  autocomplete="off" value="72">
                            72 Hz
                        </label>
                        <label class="btn btn-primary">
                            <input type="radio" name="displayRefreshRate"  autocomplete="off" value="80">
                            80 Hz
                        </label>
                        <label class="btn btn-primary">
                            <input type="radio" name="displayRefreshRate" autocomplete="off" value="90">
                            90 Hz
                        </label>
                        <label class="btn btn-primary">
                            <input type="radio" name="displayRefreshRate" autocomplete="off" value="120">
                            120 Hz
                        </label>

                    </div> `;

            el.after(grp);

            $(document).ready(() => {
                $("input:radio[name='displayRefreshRate']").on("change", () => {
                    setRefreshRateValue($("input:radio:checked[name='displayRefreshRate']").val());
                });
                preferredFps.on("change", () => {
                    setRefreshRateRadio();
                });

                $("#_root_video_displayRefreshRate").on("change", (ev) => {
                    setRefreshRateValue($("#_root_video_displayRefreshRate").val());
                });

                setRefreshRateRadio();
            });
        }

        function setupAudioDropdown(section, dropdownDevice, targetDevice, direction) {
            let el = $("#_root_audio_" + section + "_content_" + dropdownDevice);
            el.parent().addClass("special");
            el.unbind();

            el.append(`<option value="default"> Default </option>`);
            audio_devices[direction].forEach((deviceName) => {
                el.append(`<option value="${deviceName}"> ${deviceName} </option>`);
            });

            let currentSetting =
                alvrSettings.getSession().sessionSettings.audio[section].content[targetDevice];

            //select the current option in dropdown
            if (currentSetting.variant == "default") {
                el.val("default");
            } else if (currentSetting.variant == "name") {
                el.val(currentSetting.name);
            }

            const target = $("#_root_audio_" + section + "_content_" + targetDevice + "-choice-");

            let updating = false;
            //add listener to change
            el.change((ev) => {
                if (!updating) {
                    updating = true;

                    target.children().first().children().filter(".active").removeClass("active");

                    let selection = $(ev.target).val();
                    if (selection == "default") {
                        let targetVariant = $(
                            "#_root_audio_" +
                                section +
                                "_content_" +
                                targetDevice +
                                "_default-choice-"
                        );
                        targetVariant.prop("checked", true);
                        alvrSettings.storeParam(targetVariant);
                    } else {
                        let targetVariant = $(
                            "#_root_audio_" + section + "_content_" + targetDevice + "_name-choice-"
                        );
                        targetVariant.prop("checked", true);
                        alvrSettings.storeParam(targetVariant);

                        let targetName = $(
                            "#_root_audio_" + section + "_content_" + targetDevice + "_name"
                        );
                        targetName.val(selection);
                        alvrSettings.storeParam(targetName);
                    }

                    updating = false;
                }
            });

            target.change(() => {
                if (!updating) {
                    updating = true;

                    let currentSetting =
                        alvrSettings.getSession().sessionSettings.audio[section].content[
                            targetDevice
                        ];

                    if (currentSetting.variant == "default") {
                        el.val("default");
                    } else if (currentSetting.variant == "name") {
                        el.val(currentSetting.name);
                    }
                    el.change();

                    updating = false;
                }
            });
        }

        function setAudioDeviceList() {
            setupAudioDropdown("gameAudio", "deviceDropdown", "deviceId", "output");
            setupAudioDropdown("microphone", "inputDeviceDropdown", "inputDeviceId", "output");
            setupAudioDropdown("microphone", "outputDeviceDropdown", "outputDeviceId", "input");
        }

        function setTrackingSpeed() {
            const el = $("#_root_headset_controllers_content_trackingSpeed");

            const poseTimeOffset = $("#_root_headset_controllers_content_poseTimeOffset");
            const clientsidePrediction = $(
                "#_root_headset_controllers_content_clientsidePrediction"
            );
            const serversidePrediction = $(
                "#_root_headset_controllers_content_serversidePrediction"
            );

            const oculus = i18nWizard.oculusTracking;
            const steamvr = i18nWizard.steamvrTracking;
            const normal = i18nWizard.normalTracking;
            const medium = i18nWizard.mediumTracking;
            const fast = i18nWizard.fastTracking;
            const custom = i18n.customTracking;

            const customButton = `<label id="trackingSpeedCustomButton" class="btn btn-primary active">
            <input type="radio" name="trackingSpeed"  autocomplete="off" value="custom" checked>
                ${custom}
            </label> `;

            function setTrackingRadio() {
                $("#trackingSpeedCustomButton").remove();
                $("input:radio[name='trackingSpeed']").parent().removeClass("active");

                if (clientsidePrediction.is(":checked") && serversidePrediction.is(":checked")) {
                    $("#trackingSpeedButtons").append(customButton);
                } else if (clientsidePrediction.is(":checked")) {
                    $("input:radio[name='trackingSpeed'][value='oculus']").prop("checked", "true");
                    $("input:radio[name='trackingSpeed'][value='oculus']")
                        .parent()
                        .addClass("active");
                } else if (serversidePrediction.is(":checked")) {
                    $("input:radio[name='trackingSpeed'][value='steamvr']").prop("checked", "true");
                    $("input:radio[name='trackingSpeed'][value='steamvr']")
                        .parent()
                        .addClass("active");
                } else {
                    switch (poseTimeOffset.val()) {
                        case "-1":
                            $("input:radio[name='trackingSpeed'][value='fast']").prop(
                                "checked",
                                "true"
                            );
                            $("input:radio[name='trackingSpeed'][value='fast']")
                                .parent()
                                .addClass("active");
                            break;
                        case "-0.03":
                            $("input:radio[name='trackingSpeed'][value='medium']").prop(
                                "checked",
                                "true"
                            );
                            $("input:radio[name='trackingSpeed'][value='medium']")
                                .parent()
                                .addClass("active");
                            break;
                        case "0.01":
                            $("input:radio[name='trackingSpeed'][value='normal']").prop(
                                "checked",
                                "true"
                            );
                            $("input:radio[name='trackingSpeed'][value='normal']")
                                .parent()
                                .addClass("active");
                            break;
                        default:
                            console.log("custom tracking speed");
                            $("#trackingSpeedButtons").append(customButton);
                            break;
                    }
                }
            }

            function setTrackingValue(val) {
                // need to store parameters quickly, otherwise it seems to not apply properly
                switch (val) {
                    case "oculus":
                        clientsidePrediction.prop("checked", true);
                        break;
                    case "steamvr":
                    case "normal":
                    case "medium":
                    case "fast":
                        clientsidePrediction.prop("checked", false);
                        break;
                    default:
                        break;
                }
                alvrSettings.storeParam(clientsidePrediction);
                setTrackingRadio();
                switch (val) {
                    case "steamvr":
                        serversidePrediction.prop("checked", true);
                        break;
                    case "oculus":
                    case "normal":
                    case "medium":
                    case "fast":
                        serversidePrediction.prop("checked", false);
                        break;
                    default:
                        break;
                }
                alvrSettings.storeParam(serversidePrediction);
                setTrackingRadio();
                switch (val) {
                    case "normal":
                        poseTimeOffset.val("0.01");
                        break;
                    case "medium":
                        poseTimeOffset.val("-0.03");
                        break;
                    case "fast":
                        poseTimeOffset.val("-1");
                        break;
                    default:
                        break;
                }
                alvrSettings.storeParam(poseTimeOffset);
                setTrackingRadio();
            }

            //move elements into better layout
            const text = el.parent().text().trim();
            el.parent().find("label").remove();

            const grp = `<div class="card-title"> ${text}
                    ${alvrSettings.getHelpReset(
                        "trackingSpeed",
                        "_root_headset_controllers_content",
                        "steamvr",
                        (postFix = ""),
                        "trackingSpeed",
                        i18nWizard.steamvrTracking
                    )}
                        </div>
                        <div class="btn-group" data-toggle="buttons" id="trackingSpeedButtons">
                            <label style="min-width:10%" class="btn btn-primary">
                                <input type="radio" name="trackingSpeed"  autocomplete="off" value="oculus">
                                ${oculus}
                            </label>
                            <label style="min-width:10%" class="btn btn-primary">
                                <input type="radio" name="trackingSpeed"  autocomplete="off" value="steamvr">
                                ${steamvr}
                            </label>
                            <label style="min-width:10%" class="btn btn-primary">
                                <input type="radio" name="trackingSpeed"  autocomplete="off" value="normal">
                                ${normal}
                            </label>
                            <label class="btn btn-primary">
                                <input type="radio" name="trackingSpeed"  autocomplete="off" value="medium">
                                ${medium}
                            </label>
                            <label class="btn btn-primary">
                                <input type="radio" name="trackingSpeed" autocomplete="off" value="fast">
                               ${fast}
                            </label>
                        </div> `;

            el.after(grp);

            $(document).ready(() => {
                $("input:radio[name='trackingSpeed']").on("change", () => {
                    setTrackingValue($("input:radio:checked[name='trackingSpeed']").val());
                });
                poseTimeOffset.on("change", () => {
                    setTrackingRadio();
                });
                clientsidePrediction.on("change", () => {
                    setTrackingRadio();
                });
                serversidePrediction.on("change", () => {
                    setTrackingRadio();
                });

                $("#_root_headset_controllers_content_trackingSpeed").on("change", (ev) => {
                    setTrackingValue($("#_root_headset_controllers_content_trackingSpeed").val());
                });

                setTrackingRadio();
            });
        }

        function setTheme() {
            const themes = {
                classic: {
                    bootstrap: "css/bootstrap.min.css",
                    selectal: "js/lib/selectal.min.css",
                    style: "css/style.css",
                },
                darkly: {
                    bootstrap: "css/darkly/bootstrap.min.css",
                    selectal: "css/darkly/selectal.min.css",
                    style: "css/darkly/style.css",
                },
            };
            const bootstrap = $("#bootstrap");
            const selectal = $("#selectal");
            const style = $("#style");

            const themeSelector = $("form#_root_extra_theme-choice-").first();
            let themeColor = $("input[name='theme']:checked").val();

            if (themeColor == "systemDefault") {
                if (
                    window.matchMedia &&
                    window.matchMedia("(prefers-color-scheme: dark)").matches
                ) {
                    themeColor = "darkly";
                } else {
                    themeColor = "classic";
                }
            }

            window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", (e) => {
                themeColor = e.matches ? "darkly" : "classic";
                bootstrap.attr("href", themes[themeColor]["bootstrap"]);
                selectal.attr("href", themes[themeColor]["selectal"]);
                style.attr("href", themes[themeColor]["style"]);
            });

            bootstrap.attr("href", themes[themeColor]["bootstrap"]);
            selectal.attr("href", themes[themeColor]["selectal"]);
            style.attr("href", themes[themeColor]["style"]);

            themeSelector.on("change", function () {
                themeColor = $("input[name='theme']:checked", "#_root_extra_theme-choice-").val();
                if (themeColor == "systemDefault") {
                    if (
                        window.matchMedia &&
                        window.matchMedia("(prefers-color-scheme: dark)").matches
                    ) {
                        themeColor = "darkly";
                    } else {
                        themeColor = "classic";
                    }
                }

                if (bootstrap.attr("href") == themes[themeColor]["bootstrap"]) {
                    return;
                } else {
                    $("body").fadeOut("fast", function () {
                        console.log("changing theme to " + themeColor);
                        bootstrap.attr("href", themes[themeColor]["bootstrap"]);
                        selectal.attr("href", themes[themeColor]["selectal"]);
                        style.attr("href", themes[themeColor]["style"]);
                        $(this).fadeIn();
                    });
                }
            });
        }
    };
});
