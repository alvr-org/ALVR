define([
    "i18n!app/nls/settings",
    "lib/selectal",
    "json!../../audio_devices",
    "json!app/resources/HTCVive.json",
    "json!app/resources/OculusRift.json",
    "json!app/resources/OculusTouch.json",
    "json!app/resources/ValveIndex.json",
    "json!app/resources/HTCViveWand.json"


], function (i18n, select, audio_devices, vive, rifts, touch, index, vivewand) {
    return function (alvrSettings) {
        var self = this;
        const video_scales = [25, 50, 66, 75, 100, 125, 150, 200];

        self.setCustomSettings = function () {

            try {
                setDeviceList();
                setVideoOptions();
                setBitrateOptions();
                setSuppressFrameDrop();
                setDisableThrottling();
                setHeadsetEmulation();
                setControllerEmulation();
                setBufferOffset();
                setTheme();
            } catch (error) {
                Lobibox.notify("error", {
                    rounded: true,
                    delay : -1,
                    delayIndicator: false,
                    sound: false,
                    position: "bottom left",
                    iconSource: 'fontAwesome',
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
            const controllerOptions = [touch, touch, index, index, vivewand, vivewand];

            controller.append(`<option value="0">Oculus Rift S</option>`);
            controller.append(`<option value="1">Oculus Rift S (no handtracking pinch)</option>`);
            controller.append(`<option value="2">Valve Index</option>`);
            controller.append(`<option value="3">Valve Index (no handtracking pinch)</option>`);
            controller.append(`<option value="4">HTC Vive</option>`);
            controller.append(`<option value="5">HTC Vive (no handtracking pinch)</option>`);
            
            const select = new Selectal('#_root_headset_controllers_content_controllerMode');
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
            const headsetOptions = [rifts, vive];

            headset.append(`<option value="0">Oculus Rift S</option>`);
            headset.append(`<option value="1">HTC Vive</option>`);

            const select = new Selectal('#_root_headset_headsetEmulationMode');
            headset = $("#_root_headset_headsetEmulationMode");


            if ($(headsetBase + "modelNumber").val() == "Oculus Rift S") {
                headset.val(0);
            } else {
                headset.val(1);
            }

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

        function setSuppressFrameDrop() {
            const suppress = $("#_root_connection_suppressFrameDrop");
            const queue = $("#_root_connection_frameQueueSize");
            suppress.parent().addClass("special");
            suppress.unbind();

            suppress.parent().find(".helpReset").remove();
            suppress.after(alvrSettings.getHelpReset("suppressFrameDrop", "_root_connection", false));


            var updating = false;
            var updateCheckbox = function () {
                updating = true;
                if (queue.val() >= 5) {
                    suppress.prop("checked", true);
                } else {
                    suppress.prop("checked", false);
                }
                updating = false;
            }
            updateCheckbox();

            queue.change((ev) => {
                updateCheckbox();
            });

            suppress.change((ev) => {
                if (alvrSettings.isUpdating() || updating) {
                    return;
                }
                if (suppress.prop("checked")) {
                    queue.val(5);
                } else {
                    queue.val(1);
                }
                alvrSettings.storeParam(queue);
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


            const targetWidth = $("#_root_video_renderResolution_absolute_width");
            const targetHeight = $("#_root_video_renderResolution_absolute_height");

            const scale = $("#_root_video_renderResolution_scale");

            var useScale = $("#_root_video_renderResolution_scale-choice-").prop("checked");

            video_scales.forEach(scale => {
                dropdown.append(`<option value="${scale}"> ${scale}% </option>`);
            });

            //dropdown.append(`<option value="custom"> ${i18n.customVideoScale}</option>`);

            var absWidth;
            var absHeight;

            const select = new Selectal('#_root_video_resolutionDropdown');
            dropdown = $("#_root_video_resolutionDropdown");

            var customRes = `<div style="display:inline;" id="customVideoScale"><b>${i18n.customVideoScale} </b></div>`;
            $("#_root_video_resolutionDropdown-selectal").after(customRes);
            customRes = $("#customVideoScale");
            customRes.hide();

            var update = false;

            var updateDropdown = function () {
                useScale = $("#_root_video_renderResolution_scale-choice-").prop("checked");
                if (useScale) {
                    if (video_scales.indexOf(scale.val() * 100) != -1) {
                        dropdown.val(scale.val() * 100);
                        $("#_root_video_resolutionDropdown-selectal").show();
                        customRes.hide();
                    } else {
                        $("#_root_video_resolutionDropdown-selectal").hide()
                        customRes.show();
                    }
                } else if (alvrSettings.getSession().lastClients.length > 0) {

                    //TODO: always custom or try to determine scale?

                    absWidth = alvrSettings.getSession().lastClients[0].handshakePacket.renderWidth;
                    absHeight = alvrSettings.getSession().lastClients[0].handshakePacket.renderHeight;

                    var factor = targetWidth.val() / absWidth;

                    if (video_scales.indexOf(factor * 100) != -1) {
                        dropdown.val(factor * 100);
                        $("#_root_video_resolutionDropdown-selectal").show()
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


            $("#_root_video_renderResolution_absolute_width,#_root_video_renderResolution_absolute_height,#_root_video_renderResolution_scale").change((ev) => {
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
                scale.val(val / 100);

                alvrSettings.storeParam(scale, true);

                //TODO: set custom res?
                if (absWidth !== undefined && absHeight !== undefined) {
                    targetWidth.val(scale * absWidth);
                    targetHeight.val(scale * absHeight);

                    alvrSettings.storeParam(targetWidth, true);
                    alvrSettings.storeParam(targetHeight, true);
                }

                //force scale mode
                $("#_root_video_renderResolution_scale-choice-").prop("checked", true);
                alvrSettings.storeParam($("#_root_video_renderResolution_scale-choice-"), true);
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

        function setDeviceList() {

            if (audio_devices == null|| audio_devices.length == 0) {
                $("#_root_audio_gameAudio_content_deviceDropdown").hide();
                $("#_root_audio_microphone_content_deviceDropdown").hide();

                Lobibox.notify("warning", {
                    size: "mini",
                    rounded: true,
                    delayIndicator: false,
                    sound: false,
                    position: "bottom left",
                    iconSource: 'fontAwesome',
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
                    current = alvrSettings.getSession().settingsCache.audio.gameAudio.content.device;
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

                let select = new Selectal('#_root_audio_gameAudio_content_deviceDropdown');
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
                    current = alvrSettings.getSession().settingsCache.audio.microphone.content.device;
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

                let select = new Selectal('#_root_audio_microphone_content_deviceDropdown');
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



        function setTheme() {
            const themes = {
                "classic": {"bootstrap": "css/bootstrap.min.css", "selectal": "js/lib/selectal.min.css", "style": "css/style.css"},
                "darkly" : {"bootstrap": "css/darkly/bootstrap.min.css", "selectal": "css/darkly/selectal.min.css", "style": "css/darkly/style.css"}
            }
            var bootstrap = $("#bootstrap");
            var selectal = $("#selectal");
            var style = $("#style");
            
            var themeSelector = $("form#_root_extra_theme-choice-").first();
            var themeColor = $("input[name='theme']:checked").val();

            if (themeColor == "systemDefault") {
                if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
                    themeColor = "darkly";
                } else {
                    themeColor = "classic";
                }
            }

            window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', e => {
                themeColor = e.matches ? "darkly" : "classic";
                bootstrap.attr("href", themes[themeColor]["bootstrap"]);
                selectal.attr("href", themes[themeColor]["selectal"]);
                style.attr("href", themes[themeColor]["style"]);
            });

            bootstrap.attr("href", themes[themeColor]["bootstrap"]);
            selectal.attr("href", themes[themeColor]["selectal"]);
            style.attr("href", themes[themeColor]["style"]);

            themeSelector.on("change", function() {
                themeColor = $("input[name='theme']:checked", "#_root_extra_theme-choice-").val();
                if (themeColor == "systemDefault") {
                    if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
                        themeColor = "darkly";
                    } else {
                        themeColor = "classic";
                    }
                }

                if (bootstrap.attr("href") == themes[themeColor]["bootstrap"]) {
                    return;
                } else {
                    $("body").fadeOut('fast', function() {
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