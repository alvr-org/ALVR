define([
    "i18n!app/nls/settings",
    "i18n!app/nls/wizard",
    "lib/selectal",
    "json!../../audio-devices",
    "json!app/resources/HTCVive.json",
    "json!app/resources/OculusRift.json",
    "json!app/resources/Quest2.json",
    "json!app/resources/OculusTouch.json",
    "json!app/resources/ValveIndex.json",
    "json!app/resources/HTCViveWand.json",
    "json!app/resources/Quest2Touch.json",

], function (i18n,i18nWizard, select, audio_devices, vive, rifts, quest2, touch, index, vivewand, q2touch) {
    return function (alvrSettings) {
        var self = this;
        const video_scales = [25, 50, 66, 75, 100, 125, 150, 200];

        self.setCustomSettings = function () {

            try {
                setDeviceList();
                setVideoOptions();
                setBitrateOptions();
                setRefreshRate();
                setDisableThrottling();
                setHeadsetEmulation();
                setControllerEmulation();
                setTrackingSpeed();
                setBufferOffset();
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

        }

        function setBufferOffset() {
            const bufferOffset = $("#_root_connection_bufferOffset");
            bufferOffset.unbind();
            bufferOffset.parent().addClass("special");

            //readd the slider input listener to update the current value
            bufferOffset.on("input", (el) => {
                $("#_root_connection_bufferOffset_label").text("[" + el.target.value + "]")
            });

            bufferOffset.prop("min", "-100");
            bufferOffset.prop("max", "100");
            bufferOffset.prop("step", "1");

            const bitrate = $("#_root_video_encodeBitrateMbs");
            const bufferSize = $("#_root_connection_clientRecvBufferSize");

            bufferOffset.val((bufferSize.val() / 1000) - bitrate.val() * 2);
            $("#_root_connection_bufferOffset_label").text("[" + bufferOffset.val() + "]")

            bufferOffset.change((ev) => {
                bufferSize.val(Math.max(bitrate.val() * 2 * 1000 + bufferOffset.val() * 1000, 0));

                console.log("buffer size now", bufferSize.val())
                alvrSettings.storeParam(bufferSize);

                //set default reset value to value defined by bitrate
                var def = bufferSize.parent().find("i[default]");
                def.attr("default", bufferSize.val());
            });

        }

        function setControllerEmulation() {
            var controller = $("#_root_headset_controllers_content_controllerMode");
            controller.unbind();
            controller.after(alvrSettings.getHelpReset("controllerMode", "_root_headset_controllers_content", 0));
            controller.parent().addClass("special");

            const controllerBase = "#_root_headset_controllers_content_";
            const controllerMode = $(controllerBase + "modeIdx")
            const controllerOptions = [touch, touch, index, index, vivewand, vivewand, q2touch, q2touch];

            controller.append(`<option value="0">Oculus Rift S</option>`);
            controller.append(`<option value="1">Oculus Rift S (no handtracking pinch)</option>`);
            controller.append(`<option value="2">Valve Index</option>`);
            controller.append(`<option value="3">Valve Index (no handtracking pinch)</option>`);
            controller.append(`<option value="4">HTC Vive</option>`);
            controller.append(`<option value="5">HTC Vive (no handtracking pinch)</option>`);
            controller.append(`<option value="6">Oculus Quest 2</option>`);
            controller.append(`<option value="7">Oculus Quest 2 (no handtracking pinch)</option>`);

            const select = new Selectal("#_root_headset_controllers_content_controllerMode");
            controller = $("#_root_headset_controllers_content_controllerMode");

            controller.val(controllerMode.val());
            controller.change();

            controller.change((ev) => {
                for (var key in controllerOptions[controller.val()]) {
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
            var headset = $("#_root_headset_headsetEmulationMode");
            headset.unbind();
            headset.after(alvrSettings.getHelpReset("headsetEmulationMode", "_root_headset", 0));
            headset.parent().addClass("special");

            const headsetBase = "#_root_headset_";
            const headsetMode = $(headsetBase + "modeIdx")
            const headsetOptions = [rifts, vive, quest2];

            headset.append(`<option value="0">Oculus Rift S</option>`);
            headset.append(`<option value="1">HTC Vive</option>`);
            headset.append(`<option value="2">Oculus Quest 2</option>`);

            const select = new Selectal("#_root_headset_headsetEmulationMode");
            headset = $("#_root_headset_headsetEmulationMode");


            headset.val(headsetMode.val());
            headset.change();

            headset.change((ev) => {
                for (var key in headsetOptions[headset.val()]) {
                    const target = $(headsetBase + key);
                    target.val(headsetOptions[headset.val()][key]);
                    alvrSettings.storeParam(target, true);
                }
                alvrSettings.storeSession("settings");
            });
        }

        function setDisableThrottling() {
            const disableThrottling = $("#_root_connection_disableThrottling");
            const throttleBitrate = $("#_root_connection_throttlingBitrateBits");
            const bitrate = $("#_root_video_encodeBitrateMbs");

            disableThrottling.parent().addClass("special");
            disableThrottling.unbind();

            disableThrottling.parent().find(".helpReset").remove();
            disableThrottling.after(alvrSettings.getHelpReset("disableThrottling", "_root_connection", false));

            var updating = false;
            var updateCheckbox = function () {
                updating = true;
                if (throttleBitrate.val() == 0) {
                    disableThrottling.prop("checked", true);
                } else {
                    disableThrottling.prop("checked", false);
                }
                updating = false;
            }
            updateCheckbox();

            throttleBitrate.change((ev) => {
                updateCheckbox();
            });


            disableThrottling.change((ev) => {
                if (alvrSettings.isUpdating() || updating) {
                    return;
                }
                if (disableThrottling.prop("checked")) {
                    throttleBitrate.val(0);
                } else {
                    throttleBitrate.val(bitrate.val() * 1000000 * 3 / 2 + 2000000); //2mbit for audio
                }
                alvrSettings.storeParam(throttleBitrate);
            });
        }

        function setVideoOptions() {
            var dropdown = $("#_root_video_resolutionDropdown");
            dropdown.after(alvrSettings.getHelpReset("resolutionDropdown", "_root_video", "100"));
            dropdown.parent().addClass("special");
            dropdown.unbind();

            const renderScale = $("#_root_video_renderResolution_scale");
            const targetScale = $("#_root_video_recommendedTargetResolution_scale");
            const renderScaleVariant = $("#_root_video_renderResolution_scale-choice-");
            const targetScaleVariant = $("#_root_video_recommendedTargetResolution_scale-choice-");

            video_scales.forEach(scale => {
                dropdown.append(`<option value="${scale}"> ${scale}% </option>`);
            });

            const select = new Selectal("#_root_video_resolutionDropdown");
            dropdown = $("#_root_video_resolutionDropdown");

            var customRes = `<div style="display:inline;" id="customVideoScale"><b>${i18n.customVideoScale} </b></div>`;
            $("#_root_video_resolutionDropdown-selectal").after(customRes);
            customRes = $("#customVideoScale");
            customRes.hide();

            var update = false;

            var updateDropdown = function () {
                useScale = renderScaleVariant.prop("checked") && targetScaleVariant.prop("checked");
                sameScale = renderScale.val() == targetScale.val();
                if (useScale && sameScale) {
                    if (video_scales.indexOf(renderScale.val() * 100) != -1) {
                        dropdown.val(renderScale.val() * 100);
                        $("#_root_video_resolutionDropdown-selectal").show();
                        customRes.hide();
                    } else {
                        $("#_root_video_resolutionDropdown-selectal").hide()
                        customRes.show();
                    }
                } else {
                    $("#_root_video_resolutionDropdown-selectal").hide()
                    //always custom
                    customRes.show();
                }
                dropdown.change();
            }

            updateDropdown();


            $("#_root_video_renderResolution_scale-choice-,#_root_video_recommendedTargetResolution_scale-choice-,#_root_video_renderResolution_scale,#_root_video_recommendedTargetResolution_scale").change((ev) => {
                if (update) {
                    return;
                }

                update = true;
                updateDropdown();
                update = false;
            })


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
                renderScaleVariant.parent().parent().children().filter(".active").removeClass("active")
                alvrSettings.storeParam(renderScaleVariant, true);
                targetScaleVariant.prop("checked", true);
                targetScaleVariant.parent().parent().children().filter(".active").removeClass("active")
                alvrSettings.storeParam(targetScaleVariant, true);
                alvrSettings.storeSession("settings");

                update = false;
            });

        }

        function setBitrateOptions() {
            const bitrate = $("#_root_video_encodeBitrateMbs");
            const bufferOffset = $("#_root_connection_bufferOffset");
            const bufferSize = $("#_root_connection_clientRecvBufferSize");
            const throttleBitrate = $("#_root_connection_throttlingBitrateBits");
            const disableThrottling = $("#_root_connection_disableThrottling");

            bitrate.unbind();

            bitrate.change((ev) => {
                if (alvrSettings.isUpdating()) {
                    return;
                }

                alvrSettings.storeParam(bitrate, true);

                bufferSize.val(Math.max(bitrate.val() * 2 * 1000 + bufferOffset.val() * 1000, 0));
                alvrSettings.storeParam(bufferSize, true);

                //set default reset value to value defined by bitrate
                var def = bufferSize.parent().find("i[default]");
                def.attr("default", bufferSize.val());

                //50% margin
                if (disableThrottling.prop("checked")) {
                    throttleBitrate.val(0);
                } else {
                    throttleBitrate.val(bitrate.val() * 1000000 * 3 / 2 + 2000000); //2mbit for audio
                }

                alvrSettings.storeParam(throttleBitrate, true);

                def = throttleBitrate.parent().find("i[default]");
                def.attr("default", throttleBitrate.val());



                alvrSettings.storeSession("settings");

            });

            //set default reset buffer size according to bitrate
            var def = bufferSize.parent().find("i[default]");
            def.attr("default", bitrate.val() * 2 * 1000);

            def = throttleBitrate.parent().find("i[default]");
            def.attr("default", bitrate.val() * 1000000 * 3 / 2 + 2000000);    //2mbit for audio
        }

        function setRefreshRate() {
            const el = $("#_root_video_displayRefreshRate");

            const preferredFps = $("#_root_video_preferredFps");

            const custom = i18n.customRefreshRate

            const customButton = `<label id="displayRefreshRateCustomButton" class="btn btn-primary active">
            <input  type="radio" name="displayRefreshRate"  autocomplete="off" value="custom" checked>
                ${custom}
            </label> `;

            function setRefreshRateRadio() {             

                $("#displayRefreshRateCustomButton").remove();
                $("input:radio[name='displayRefreshRate']").parent().removeClass("active");          

                switch ( preferredFps.val()) {
                    case "90":
                    case "80":
                    case "72":
                    case "60":
                        $("input:radio[name='displayRefreshRate'][value='" + preferredFps.val() + "']").prop("checked", "true");
                        $("input:radio[name='displayRefreshRate'][value='" + preferredFps.val() + "']").parent().addClass("active");
                        break;

                    default:
                        console.log("custom refresh rate")
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
            const  text = el.parent().text().trim();
            el.parent().find("label").remove();

            const grp = `
                    <div class="card-title"> ${text}
                    ${alvrSettings.getHelpReset("displayRefreshRate", "_root_video", 72,  postFix = "", "displayRefreshRate", "72 Hz")}
                    </div>
                    <div class="btn-group" data-toggle="buttons" id="displayRefreshRateButtons">
                        <label style="min-width:10%" class="btn btn-primary">
                            <input  type="radio" name="displayRefreshRate"  autocomplete="off" value="60">
                            60 Hz
                        </label>
                        <label class="btn btn-primary">
                            <input  type="radio" name="displayRefreshRate"  autocomplete="off" value="72">
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
                                                  
                    </div> `

            el.after(grp);


            $(document).ready(() => {
                $("input:radio[name='displayRefreshRate']").on("change", () => {
                    setRefreshRateValue($("input:radio:checked[name='displayRefreshRate']").val());   
                });
                preferredFps.on("change", () => {                   
                    setRefreshRateRadio();
                });   
                
                $("#_root_video_displayRefreshRate").on("change", (ev) => {
                    setRefreshRateValue( $("#_root_video_displayRefreshRate").val());  
                });

                setRefreshRateRadio();
            });
        }

        function setDeviceList() {

            if (audio_devices == null || audio_devices.length == 0) {
                $("#_root_audio_gameAudio_content_deviceDropdown").hide();
                $("#_root_audio_microphone_content_deviceDropdown").hide();

                Lobibox.notify("warning", {
                    size: "mini",
                    rounded: true,
                    delayIndicator: false,
                    sound: false,
                    position: "bottom left",
                    iconSource: "fontAwesome",
                    msg: i18n.audioDeviceError,
                    closable: true,
                    messageHeight: 250,
                });


                return;
            }

            // Game audio
            {
                let el = $("#_root_audio_gameAudio_content_deviceDropdown");
                el.parent().addClass("special")
                el.unbind();

                let target = $("#_root_audio_gameAudio_content_device");

                let current = "";
                try {
                    current = alvrSettings.getSession().sessionSettings.audio.gameAudio.content.device;
                } catch (err) {
                    console.error("Layout of settings changed, audio devices can not be added. Please report this bug!");
                }

                audio_devices.list.forEach(device => {
                    let name = device[1];
                    if (device[0] === audio_devices.default_game_audio) {
                        name = "(default) " + device[1];
                        el.after(alvrSettings.getHelpReset("deviceDropdown", "_root_audio_gameAudio_content", device[0]));

                        const deviceReset = $("#_root_audio_gameAudio_content_device").parent().find(".helpReset .paramReset");
                        deviceReset.attr("default", device[0])
                    }
                    el.append(`<option value="${device[0]}"> ${name}  </option>`)
                });

                if (audio_devices.default_game_audio === null && audio_devices.list.length != 0) {
                    el.after(alvrSettings.getHelpReset("deviceDropdown", "_root_audio_gameAudio_content", audio_devices.list[0][0]));

                    const deviceReset = $("#_root_audio_gameAudio_content_device").parent().find(".helpReset .paramReset");
                    deviceReset.attr("default", audio_devices.list[0][0])
                }

                //set default as current audio device if empty
                if (current.trim() === "") {
                    target.val(audio_devices.default_game_audio);
                    target.change();
                    alvrSettings.storeParam(target);
                }


                //move selected audio device to top of list
                let $el = $("#_root_audio_gameAudio_content_deviceDropdown").find("option[value='" + target.val() + "']").remove();
                $("#_root_audio_gameAudio_content_deviceDropdown").prepend($el);

                let select = new Selectal("#_root_audio_gameAudio_content_deviceDropdown");
                el = $("#_root_audio_gameAudio_content_deviceDropdown");

                //select the current option in dropdown
                el.val(target.val());


                let updating = false;
                //add listener to change
                el.change((ev) => {
                    if (!updating) {
                        updating = true;
                        target.val($(ev.target).val());
                        target.change();
                        updating = false;
                    }
                })

                target.change(() => {
                    if (!updating) {
                        updating = true;
                        el.val(target.val());
                        el.change();
                        updating = false;
                    }
                })
            }

            // Microphone
            {
                let el = $("#_root_audio_microphone_content_deviceDropdown");
                el.parent().addClass("special")
                el.unbind();

                let target = $("#_root_audio_microphone_content_device");

                let current = "";
                try {
                    current = alvrSettings.getSession().sessionSettings.audio.microphone.content.device;
                } catch (err) {
                    console.error("Layout of settings changed, audio devices can not be added. Please report this bug!");
                }

                audio_devices.list.forEach(device => {
                    let label = device[1];
                    if (device[0] === audio_devices.default_microphone) {
                        label = "(default) " + device[1];
                        el.after(alvrSettings.getHelpReset("deviceDropdown", "_root_audio_microphone_content", device[0]));

                        const deviceReset = $("#_root_audio_microphone_content_device").parent().find(".helpReset .paramReset");
                        deviceReset.attr("default", device[0])
                    }
                    el.append(`<option value="${device[0]}"> ${label}  </option>`)
                });

                if (audio_devices.default_microphone === null && audio_devices.list.length != 0) {
                    el.after(alvrSettings.getHelpReset("deviceDropdown", "_root_audio_microphone_content", audio_devices.list[0][0]));

                    const deviceReset = $("#_root_audio_microphone_content_device").parent().find(".helpReset .paramReset");
                    deviceReset.attr("default", audio_devices.list[0][0])
                }

                //set default as current audio device if empty
                if (current.trim() === "") {
                    target.val(audio_devices.default_microphone);
                    target.change();
                    alvrSettings.storeParam(target);
                }


                //move selected audio device to top of list
                let $el = $("#_root_audio_microphone_content_deviceDropdown").find("option[value='" + target.val() + "']").remove();
                $("#_root_audio_microphone_content_deviceDropdown").prepend($el);

                let select = new Selectal("#_root_audio_microphone_content_deviceDropdown");
                el = $("#_root_audio_microphone_content_deviceDropdown");

                //select the current option in dropdown
                el.val(target.val());


                let updating = false;
                //add listener to change
                el.change((ev) => {
                    if (!updating) {
                        updating = true;
                        target.val($(ev.target).val());
                        target.change();
                        updating = false;
                    }
                })

                target.change(() => {
                    if (!updating) {
                        updating = true;
                        el.val(target.val());
                        el.change();
                        updating = false;
                    }
                })
            }
        }

        function setTrackingSpeed() {
            const el = $("#_root_headset_controllers_content_trackingSpeed");

            const poseTimeOffset = $("#_root_headset_controllers_content_poseTimeOffset");
            const clientsidePrediction = $("#_root_headset_controllers_content_clientsidePrediction");

            const oculus = i18nWizard.oculusTracking;
            const normal = i18nWizard.normalTracking;
            const medium = i18nWizard.mediumTracking;
            const fast = i18nWizard.fastTracking;
            const custom = i18n.customTracking

            const customButton = `<label id="trackingSpeedCustomButton" class="btn btn-primary active">
            <input  type="radio" name="trackingSpeed"  autocomplete="off" value="custom" checked>
                ${custom}
            </label> `;

            function setTrackingRadio() {

                $("#trackingSpeedCustomButton").remove();
                $("input:radio[name='trackingSpeed']").parent().removeClass("active");

                if (clientsidePrediction.is(":checked")) {
                    $("input:radio[name='trackingSpeed'][value='oculus']").prop("checked", "true");
                    $("input:radio[name='trackingSpeed'][value='oculus']").parent().addClass("active");
                }
                else {
                    switch (poseTimeOffset.val()) {
                        case "-1":
                            $("input:radio[name='trackingSpeed'][value='fast']").prop("checked", "true");
                            $("input:radio[name='trackingSpeed'][value='fast']").parent().addClass("active");
                            break;
                        case "-0.03":
                            $("input:radio[name='trackingSpeed'][value='medium']").prop("checked", "true");
                            $("input:radio[name='trackingSpeed'][value='medium']").parent().addClass("active");
                            break;
                        case "0.01":
                            $("input:radio[name='trackingSpeed'][value='normal']").prop("checked", "true");
                            $("input:radio[name='trackingSpeed'][value='normal']").parent().addClass("active");
                            break;
                        default:
                            console.log("custom tracking speed")
                            $("#trackingSpeedButtons").append(customButton);
                            break;
                    }
                }
            }

            function setTrackingValue(val) {
                switch (val) {
                    case "oculus":
                        clientsidePrediction.prop("checked", true);
                        break;
                    case "normal":
                        clientsidePrediction.prop("checked", false);
                        poseTimeOffset.val("0.01");
                        break;
                    case "medium":
                        clientsidePrediction.prop("checked", false);
                        poseTimeOffset.val("-0.03");
                        break;
                    case "fast":
                        clientsidePrediction.prop("checked", false);
                        poseTimeOffset.val("-1");
                        break;
                    default:
                        break;
                }
                alvrSettings.storeParam(poseTimeOffset);
                alvrSettings.storeParam(clientsidePrediction);
                setTrackingRadio();
            }

            //move elements into better layout
            const text = el.parent().text().trim();
            el.parent().find("label").remove();

            const grp = `<div class="card-title"> ${text}
                    ${alvrSettings.getHelpReset("trackingSpeed", "_root_headset_controllers_content", "normal", postFix = "", "trackingSpeed", i18nWizard.normalTracking)}
                        </div>
            <div class="btn-group" data-toggle="buttons" id="trackingSpeedButtons">
                            <label style="min-width:10%" class="btn btn-primary">
                                <input  type="radio" name="trackingSpeed"  autocomplete="off" value="oculus">
                                ${oculus}
                            </label>
                            <label style="min-width:10%" class="btn btn-primary">
                                <input  type="radio" name="trackingSpeed"  autocomplete="off" value="normal">
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
                                                  
                    </div> `

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

                $("#_root_headset_controllers_content_trackingSpeed").on("change", (ev) => {
                    setTrackingValue($("#_root_headset_controllers_content_trackingSpeed").val());
                });

                setTrackingRadio();
            });


        }


        function setTheme() {
            const themes = {
                "classic": { "bootstrap": "css/bootstrap.min.css", "selectal": "js/lib/selectal.min.css", "style": "css/style.css" },
                "darkly": { "bootstrap": "css/darkly/bootstrap.min.css", "selectal": "css/darkly/selectal.min.css", "style": "css/darkly/style.css" }
            }
            var bootstrap = $("#bootstrap");
            var selectal = $("#selectal");
            var style = $("#style");

            var themeSelector = $("form#_root_extra_theme-choice-").first();
            var themeColor = $("input[name='theme']:checked").val();

            if (themeColor == "systemDefault") {
                if (window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches) {
                    themeColor = "darkly";
                } else {
                    themeColor = "classic";
                }
            }

            window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", e => {
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
                    if (window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches) {
                        themeColor = "darkly";
                    } else {
                        themeColor = "classic";
                    }
                }

                if (bootstrap.attr("href") == themes[themeColor]["bootstrap"]) {
                    return;
                } else {
                    $("body").fadeOut("fast", function () {
                        console.log("changing theme to " + themeColor)
                        bootstrap.attr("href", themes[themeColor]["bootstrap"]);
                        selectal.attr("href", themes[themeColor]["selectal"]);
                        style.attr("href", themes[themeColor]["style"]);
                        $(this).fadeIn();
                    });

                }

            });

        }

    }

});