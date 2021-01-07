define([
    "lib/lodash",
    "text!app/templates/uploadPreset.html"
], function(_, uploadTemplate, i18n) {
    return new function() {

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
                    msg: "type error"
                })
            } else {
                const reader = new FileReader();

                reader.addEventListener('load', function() {

                    // filereader result (file content)
                    console.log(this.result);
                    const uploadedSession = JSON.parse(this.result);

                    // json parsed file
                    console.log(uploadedSession);

                    // reset the value of the input so the user can reload the same file.
                    document.getElementById("settingUpload").value = "";

                    Lobibox.notify("success", {
                        size: "mini",
                        rounded: true,
                        delayIndicator: false,
                        sound: false,
                        msg: "Success"
                    });
                });

                reader.readAsText(file);
            }
        });

        this.addUploadPreset = function(elementId) {

            var compiledTemplate = _.template(uploadTemplate);
            var template = compiledTemplate({ id: elementId, ...i18n });

            $("#" + elementId).prepend(template);
        }
    };
});