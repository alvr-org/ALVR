define([
    "lib/lodash",
    "text!app/templates/driver.html",
    "i18n!app/nls/driver",
    "css!app/templates/driver.css"
], function (_, driverTemplate, i18n) {
    return new function () {

        var self = this;

        this.fillDriverList = function (elementId) {

            var compiledTemplate = _.template(driverTemplate);
            var template = compiledTemplate({id:elementId});

            $("#" + elementId).empty();
            $("#" + elementId).append(template);
            $(document).ready(() => {
                $.get("driver/list", undefined, (res) => {
                    if (res == -1) {
                        Lobibox.notify("error", {
                            size: "mini",
                            rounded: true,
                            delayIndicator: false,
                            sound: false,
                            msg: i18n.driverFailed
                        })
                    } else {                       
                        updateHeader(res.length, elementId);

                        res.forEach(driver => {

                            $("#driverList_" + elementId + " table").append(`<tr>
                            <td>${driver}</td>
                            <td>
                                <button path="${driver}" type="button" class="btn btn-primary removeDriverButton">Remove</button>
                            </td></tr>`);
                        });

                        $(document).ready(() => {
                            $("#driverList_" + elementId + " * > .removeDriverButton").click((evt) => {
                                const path = $(".removeDriverButton").attr("path");

                                $.ajax({
                                    type: "POST",
                                    url: `driver/unregister`,
                                    contentType: "application/json;charset=UTF-8",
                                    data: JSON.stringify(path),
                                    processData: false,
                                    success: function (res) {
                                        if (res === "") {

                                            //not very good to have the ids here
                                            self.fillDriverList("driverListPlaceholder");
                                            self.fillDriverList("registeredDriversInst");

                                            Lobibox.notify("success", {
                                                size: "mini",
                                                rounded: true,
                                                delayIndicator: false,
                                                sound: false,
                                                msg: i18n.driverUnregisterSuccessful
                                            })
                                        }
                                    },
                                    error: function (res) {
                                        Lobibox.notify("error", {
                                            size: "mini",
                                            rounded: true,
                                            delayIndicator: false,
                                            sound: false,
                                            msg: i18n.driverUnregisterFailed
                                        })
                                    }
                                });


                            });
                        });


                    }
                })


            });

        }


        updateHeader = function(listSize, elementId) {
            if(listSize == 0) {
                $("#driverListHeader_" + elementId ).text(i18n.noDrivers);
                return;
            } else {
                $("#driverListHeader_" + elementId).text(i18n.registeredDrivers);
            }
        }
    };
});