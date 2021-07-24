define([
    "lib/lodash",
    "text!app/templates/driver.html",
    "i18n!app/nls/driver",
    "css!app/templates/driver.css",
], function (_, driverTemplate, i18n) {
    return new (function () {
        const self = this;

        $(document).on("click", ".registerAlvrDriver", () => {
            $.get("api/driver/register", undefined, (res) => {
                if (res != -1) {
                    Lobibox.notify("success", {
                        size: "mini",
                        rounded: true,
                        delayIndicator: false,
                        sound: false,
                        msg: i18n.registerAlvrDriverSuccess,
                    });
                }
            });
            self.fillDriverList("registeredDriversInst");
        });

        this.fillDriverList = function (elementId) {
            const compiledTemplate = _.template(driverTemplate);
            const template = compiledTemplate({ id: elementId, ...i18n });

            $("#" + elementId).empty();
            $("#" + elementId).append(template);
            const _elementId = elementId;
            // this call need const variable unless you want them overwriten by the next call.
            $(document).ready(() => {
                $.get("api/driver/list", undefined, (res) => {
                    if (res == -1) {
                        Lobibox.notify("error", {
                            size: "mini",
                            rounded: true,
                            delayIndicator: false,
                            sound: false,
                            msg: i18n.driverFailed,
                        });
                    } else {
                        updateHeader(res.length, _elementId);

                        $("#driverList_" + _elementId + " table").empty(); //clears table, helps with race condition
                        res.forEach((driver) => {
                            $("#driverList_" + _elementId + " table").append(`<tr>
                            <td>${driver}</td>
                            <td>
                                <button path="${driver}" type="button" class="btn btn-primary removeDriverButton">${i18n["removeDriver"]}</button>
                            </td></tr>`);
                        });

                        $(document).ready(() => {
                            $("#driverList_" + _elementId + " * > .removeDriverButton").click(
                                (evt) => {
                                    const path = $(evt.target).attr("path");

                                    $.ajax({
                                        type: "POST",
                                        url: "api/driver/unregister",
                                        contentType: "application/json;charset=UTF-8",
                                        data: JSON.stringify(path),
                                        processData: false,
                                        success: function (res) {
                                            if (res === "") {
                                                //not very good to have the ids here
                                                self.fillDriverList("registeredDriversInst");

                                                Lobibox.notify("success", {
                                                    size: "mini",
                                                    rounded: true,
                                                    delayIndicator: false,
                                                    sound: false,
                                                    msg: i18n.driverUnregisterSuccessful,
                                                });
                                            }
                                        },
                                        error: function (res) {
                                            Lobibox.notify("error", {
                                                size: "mini",
                                                rounded: true,
                                                delayIndicator: false,
                                                sound: false,
                                                msg: i18n.driverUnregisterFailed,
                                            });
                                        },
                                    });
                                }
                            );
                        });
                    }
                });
            });
        };

        updateHeader = function (listSize, elementId) {
            if (listSize == 0) {
                $("#driverListHeader_" + elementId).text(i18n.noDrivers);
                return;
            } else {
                $("#driverListHeader_" + elementId).text(i18n.registeredDrivers);
            }
        };
    })();
});
