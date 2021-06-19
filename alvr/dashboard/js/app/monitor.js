define([
    "text!app/templates/addClientModal.html",
    "text!app/templates/configureClientModal.html",
    "text!app/templates/monitor.html",
    "json!../../api/session/load",
    "lib/lodash",
    "i18n!app/nls/monitor",
    "i18n!app/nls/notifications",
    "css!app/templates/monitor.css",
    // eslint-disable-next-line requirejs/no-js-extension
    "js/lib/uPlot.iife.min.js",
    "css!js/lib/uPlot.min.css",
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

            const log_listener = new WebSocket("ws://" + arr[2] + "/api/log");

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
                const displayName = connection.displayName;

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
                            url: "api/client/add",
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
                            url: "api/client/trust",
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
                    url: "api/client/remove",
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
                        url: "api/client/trust",
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
                        url: "api/client/remove",
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
                case "Statistics":
                    updateStatistics(json.data);
                    break;
                case "SessionUpdated":
                    updateSession();
                    break;
                default:
                    break;
            }
        }

        function legendAsTooltipPlugin({ className, style = { backgroundColor:"rgba(255, 249, 196, 0.92)", color: "black" } } = {}) {
            let legendEl;

            function init(u, opts) {
                legendEl = u.root.querySelector(".u-legend");

                legendEl.classList.remove("u-inline");
                className && legendEl.classList.add(className);

                uPlot.assign(legendEl.style, {
                    textAlign: "left",
                    pointerEvents: "none",
                    display: "none",
                    position: "absolute",
                    left: 0,
                    top: 0,
                    zIndex: 100,
                    boxShadow: "2px 2px 10px rgba(0,0,0,0.5)",
                    ...style
                });

                // hide series color markers
                const idents = legendEl.querySelectorAll(".u-marker");

                for (let i = 0; i < idents.length; i++)
                    idents[i].style.display = "none";

                const overEl = u.over;
                overEl.style.overflow = "visible";

                // move legend into plot bounds
                overEl.appendChild(legendEl);

                // show/hide tooltip on enter/exit
                overEl.addEventListener("mouseenter", () => {legendEl.style.display = null;});
                overEl.addEventListener("mouseleave", () => {legendEl.style.display = "none";});

                // let tooltip exit plot
            //    overEl.style.overflow = "visible";
            }

            function update(u) {
                const { left, top } = u.cursor;
                legendEl.style.transform = "translate(" + left + "px, " + top + "px)";
            }

            return {
                hooks: {
                    init: init,
                    setCursor: update,
                }
            };
        }

        function stack(data, omit) {
            let data2 = [];
            let bands = [];
            let d0Len = data[0].length;
            let accum = Array(d0Len);

            for (let i = 0; i < d0Len; i++)
                accum[i] = 0;

            for (let i = 1; i < data.length; i++)
                data2.push(omit(i) ? data[i] : data[i].map((v, i) => (accum[i] += +v)));

            for (let i = 1; i < data.length; i++)
                !omit(i) && bands.push({
                    series: [
                        data.findIndex((s, j) => j > i && !omit(j)),
                        i,
                    ],
                });

            bands = bands.filter(b => b.series[1] > -1);

            return {
                data: [data[0]].concat(data2),
                bands,
            };
        }

        function getStackedOpts(opts, data) {
            let stacked = stack(data, i => false);

            opts.bands = stacked.bands;

            // restack on toggle
            opts.hooks = {
                setSeries: [
                    (u, i) => {
                        let stacked = stack(data, i => !u.series[i].show);
                        u.delBand(null);
                        stacked.bands.forEach(b => u.addBand(b));
                        u.setData(stacked.data);
                    }
                ],
            };

            return {opts, data: stacked.data};
        }

        function getThemedOpts(opts) {
            opts.axes[0].stroke = "#ffffff";
            opts.axes[0].grid.stroke = "#444444";
            opts.axes[0].ticks.stroke = "#444444";
            opts.axes[1].stroke = "#ffffff";
            opts.axes[1].grid.stroke = "#444444";
            opts.axes[1].ticks.stroke = "#444444";
            return opts;
        }

        let themeColor = $("input[name='theme']:checked").val();

        if (themeColor == "systemDefault") {
            if (
                window.matchMedia &&
                window.matchMedia("(prefers-color-scheme: dark)").matches
            ) {
                themeColor = "darkly";
            } else {
                themeColor = "classic";
            }
        }

        const now = parseInt(new Date().getTime());

        const length = 2000;

        let latencyGraphData = [
            Array(length).fill(now),
            Array(length).fill(null),
            Array(length).fill(null),
            Array(length).fill(null),
            Array(length).fill(null),
            Array(length).fill(null),
            Array(length).fill(null),
            Array(length).fill(null),
            Array(length).fill(null),
        ];

        latencyGraphData[0].shift();
        latencyGraphData[0].unshift(now - 10000);

        let latencyGraphOptions = {
            width: 560,
            height: 220,
            cursor: {
                 drag: {
                    dist: 10,
                    uni: 20,
                 },
                 sync: {
                    key: "graph",
                    scales: ["x"],
                 },
            },
            pxAlign: 0,
            ms: 1,
            pxSnap: false,
            plugins: [
                legendAsTooltipPlugin(),
            ],
            series:
            [
                {
                    label: "Total",
                    value: (u, v, si, i) => (latencyGraphData[1][i] + latencyGraphData[2][i] + latencyGraphData[3][i] + latencyGraphData[4][i] || 0).toFixed(3) + " ms",
                },
                {
                    label: "Send",
                    stroke: "#ed38c0",
                    fill: "#ed38c0",
                    value: (u, v, si, i) => (latencyGraphData[si][i] || 0).toFixed(3) + " ms",
                    spanGaps: false,
                },
                {
                    label: "Render",
                    stroke: "#4a15ea",
                    fill: "#4a15ea",
                    value: (u, v, si, i) => (latencyGraphData[si][i] || 0).toFixed(3) + " ms",
                    spanGaps: false,
                },
                {
                    label: "Idle",
                    stroke: "#1be44e",
                    fill: "#1be44e",
                    value: (u, v, si, i) => (latencyGraphData[si][i] || 0).toFixed(3) + " ms",
                    spanGaps: false,
                },
                {
                    label: "Wait",
                    stroke: "#d5d52b",
                    fill: "#d5d52b",
                    value: (u, v, si, i) => (latencyGraphData[si][i] || 0).toFixed(3) + " ms",
                    spanGaps: false,
                },
                {
                    label: "Encode",
                    stroke: "#1f77b4",
                    fill: "#1f77b4",
                    value: (u, v, si, i) => (latencyGraphData[si][i] || 0).toFixed(3) + " ms",
                    spanGaps: false,
                },
                {
                    label: "Transport",
                    stroke: "#2ca02c",
                    fill: "#2ca02c",
                    value: (u, v, si, i) => (latencyGraphData[si][i] || 0).toFixed(3) + " ms",
                    spanGaps: false,
                },
                {
                    label: "Decode",
                    stroke: "#ff7f0e",
                    fill: "#ff7f0e",
                    value: (u, v, si, i) => (latencyGraphData[si][i] || 0).toFixed(3) + " ms",
                    spanGaps: false,
                },
                {
                    label: "Other",
                    stroke: "#d62728",
                    fill: "#d62728",
                    value: (u, v, si, i) => (latencyGraphData[si][i] || 0).toFixed(3) + " ms",
                    spanGaps: false,
                },
            ],
            axes:
            [
                {
                    space: 40,
                    values: [
                        [1000, ":{ss}", null, null, null, null, null, null, 1],
                        [1, ":{ss}.{fff}", null, null, null, null, null, null, 1],
                    ],
                    grid: {},
                    ticks: {},
                },
                {
                    grid: {},
                    ticks: {},
                },
            ],
        };

        if (themeColor == "darkly") {
            latencyGraphOptions = getThemedOpts(latencyGraphOptions);
        }
        latencyGraphOptions = getStackedOpts(latencyGraphOptions, latencyGraphData).opts;

        let framerateGraphData = [
            Array(length).fill(now),
            Array(length).fill(null),
            Array(length).fill(null),
        ];

        framerateGraphData[0].shift();
        framerateGraphData[0].unshift(now - 10000);

        let framerateGraphOptions = {
            width: 560,
            height: 180,
            cursor: {
                 drag: {
                    dist: 10,
                    uni: 20,
                 },
                 sync: {
                    key: "graph",
                    scales: ["x"],
                 },
            },
            pxAlign: 0,
            ms: 1,
            pxSnap: false,
            plugins: [
                legendAsTooltipPlugin(),
            ],
            series:
            [
                {
                    label: "---",
                    value: "",
                    show: false,
                },
                {
                    label: "Server",
                    stroke: "#1f77b4",
                    value: (u, v, si, i) => (framerateGraphData[si][i] || 0).toFixed(3) + " FPS",
                    spanGaps: false,
                },
                {
                    label: "Client",
                    stroke: "#ff7f0e",
                    value: (u, v, si, i) => (framerateGraphData[si][i] || 0).toFixed(3) + " FPS",
                    spanGaps: false,
                },
            ],
            axes:
            [
                {
                    space: 40,
                    values: [
                        [1000, ":{ss}", null, null, null, null, null, null, 1],
                        [1, ":{ss}.{fff}", null, null, null, null, null, null, 1],
                    ],
                    grid: {},
                    ticks: {},
                },
                {
                    grid: {},
                    ticks: {},
                },
            ],
        };

        if (themeColor == "darkly") {
            framerateGraphOptions = getThemedOpts(framerateGraphOptions);
        }

        function initPerformanceGraphs() {
            latencyGraph = new uPlot(latencyGraphOptions, latencyGraphData, document.getElementById("latencyGraphArea"));
            framerateGraph = new uPlot(framerateGraphOptions, framerateGraphData, document.getElementById("framerateGraphArea"));
        }

        function updatePerformanceGraphs(statistics) {
            const now = parseInt(new Date().getTime());

            const otherLatency =
                statistics["totalLatency"] -
                statistics["sendLatency"] -
                statistics["renderTime"] -
                statistics["idleTime"] -
                statistics["waitTime"] -
                statistics["encodeLatency"] -
                statistics["transportLatency"] -
                statistics["decodeLatency"];

            if (otherLatency > 0) {
                for (let i = 0; i < 9; i++) {
                    latencyGraphData[i].shift();
                }

                latencyGraphData[0].push(now);
                latencyGraphData[1].push(statistics["sendLatency"]);
                latencyGraphData[2].push(statistics["renderTime"]);
                latencyGraphData[3].push(statistics["idleTime"]);
                latencyGraphData[4].push(statistics["waitTime"]);
                latencyGraphData[5].push(statistics["encodeLatency"]);
                latencyGraphData[6].push(statistics["transportLatency"]);
                latencyGraphData[7].push(statistics["decodeLatency"]);
                latencyGraphData[8].push(otherLatency);

                latencyGraphData[0].shift();
                latencyGraphData[0].unshift(now - 10000);

                latencyGraph.setData(stack(latencyGraphData, i => false).data);
            }
            else {
                for (let i = 1; i < 9; i++) {
                    latencyGraphData[i].shift();
                    latencyGraphData[i].push(null);
                }

                latencyGraphData[0].shift();
                latencyGraphData[0].push(now);
            }

            for (let i = 0; i < 3; i++) {
                framerateGraphData[i].shift();
            }

            framerateGraphData[0].push(now);
            framerateGraphData[1].push(statistics["serverFPS"]);
            framerateGraphData[2].push(statistics["clientFPS"]);

            framerateGraphData[0].shift();
            framerateGraphData[0].unshift(now - 10000);

            framerateGraph.setData(framerateGraphData);
        }

        let lastStatisticsUpdate = now;

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

            const now = parseInt(new Date().getTime());

            if (now > lastStatisticsUpdate + 100) {
                for (const stat in statistics) {
                    $("#statistic_" + stat).text(statistics[stat]);
                }
                lastStatisticsUpdate = now
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

                for (let i = 1; i < 9; i++) {
                    latencyGraphData[i].shift();
                    latencyGraphData[i].push(null);
                }

                latencyGraphData[0].shift();
                latencyGraphData[0].push(now);

                for (let i = 1; i < 3; i++) {
                    framerateGraphData[i].shift();
                    framerateGraphData[i].push(null);
                }

                framerateGraphData[0].shift();
                framerateGraphData[0].push(now);

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
            $.getJSON("api/session/load", function (newSession) {
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
