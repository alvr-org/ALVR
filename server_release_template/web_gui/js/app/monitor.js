define([
    "text!app/templates/addClientModal.html",
    "text!app/templates/configureClientModal.html",
    "text!app/templates/monitor.html",
    "json!../../session",
    "lib/lodash",
    "i18n!app/nls/monitor",
    "i18n!app/nls/notifications",
    "css!app/templates/monitor.css",
    // eslint-disable-next-line requirejs/no-js-extension
    "js/lib/epoch.js",
    "css!js/lib/epoch.css",
], function (
    addClientModalTemplate,
    configureClientModalTemplate,
    monitorTemplate,
    session,
    _,
    i18n,
    i18nNotifications,
) {
    return function (alvrSettings) {
        let notificationLevels = [];
        let timeoutHandler;
        let latencyGraph;
        let framerateGraph;
        let clientConnected = false;

        function logInit() {
            const url = window.location.href;
            const arr = url.split("/");

            const log_listener = new WebSocket("ws://" + arr[2] + "/log");

            log_listener.onopen = (ev) => {
                console.log("Log listener started");
            };

            log_listener.onerror = (ev) => {
                console.log("Log error", ev);
            };

            log_listener.onclose = (ev) => {
                console.log("Log closed", ev);
                logInit();
            };

            log_listener.addEventListener("message", function (e) {
                addLogLine(e.data);
            });

            $("#_root_extra_notificationLevel-choice-").change((ev) => {
                initNotificationLevel();
            });
        }

        function init() {
            let compiledTemplate = _.template(monitorTemplate);
            const template = compiledTemplate(i18n);

            compiledTemplate = _.template(addClientModalTemplate);
            const templateAddClient = compiledTemplate(i18n);

            $("#monitor").append(template);

            $(document).ready(() => {
                logInit();
                initNotificationLevel();
                initAddClientModal(templateAddClient);
                initPerformanceGraphs();

                updateClients();
            });
        }

        function updateClients() {
            $("#newClientsDiv" + " table").empty();
            $("#trustedClientsDiv" + " table").empty();

            Object.entries(session.clientConnections).forEach((pair) => {
                const hostname = pair[0];
                const connection = pair[1];
                //var address = connection.lastLocalIp;
                const displayName = connection.deviceName;

                if (connection.trusted) {
                    addTrustedClient(displayName, hostname);
                } else {
                    addNewClient(displayName, hostname);
                }
            });
        }

        function initNotificationLevel() {
            const level = $("input[name='notificationLevel']:checked").val();

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
                    notificationLevels = [
                        "[ERROR]",
                        "[WARN]",
                        "[INFO]",
                        "[DEBUG]",
                    ];
                    break;
                default:
                    notificationLevels = [];
                    break;
            }
        }

        function initAddClientModal(template) {
            $("#showAddClientModal").click(() => {
                $("#addClientModal").remove();
                $("body").append(template);
                $(document).ready(() => {
                    $("#addClientModal").modal({
                        backdrop: "static",
                        keyboard: false,
                    });
                    $("#clientAddButton").click(() => {
                        const deviceName = $("#deviceName").val();
                        const clientHostname = $("#clientHostname").val();
                        const ip = $("#clientIP").val();

                        if (!validateHostname(clientHostname)) {
                            Lobibox.notify("error", {
                                size: "mini",
                                rounded: true,
                                delayIndicator: false,
                                sound: false,
                                position: "bottom right",
                                msg: i18n["error_DuplicateHostname"],
                            });
                            return;
                        }

                        if (!validateIPv4address(ip)) {
                            Lobibox.notify("error", {
                                size: "mini",
                                rounded: true,
                                delayIndicator: false,
                                sound: false,
                                position: "bottom right",
                                msg: i18n["error_InvalidIp"],
                            });
                            return;
                        }

                        $.ajax({
                            type: "POST",
                            url: "client/add",
                            contentType: "application/json;charset=UTF-8",
                            data: JSON.stringify([
                                deviceName,
                                clientHostname,
                                ip,
                            ]),
                        });

                        $("#addClientModal").modal("hide");
                        $("#addClientModal").remove();
                    });
                });
            });
        }

        function initConfigureClientModal(hostname) {
            const id = hostname.replace(/\./g, "");
            $("#btnConfigureClient_" + id).click(() => {
                compiledTemplate = _.template(configureClientModalTemplate);
                templateConfigureClient = compiledTemplate({
                    i18n: i18n,
                    knownIps: session.clientConnections[hostname].manualIps,
                });

                $("#configureClientModal").remove();
                $("body").append(templateConfigureClient);

                const _hostmane = hostname;
                // this call need const variable unless you want them overwriten by the next call.
                $(document).ready(() => {
                    $("#configureClientModal").modal({
                        backdrop: "static",
                        keyboard: false,
                    });

                    $("#addNewIpAddressButton").click(() => {
                        const ip = $("#newIpAddress").val();

                        if (
                            session.clientConnections[
                                _hostmane
                            ].manualIps.includes(ip)
                        ) {
                            Lobibox.notify("error", {
                                size: "mini",
                                rounded: true,
                                delayIndicator: false,
                                sound: false,
                                position: "bottom right",
                                msg: i18n["error_DuplicateIp"],
                            });
                            return;
                        }

                        if (!validateIPv4address(ip)) {
                            Lobibox.notify("error", {
                                size: "mini",
                                rounded: true,
                                delayIndicator: false,
                                sound: false,
                                position: "bottom right",
                                msg: i18n["error_InvalidIp"],
                            });
                            return;
                        }

                        $.ajax({
                            type: "POST",
                            url: "client/trust",
                            contentType: "application/json;charset=UTF-8",
                            data: JSON.stringify([_hostmane, ip]),
                        });

                        $("#knowIpsListDiv").append(`
                            <div class="row mt-2"><div class="col">
                                <span>${ip}</span>
                                <button type="button" class="btn btn-sm btn-primary float-right removeIpAddressButton" data-ip="${ip}">${i18n["configureClientRemoveIp"]}</button>
                            </div></div>`);
                        configureClientModal_BindRemoveIpButtons(_hostmane);
                    });

                    configureClientModal_BindRemoveIpButtons(_hostmane);
                });
            });
        }

        function configureClientModal_BindRemoveIpButtons(hostname) {
            $(".removeIpAddressButton").off("click");
            $(".removeIpAddressButton").click((evt) => {
                const ip = $(evt.target).attr("data-ip");

                $.ajax({
                    type: "POST",
                    url: "client/remove",
                    contentType: "application/json;charset=UTF-8",
                    data: JSON.stringify([hostname, ip]),
                });

                $(evt.target).parent().parent().remove();
            });
        }

        function addNewClient(displayName, hostname) {
            if (!validateHostname(hostname)) return;

            const id = hostname.replace(/\./g, "");

            $("#newClientsDiv" + " table")
                .append(`<tr><td type="${displayName}" hostname="${hostname}" id="newClient_${id}">${displayName} (${hostname}) </td>
            <td><button type="button" id="btnAddTrustedClient_${id}" class="btn btn-primary">${i18n["addTrustedClient"]}</button>
            </td></tr>`);

            const _hostmane = hostname;
            // this call need const variable unless you want them overwriten by the next call.
            $(document).ready(() => {
                $("#btnAddTrustedClient_" + id).click(() => {
                    $.ajax({
                        type: "POST",
                        url: "client/trust",
                        contentType: "application/json;charset=UTF-8",
                        data: JSON.stringify([_hostmane, null]),
                    });
                });
            });
        }

        function addTrustedClient(displayName, hostname) {
            if (!validateHostname(hostname)) return;

            const id = hostname.replace(/\./g, "");

            $("#trustedClientsDiv" + " table")
                .append(`<tr><td type="${displayName}" hostname="${hostname}" id="trustedClient_${id}">${displayName} (${hostname}) </td>
            <td><button type="button" id="btnConfigureClient_${id}" class="btn btn-primary ml-auto">${i18n["configureClientButton"]}</button>
            <button type="button" id="btnRemoveTrustedClient_${id}" class="btn btn-primary">${i18n["removeTrustedClient"]}</button>
            </td></tr>`);

            const _hostmane = hostname;
            // this call need const variable unless you want them overwriten by the next call.
            $(document).ready(() => {
                $("#btnRemoveTrustedClient_" + id).click(() => {
                    $.ajax({
                        type: "POST",
                        url: "client/remove",
                        contentType: "application/json;charset=UTF-8",
                        data: JSON.stringify([_hostmane, null]),
                    });
                });
            });

            initConfigureClientModal(hostname);
        }

        function validateHostname(hostname) {
            const id = hostname.replace(/\./g, "");

            if ($("#newClient_" + id).length > 0) {
                console.warn("Client already in new list:", hostname);
                return false;
            }

            if ($("#trustedClient_" + id).length > 0) {
                console.warn("Client already in trusted list:", hostname);
                return false;
            }
            return true;
        }

        function validateIPv4address(ipaddress) {
            if (
                /^(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/.test(
                    ipaddress,
                )
            ) {
                return true;
            }
            console.warn("The IP address is invalid.");
            return false;
        }

        function addLogLine(line) {
            let idObject = undefined;

            console.log(line);

            const json_start_idx = line.indexOf("#{");
            const json_end_idx = line.indexOf("}#");
            if (json_start_idx != -1 && json_end_idx != -1) {
                idObject = line.substring(json_start_idx + 1, json_end_idx + 1);
            }

            const split = line.split(" ");
            line = line.replace(split[0] + " " + split[1], "");

            const skipWithoutId = $(
                "#_root_extra_excludeNotificationsWithoutId",
            ).prop("checked");

            if (idObject !== undefined) {
                idObject = JSON.parse(idObject);
                handleJson(idObject);
            }

            if (notificationLevels.includes(split[1].trim())) {
                if (
                    !(skipWithoutId && idObject === undefined) &&
                    Lobibox.notify.list.length < 2
                ) {
                    Lobibox.notify(getNotificationType(split[1]), {
                        size: "mini",
                        rounded: true,
                        delayIndicator: false,
                        sound: false,
                        position: "bottom left",
                        title: getI18nNotification(idObject, line, split[1])
                            .title,
                        msg: getI18nNotification(idObject, line, split[1]).msg,
                    });
                }
            }

            const row = `<tr><td>${split[0]}</td><td>${
                split[1]
            }</td><td>${line.trim()}</td></tr>`;
            $("#loggingTable").append(row);
            if ($("#loggingTable").children().length > 500) {
                $("#loggingTable tr").first().remove();
            }
        }

        function getI18nNotification(idObject, line, level) {
            if (idObject === undefined) {
                return { title: level, msg: line };
            } else {
                //TODO: line could contain additional info for the msg

                if (i18nNotifications[idObject.id + ".title"] !== undefined) {
                    return {
                        title: i18nNotifications[idObject.id + ".title"],
                        msg: i18nNotifications[idObject.id + ".msg"],
                    };
                } else {
                    console.log(
                        "Notification with additional info: ",
                        idObject.id,
                    );
                    return { title: level, msg: idObject.id + ": " + line };
                }
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

        function handleJson(json) {
            switch (json.id) {
                case "statistics":
                    updateStatistics(json.content);
                    break;
                case "sessionUpdated":
                    updateSession();
                    break;
                default:
                    break;
            }
        }

        function initPerformanceGraphs() {
            const now = parseInt(new Date().getTime() / 1000);
            latencyGraph = $("#latencyGraphArea").epoch({
                type: "time.area",
                axes: ["left", "bottom"],
                data: [
                    {
                        label: "Encode",
                        values: [{ time: now, y: 0 }],
                    },
                    {
                        label: "Decode",
                        values: [{ time: now, y: 0 }],
                    },
                    {
                        label: "Transport",
                        values: [{ time: now, y: 0 }],
                    },
                    {
                        label: "Other",
                        values: [{ time: now, y: 0 }],
                    },
                ],
            });

            framerateGraph = $("#framerateGraphArea").epoch({
                type: "time.line",
                axes: ["left", "bottom"],
                data: [
                    {
                        label: "Server FPS",
                        values: [{ time: now, y: 0 }],
                    },
                    {
                        label: "Client FPS",
                        values: [{ time: now, y: 0 }],
                    },
                ],
            });
        }

        function updatePerformanceGraphs(statistics) {
            $("#divPerformanceGraphsContent").show();
            $("#divPerformanceGraphsEmptyMsg").hide();

            const now = parseInt(new Date().getTime() / 1000);
            const otherLatency =
                statistics["totalLatency"] -
                statistics["encodeLatency"] -
                statistics["decodeLatency"] -
                statistics["transportLatency"];

            latencyGraph.push([
                { time: now, y: statistics["encodeLatency"] },
                { time: now, y: statistics["decodeLatency"] },
                { time: now, y: statistics["transportLatency"] },
                { time: now, y: otherLatency },
            ]);

            framerateGraph.push([
                { time: now, y: statistics["serverFPS"] },
                { time: now, y: statistics["clientFPS"] },
            ]);
        }

        function updateStatistics(statistics) {
            clearTimeout(timeoutHandler);
            // $("#connectionCard").hide();
            // $("#statisticsCard").show();
            if (!clientConnected) {
                clientConnected = true;
                // hide connection
                if ($("#connectionTab").hasClass("active"))
                    $("#connectionTab").removeClass("active");
                if ($("#connection").hasClass("active"))
                    $("#connection").removeClass("active");
                // show statistics
                if (!$("#statisticsTab").hasClass("active"))
                    $("#statisticsTab").addClass("active");
                if (!$("#statistics").hasClass("active"))
                    $("#statistics").addClass("active");
                if (!$("#statistics").hasClass("show"))
                    $("#statistics").addClass("show");
                // hide performanceGraphs
                if ($("#performanceGraphsTab").hasClass("active"))
                    $("#performanceGraphsTab").removeClass("active");
                if ($("#performanceGraphs").hasClass("active"))
                    $("#performanceGraphs").removeClass("active");
                if ($("#performanceGraphs").hasClass("show"))
                    $("#performanceGraphs").removeClass("show");
                // hide logging
                if ($("#loggingTab").hasClass("active"))
                    $("#loggingTab").removeClass("active");
                if ($("#logging").hasClass("active"))
                    $("#logging").removeClass("active");
                if ($("#logging").hasClass("show"))
                    $("#logging").removeClass("show");
            }

            for (const stat in statistics) {
                $("#statistic_" + stat).text(statistics[stat]);
            }
            timeoutHandler = setTimeout(() => {
                // $("#connectionCard").show();
                // $("#statisticsCard").hide();
                clientConnected = false;
                // show connection
                if (!$("#connectionTab").hasClass("active"))
                    $("#connectionTab").addClass("active");
                if (!$("#connection").hasClass("active"))
                    $("#connection").addClass("active");
                // hide statistics
                if ($("#statisticsTab").hasClass("active"))
                    $("#statisticsTab").removeClass("active");
                if ($("#statistics").hasClass("active"))
                    $("#statistics").removeClass("active");
                if ($("#statistics").hasClass("show"))
                    $("#statistics").removeClass("show");
                // hide performanceGraphs
                if ($("#performanceGraphsTab").hasClass("active"))
                    $("#performanceGraphsTab").removeClass("active");
                if ($("#performanceGraphs").hasClass("active"))
                    $("#performanceGraphs").removeClass("active");
                if ($("#performanceGraphs").hasClass("show"))
                    $("#performanceGraphs").removeClass("show");
                // hide logging
                if ($("#loggingTab").hasClass("active"))
                    $("#loggingTab").removeClass("active");
                if ($("#logging").hasClass("active"))
                    $("#logging").removeClass("active");
                if ($("#logging").hasClass("show"))
                    $("#logging").removeClass("show");
            }, 2000);

            updatePerformanceGraphs(statistics);
        }

        let isUpdating = false;

        function updateSession() {
            //ugly hack to avoid loop
            if (isUpdating) {
                return;
            }
            isUpdating = true;
            $.getJSON("session", function (newSession) {
                session = newSession;
                updateClients();
                alvrSettings.updateSession(session);
                isUpdating = false;
            });
        }

        function pack(data) {
            const extraByteMap = [1, 1, 1, 1, 2, 2, 3, 0];
            const count = data.length;
            let str = "";

            for (let index = 0; index < count; ) {
                let ch = data[index++];
                if (ch & 0x80) {
                    let extra = extraByteMap[(ch >> 3) & 0x07];
                    if (!(ch & 0x40) || !extra || index + extra > count)
                        return null;

                    ch = ch & (0x3f >> extra);
                    for (; extra > 0; extra -= 1) {
                        const chx = data[index++];
                        if ((chx & 0xc0) != 0x80) return null;

                        ch = (ch << 6) | (chx & 0x3f);
                    }
                }

                str += String.fromCharCode(ch);
            }

            return str;
        }

        init();
    };
});
