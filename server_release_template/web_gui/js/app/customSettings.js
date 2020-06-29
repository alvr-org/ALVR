define([
    "i18n!app/nls/settings",
    "json!../../audio_devices",

], function (i18n, audio_devices) {
    return function (alvrSettings) {
        var self = this;
        const video_scales = [25, 50, 66, 75, 100, 125, 150, 200];

        self.setCustomSettings = function () {
            setDeviceList();
            setVideoOptions();
            setBitrateOptions();
            setSuppressFrameDrop();
        }

        function setSuppressFrameDrop() {
            const suppress = $("#_root_connection_suppressFrameDrop");
            const queue = $("#_root_connection_frameQueueSize");
            suppress.parent().addClass("special");
            suppress.unbind();

            suppress.parent().find(".helpReset").remove();
            suppress.after(alvrSettings.getHelpReset("suppressFrameDrop", "_root_connection", true));


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
                if (alvrSettings.isUpdating()  || updating ) {
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

        function setVideoOptions() {
            const el = $("#_root_video_resolutionDropdown");
            el.after(alvrSettings.getHelpReset("resolutionDropdown", "_root_video", "100"));
            el.parent().addClass("special");
            el.unbind();

            const targetWidth = $("#_root_video_renderResolution_absolute_width");
            const targetHeight = $("#_root_video_renderResolution_absolute_height");

            const scale = $("#_root_video_renderResolution_scale");

            const useScale = $("#_root_video_renderResolution_scale-choice-").prop("checked");

            video_scales.forEach(scale => {
                el.append(`<option value="${scale}"> ${scale}% </option>`);
            });
            el.append(`<option value="custom"> ${i18n.customVideoScale}</option>`);

            var absWidth;
            var absHeight;

            var updateDropdown = function () {
                if (useScale) {
                    if (video_scales.indexOf(scale.val() * 100) != -1) {
                        el.val(scale.val() * 100);
                    } else {
                        el.val("custom");
                    }
                } else if (alvrSettings.getSession().lastClients.length > 0) {

                    //TODO: always custom or try to determine scale?

                    absWidth = alvrSettings.getSession().lastClients[0].handshakePacket.renderWidth;
                    absHeight = alvrSettings.getSession().lastClients[0].handshakePacket.renderHeight;

                    var factor = targetWidth.val() / absWidth;

                    if (video_scales.indexOf(factor * 100) != -1) {
                        el.val(factor * 100);
                    } else {
                        el.val("custom");
                    }

                } else {
                    //always custom
                    el.val("custom");
                }
            }
            updateDropdown();

            $("#_root_video_renderResolution_absolute_width,#_root_video_renderResolution_absolute_height,#_root_video_renderResolution_scale").change((ev) => {
                updateDropdown();
            })


            el.change((ev) => {
                const val = $(ev.target).val();
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
                alvrSettings.storeParam( $("#_root_video_renderResolution_scale-choice-") , true);         

                alvrSettings.storeSession();
            });

        }

        function setBitrateOptions() {
            const bitrate = $("#_root_video_encodeBitrateMbs");
            const bufferSize = $("#_root_connection_clientRecvBufferSize");
            const throttleBitrate = $("#_root_connection_throttlingBitrateBits");

            bitrate.unbind();

            bitrate.change((ev) => {
                if (alvrSettings.isUpdating()) {
                    return;
                }

                alvrSettings.storeParam(bitrate, true);

                bufferSize.val(bitrate.val() * 2 * 1000);
                alvrSettings.storeParam(bufferSize, true);

                //set default reset value to value defined by bitrate
                var def = bufferSize.parent().find("i[default]");
                def.attr("default", bufferSize.val());

                //50% margin
                throttleBitrate.val(bitrate.val() * 1000000 * 3 / 2 + 2000000); //2mbit for audio
                alvrSettings.storeParam(throttleBitrate, true);

                def = throttleBitrate.parent().find("i[default]");
                def.attr("default", throttleBitrate.val());

                

                alvrSettings.storeSession();

            });

            //set default reset buffer size according to bitrate
            var def = bufferSize.parent().find("i[default]");
            def.attr("default", bitrate.val() * 2 * 1000);

            def = throttleBitrate.parent().find("i[default]");
            def.attr("default", bitrate.val() * 1000000 * 3 / 2 + 2000000);    //2mbit for audio
        }

        function setDeviceList() {
            const el = $("#_root_audio_gameAudio_content_deviceDropdown");
            el.parent().addClass("special")
            //el.unbind();

            const target = $("#_root_audio_gameAudio_content_device");
            let current = "";
            try {
                current = alvrSettings.getSession().settingsCache.audio.gameAudio.content.device;
            } catch (err) {
                console.error("Layout of settings changed, audio devices can not be added. Please report this bug!");
            }

            audio_devices.list.forEach(device => {
                let name = device[1];
                if (device[0] === audio_devices.default) {
                    name = "(default) " + device[1];
                    el.after(alvrSettings.getHelpReset("deviceDropdown", "_root_audio_gameAudio_content", device[0]));
                }
                el.append(`<option value="${device[0]}"> ${name}  </option>`)
            });

            //set default as current audio device if empty
            if (current.trim() === "") {
                target.val(audio_devices.default);
                target.change();
            }

            //move selected audio device to top of list
            var $el = $("#_root_audio_gameAudio_content_deviceDropdown").find("option[value='" + target.val() + "']").remove();
            $("#_root_audio_gameAudio_content_deviceDropdown").find('option:eq(0)').before($el);

            //select the current option in dropdown
            el.val(target.val());

            //add listener to change
            el.change((ev) => {
                target.val($(ev.target).val());
                target.change();
            })
        }
    }

});