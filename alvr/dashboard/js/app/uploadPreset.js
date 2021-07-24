define(["jquery", "lib/lodash", "text!app/templates/uploadPreset.html"], function (
    $,
    _,
    uploadTemplate,
    i18n
) {
    return new (function () {
        let _webClientId = "";

        // uploadButton trigger the input file
        $(document).on("click", "#settingUploadButton", () => {
            document.getElementById("settingUpload").click();
        });

        // input file after selected a file
        $(document).on("change", "#settingUpload", () => {
            const file = document.getElementById("settingUpload").files[0];
            if (file.type !== "application/json") {
                Lobibox.notify("error", {
                    size: "mini",
                    rounded: true,
                    delayIndicator: false,
                    sound: false,
                    msg: "type error",
                });
            } else {
                const reader = new FileReader();

                reader.addEventListener("load", function () {
                    // filereader result (file content)
                    const uploadedSession = this.result;

                    try {
                        const jsonSession = JSON.parse(uploadedSession);
                        $.ajax({
                            type: "POST",
                            url: "/api/session/store",
                            contentType: "application/json;charset=UTF-8",
                            data: JSON.stringify({
                                updateType: "settings",
                                webClientId: _webClientId,
                                session: jsonSession,
                            }),
                            processData: false,
                            success: function (res) {
                                if (res === "") {
                                    Lobibox.notify("success", {
                                        size: "mini",
                                        rounded: true,
                                        delayIndicator: false,
                                        sound: false,
                                        msg: "Success",
                                    });
                                } else {
                                    Lobibox.notify("error", {
                                        size: "mini",
                                        rounded: true,
                                        delayIndicator: false,
                                        sound: false,
                                        title: "Error while storing the settings",
                                        msg: res,
                                    });
                                    console.log("FAILED", res);
                                }
                            },
                            error: function (res) {
                                Lobibox.notify("error", {
                                    size: "mini",
                                    rounded: true,
                                    delayIndicator: false,
                                    sound: false,
                                    title: "Error while storing the settings",
                                    msg: res,
                                });
                                console.log("FAILED", res);
                            },
                        });
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
                        console.log("JSON error", error);
                    }

                    // reset the value of the input so the user can reload the same file.
                    document.getElementById("settingUpload").value = "";
                });

                reader.readAsText(file);
            }
        });

        this.addUploadPreset = function (elementId, webClientId) {
            _webClientId = webClientId;
            const compiledTemplate = _.template(uploadTemplate);
            const template = compiledTemplate({ id: elementId, ...i18n });

            $("#" + elementId).prepend(template);
        };
    })();
});
