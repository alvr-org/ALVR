define([
    "lib/lodash",
    "text!app/templates/wizard.html",
    "css!app/templates/wizard.css"
], function (_, wizardTemplate) {
    return function (alvrSettings) {


        this.showWizard = function () {
            var currentPage = 0;
            var compiledTemplate = _.template(wizardTemplate);
            var test = compiledTemplate();

            $("#setupWizard").remove();
            $("body").append(test);
            $(document).ready(() => {
                $('#setupWizard').modal({
                    backdrop: 'static',
                    keyboard: false
                });
                $("#installDriver").click(() => {
                    $.get("driver/register", undefined, (res) => {
                        if (res == -1) {
                            Lobibox.notify("error", {
                                size: "mini",
                                rounded: true,
                                delayIndicator: false,
                                sound: false,
                                msg: "Registering driver failed"
                            })
                        } else {
                            Lobibox.notify("success", {
                                size: "mini",
                                rounded: true,
                                delayIndicator: false,
                                sound: false,
                                msg: "Driver successfully registred"
                            })
                        }
                    })
                })

                $("#addFirewall").click(() => {
                    $.get("firewall-rules/add", undefined, (res) => {
                        if (res == -1) {
                            Lobibox.notify("error", {
                                size: "mini",
                                rounded: true,
                                delayIndicator: false,
                                sound: false,
                                msg: "Setting firewall rules failed"
                            })
                        } else {
                            Lobibox.notify("success", {
                                size: "mini",
                                rounded: true,
                                delayIndicator: false,
                                sound: false,
                                msg: "Firewall rules successfully set"
                            })
                        }
                    })
                })

                $(".poseOffsetButton").change((ev) => {
                    var target = $(ev.target);

                    //IDs depend on the schema!
                    switch (target.attr("value")) {
                        case "normal":
                            $("#root_Main_headset_controllers_content_poseTimeOffset").val("0.01")
                            break;
                        case "medium":
                            $("#root_Main_headset_controllers_content_poseTimeOffset").val("0")
                        case "fast":
                            $("#root_Main_headset_controllers_content_poseTimeOffset").val("-1")
                        default:
                            break;
                    }

                    console.log(target.attr("value"))
                })

                $(".performanceOptions").change((ev) => {
                    var target = $(ev.target);

                    //IDs depend on the schema!
                    switch (target.attr("value")) {
                        case "compatibility":
                            //TODO: add compat options
                            break;
                        case "performance":
                            //TODO: add performance options
                            break;

                        default:
                            break;
                    }

                    console.log(target.attr("value"))
                })

                $("#wizardNextButton").click(() => {

                    if (currentPage >= $("#wizardMain").children().length - 1) {
                        $('#setupWizard').modal('hide');
                        alvrSettings.disableWizard();
                        return;
                    }

                    if (currentPage >= $("#wizardMain").children().length - 2) {
                        $("#wizardNextButton").text("Close")
                    }


                    $($("#wizardMain").children().get(currentPage)).hide();
                    $($("#wizardMain").children().get(currentPage + 1)).show();

                    $("#wizardNextButton").blur();

                    currentPage += 1;
                })

            });

        }



    };
});