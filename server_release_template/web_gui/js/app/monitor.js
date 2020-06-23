define([
    "text!app/templates/addClientModal.html",
    "text!app/templates/monitor.html",
    "lib/lodash",
    "i18n!app/nls/monitor",
    "i18n!app/nls/notifications",
    "css!app/templates/monitor.css"

], function (addClientModalTemplate, monitorTemplate, _, i18n, i18nNotifications) {
    return function (alvrSettings) {

        var notificationLevels = [];

        function logInit() {
            var url = window.location.href
            var arr = url.split("/");


            const log_listener = new WebSocket("ws://" + arr[2] + "/log");

            log_listener.onopen = (ev) => {
                console.log("Log listener started")
            }

            log_listener.onerror = (ev) => {
                console.log("Log error", ev)
            }

            log_listener.onclose = (ev) => {
                console.log("Log closed", ev)
            }

            log_listener.addEventListener('message', function (e) { addLogLine(e.data) });

            $("#_root_extra_notificationLevel-choice-").change((ev) => {
                initNotificationLevel();
            });


        }

        function init() {
            var compiledTemplate = _.template(monitorTemplate);
            var template = compiledTemplate(i18n);

            compiledTemplate = _.template(addClientModalTemplate);
            var template2 = compiledTemplate(i18n);

            $("#monitor").append(template);

            $(document).ready(() => {
                logInit();
                initNotificationLevel();

                //DEBUG
                addNewClient("Oculus Quest", "192.168.1.223")
                addNewClient("Oculus Quest", "192.168.1.223")
                addNewClient("Oculus Quest", "192.168.1.190")
                ///

                $("#showAddClientModal").click(() => {
                    $("#addClientModal").remove();
                    $("body").append(template2);
                    $(document).ready(() => {
                        $('#addClientModal').modal({
                            backdrop: 'static',
                            keyboard: false
                        });
                        $("#clientAddButton").click(() => {
                            //TODO: input validation
                            const type = $("#clientTypeSelect").val();
                            const ip = $("#clientIP").val();
                            addTrustedClient(type, ip);
                            $('#addClientModal').modal('hide');
                            $('#addClientModal').remove();
                        })
                    });
                })

            });
        }

        function initNotificationLevel() {
            var level = $("input[name='notificationLevel']:checked").val();

            switch (level) {
                case "error":
                    notificationLevels = ["[ERROR]"];
                    break;
                case "warning":
                    notificationLevels = ["[ERROR]", "[WARN]"];
                    break;

                case "info":
                    notificationLevels = ["[ERROR]", "[WARN]", "[INFO]"];
                    break;

                case "debug":
                    notificationLevels = ["[ERROR]", "[WARN]", "[INFO]", "[DEBUG]"];
                    break;

                default:
                    notificationLevels = [];
            }

            console.log("Notification levels are now: ", notificationLevels);

        }

        function addNewClient(type, ip) {
            const id = ip.replace(/\./g, '');

            if ($("#newClient_" + id).length > 0) {
                console.warn("Client already in new list:", type, ip);
                return;
            }

            if ($("#trustedClient_" + id).length > 0) {
                console.warn("Client already in trusted list:", type, ip);
                return;
            }

            var client = `<div class="card client" type="${type}" ip="${ip}" id="newClient_${id}">
                        ${type} ${ip} <button type="button" class="btn btn-primary">trust</button>
                        </div>`

            $("#newClientsDiv").append(client);
            $(document).ready(() => {
                $("#newClient_" + id + " button").click(() => {
                    $("#newClient_" + id).remove();
                    addTrustedClient(type, ip);

                })
            });
        }

        function addTrustedClient(type, ip) {
            const id = ip.replace(/\./g, '');

            if ($("#newClient_" + id).length > 0) {
                console.warn("Client already in new list:", type, ip);
                return;
            }

            if ($("#trustedClient_" + id).length > 0) {
                console.warn("Client already in trusted list:", type, ip);
                return;
            }

            var client = `<div class="card client" type="${type}" ip="${ip}" id="trustedClient_${id}">
                        ${type} ${ip} <button type="button" class="btn btn-primary">remove</button>
                        </div>`

            $("#trustedClientsDiv").append(client);
            $(document).ready(() => {
                $("#trustedClient_" + id + " button").click(() => {
                    $("#trustedClient_" + id).remove();
                })
            });
        }

        function addLogLine(line) {
            var idObject = undefined;

            //find parts of log line
            var split = line.split(" ");
            if (split[2].startsWith("#")) {
                var index1 = line.indexOf("#")
                var index2 = line.indexOf("#", index1 + 1)
                idObject = line.substring(index1 + 1, index2);

                line = line.substring(index2 + 1, line.length);
            } else {
                line = line.replace(split[0] + " " + split[1], "");
            }

            const skipWithoutId = $("#_root_extra_excludeNotificationsWithoutId").prop("checked");

            if (idObject !== undefined) {
                idObject = JSON.parse(idObject);
            }

            if (notificationLevels.includes(split[1].trim())) {
                if (!(skipWithoutId && idObject === undefined)) {
                    Lobibox.notify(getNotificationType(split[1]), {
                        size: "mini",
                        rounded: true,
                        delayIndicator: false,
                        sound: false,
                        title: getI18nNotification(idObject, line, split[1]).title,
                        msg: getI18nNotification(idObject, line, split[1]).msg
                    })
                }
            }

            var row = `<tr><td>${split[0]}</td><td>${split[1]}</td><td>${line.trim()}</td></tr>`;
            $("#loggingTable").append(row);
            if ($("#loggingTable").children().length > 500) {
                $("#loggingTable tr").first().remove();
            }
        }

        function getI18nNotification(idObject, line, level) {
            if (idObject === undefined) {
                return { "title": level, "msg": line };
            } else {
                //TODO: line could contain additional info for the msg
                return { "title": i18nNotifications[idObject.id + ".title"], "msg": i18nNotifications[idObject.id + ".msg"] };
            }
        }

        function getNotificationType(logSeverity) {
            switch (logSeverity.trim()) {
                case "[ERROR]":
                    return "error";
                case "[WARN]":
                    return "warning";
                case "[INFO]":
                    return "info";
                case "[DEBUG]":
                    return "default";

                default:
                    return "default";
            }
        }

        init();

    }
});