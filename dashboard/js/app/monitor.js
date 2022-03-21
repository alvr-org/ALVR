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
    "js/app/util.js",
], function (
    addClientModalTemplate,
    configureClientModalTemplate,
    monitorTemplate,
    session,
    _,
    i18n,
    i18nNotifications
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

            setInterval(fillPerformanceGraphs, 31);
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
                    notificationLevels = ["[ERROR]", "[WARN]", "[INFO]", "[DEBUG]"];
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
                            data: JSON.stringify([deviceName, clientHostname, ip]),
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

                        if (session.clientConnections[_hostmane].manualIps.includes(ip)) {
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
                    ipaddress
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

            const skipWithoutId = $("#_root_extra_excludeNotificationsWithoutId").prop("checked");

            let addToTable = true;
            if (idObject !== undefined) {
                idObject = JSON.parse(idObject);
                handleJson(idObject);
                switch (idObject.id) {
                    case "Statistics":
                        addToTable = false;
                        break;
                    case "GraphStatistics":
                        addToTable = false;
                        break;
                    default:
                        line = idObject.id;
                        break;
                }
            }

            if (notificationLevels.includes(split[1].trim())) {
                if (!(skipWithoutId && idObject === undefined) && Lobibox.notify.list.length < 2) {
                    Lobibox.notify(getNotificationType(split[1]), {
                        size: "mini",
                        rounded: true,
                        delayIndicator: false,
                        sound: false,
                        position: "bottom left",
                        title: getI18nNotification(idObject, line, split[1]).title,
                        msg: getI18nNotification(idObject, line, split[1]).msg,
                    });
                }
            }

            if (addToTable) {
                const row = `<tr><td>${split[0]}</td><td>${
                    split[1]
                }</td><td>${line.trim()}</td></tr>`;
                $("#loggingTable").append(row);
                if ($("#loggingTable").children().length > 500) {
                    $("#loggingTable tr").first().remove();
                }
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
                    console.log("Notification with additional info: ", idObject.id);
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
                case "GraphStatistics":
                    updateGraphStatistics(json.data);
                    break;
                case "SessionUpdated":
                    updateSession();
                    break;
                default:
                    break;
            }
        }

        function legendAsTooltipPlugin({
            className,
            style = {
                backgroundColor: "rgba(255, 249, 196, 0.92)",
                color: "black",
                fontFamily:
                    'Lato,-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,"Helvetica Neue",Arial,sans-serif,"Apple Color Emoji","Segoe UI Emoji","Segoe UI Symbol"',
                fontSize: "80%",
                lineHeight: "1",
            },
        } = {}) {
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
                    ...style,
                });

                const labels = legendEl.querySelectorAll(".u-label");

                for (let i = 0; i < labels.length; i++) labels[i].style.fontWeight = "700";

                const values = legendEl.querySelectorAll(".u-value");

                for (let i = 0; i < values.length; i++) values[i].style.fontWeight = "700";

                // hide series color markers
                //const idents = legendEl.querySelectorAll(".u-marker");

                //for (let i = 0; i < idents.length; i++)
                //idents[i].style.display = "none";

                const overEl = u.over;
                overEl.style.overflow = "visible";

                // move legend into plot bounds
                overEl.appendChild(legendEl);

                // show/hide tooltip on enter/exit
                overEl.addEventListener("mouseenter", () => {
                    legendEl.style.display = null;
                });
                overEl.addEventListener("mouseleave", () => {
                    legendEl.style.display = "none";
                });

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
                },
            };
        }

        function stack(data, omit) {
            let data2 = [];
            let bands = [];
            let d0Len = data[0].length;
            let accum = Array(d0Len);

            for (let i = 0; i < d0Len; i++) accum[i] = 0;

            for (let i = 1; i < data.length; i++)
                data2.push(omit(i) ? data[i] : data[i].map((v, i) => (accum[i] += +v)));

            for (let i = 1; i < data.length; i++)
                !omit(i) &&
                    bands.push({
                        series: [data.findIndex((s, j) => j > i && !omit(j)), i],
                    });

            bands = bands.filter((b) => b.series[1] > -1);

            return {
                data: [data[0]].concat(data2),
                bands,
            };
        }

        function getStackedOpts(opts, data) {
            let stacked = stack(data, (i) => false);

            opts.bands = stacked.bands;

            // restack on toggle
            opts.hooks = {
                setSeries: [
                    (u, i) => {
                        let stacked = stack(data, (i) => !u.series[i].show);
                        u.delBand(null);
                        stacked.bands.forEach((b) => u.addBand(b));
                        u.setData(stacked.data);
                    },
                ],
            };

            return opts;
        }

        function getSharedOpts(opts) {
            opts.cursor = {
                drag: {
                    dist: 10,
                    uni: 20,
                },
                sync: {
                    key: "graph",
                    scales: ["x"],
                },
            };
            (opts.pxAlign = 0),
                (opts.ms = 1),
                (opts.pxSnap = false),
                (opts.plugins = [legendAsTooltipPlugin()]);
            opts.axes = [
                {
                    size: 20,
                    space: 40,
                    values: [
                        [1000, ":{ss}", null, null, null, null, null, null, 1],
                        [1, ":{ss}.{fff}", null, null, null, null, null, null, 1],
                    ],
                    grid: {
                        width: 1,
                    },
                    ticks: {
                        size: 0,
                    },
                },
                {
                    size: 30,
                    space: 20,
                    grid: {
                        width: 1,
                    },
                    ticks: {
                        size: 0,
                    },
                },
            ];
            return opts;
        }

        function getSeries(label, stroke, fill, data, postfix) {
            return {
                label: label,
                stroke: stroke,
                fill: fill,
                value: (u, v, si, i) => (data[si][i] || 0).toFixed(3) + postfix,
                spanGaps: false,
            };
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

        function getLatencyGraphSize() {
            return {
                width: document.getElementById("statisticsCard").clientWidth,
                height: 160,
            };
        }

        function getFramerateGraphSize() {
            return {
                width: document.getElementById("statisticsCard").clientWidth,
                height: 100,
            };
        }

        let themeColor = $("input[name='theme']:checked").val();

        if (themeColor == "systemDefault") {
            if (window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches) {
                themeColor = "darkly";
            } else {
                themeColor = "classic";
            }
        }

        const now = parseInt(new Date().getTime());

        const length = 1200;
        const duration = 10000;

        let latencyGraphData = [
            Array(length + 1).fill(now),
            ...Array(8)
                .fill(null)
                .map((x) => Array(length).fill(null)),
        ];

        latencyGraphData[0].shift();
        latencyGraphData[0].unshift(now - duration);

        const graphColors = ["#7f7f7f", "#d62728", "#ff7f0e", "#1f77b4"];

        let latencyGraphOptions = {
            series: [
                {
                    label: i18n["performanceTotalLatency"],
                    value: (u, v, si, i) =>
                        (latencyGraphData[latencyGraphData.length - 1][i] || 0).toFixed(3) + " ms",
                },
                getSeries(
                    i18n["performanceReceive"],
                    graphColors[0],
                    graphColors[0],
                    latencyGraphData,
                    " ms"
                ),
                getSeries(
                    i18n["performanceRender"],
                    graphColors[1],
                    graphColors[1],
                    latencyGraphData,
                    " ms"
                ),
                getSeries(
                    i18n["performanceIdle"],
                    graphColors[2],
                    graphColors[2],
                    latencyGraphData,
                    " ms"
                ),
                getSeries(
                    i18n["performanceEncode"],
                    graphColors[3],
                    graphColors[3],
                    latencyGraphData,
                    " ms"
                ),
                getSeries(
                    i18n["performanceSend"],
                    graphColors[0],
                    graphColors[0],
                    latencyGraphData,
                    " ms"
                ),
                getSeries(
                    i18n["performanceDecode"],
                    graphColors[3],
                    graphColors[3],
                    latencyGraphData,
                    " ms"
                ),
                getSeries(
                    i18n["performanceClientIdle"],
                    graphColors[2],
                    graphColors[2],
                    latencyGraphData,
                    " ms"
                ),
            ],
        };

        latencyGraphOptions = getSharedOpts(latencyGraphOptions);
        if (themeColor == "darkly") {
            latencyGraphOptions = getThemedOpts(latencyGraphOptions);
        }
        latencyGraphOptions = getStackedOpts(latencyGraphOptions, latencyGraphData);

        let framerateGraphData = [
            Array(length + 1).fill(now),
            Array(length).fill(null),
            Array(length).fill(null),
        ];

        framerateGraphData[0].shift();
        framerateGraphData[0].unshift(now - duration);

        let framerateGraphOptions = {
            series: [
                {
                    label: "---",
                    value: "",
                    show: false,
                },
                getSeries(
                    i18n["performanceServer"],
                    graphColors[3],
                    null,
                    framerateGraphData,
                    " FPS"
                ),
                getSeries(
                    i18n["performanceClient"],
                    graphColors[2],
                    null,
                    framerateGraphData,
                    " FPS"
                ),
            ],
        };

        framerateGraphOptions = getSharedOpts(framerateGraphOptions);
        if (themeColor == "darkly") {
            framerateGraphOptions = getThemedOpts(framerateGraphOptions);
        }

        function initPerformanceGraphs() {
            latencyGraph = new uPlot(
                latencyGraphOptions,
                latencyGraphData,
                document.getElementById("latencyGraphArea")
            );
            framerateGraph = new uPlot(
                framerateGraphOptions,
                framerateGraphData,
                document.getElementById("framerateGraphArea")
            );
        }

        let lastStatisticsUpdate = now;
        let lastGraphUpdate = now;
        let lastGraphRedraw = now;

        function updatePerformanceGraphs(statistics) {
            const now = parseInt(new Date().getTime());

            for (let i = 0; i < latencyGraphData.length; i++) {
                latencyGraphData[i].shift();
            }

            latencyGraphData[0].push(statistics[0]);
            if (statistics[1] < Infinity) {
                latencyGraphData[1].push(statistics[2]);
                latencyGraphData[2].push(statistics[3]);
                latencyGraphData[3].push(statistics[4] + statistics[5]);
                latencyGraphData[4].push(statistics[6]);
                latencyGraphData[5].push(statistics[7]);
                latencyGraphData[6].push(statistics[8]);
                latencyGraphData[7].push(statistics[9]);
                latencyGraphData[8].push(statistics[1]);
            } else {
                for (let i = 1; i < latencyGraphData.length; i++) {
                    latencyGraphData[i].push(null);
                }
            }

            for (let i = 0; i < framerateGraphData.length; i++) {
                framerateGraphData[i].shift();
            }

            framerateGraphData[0].push(statistics[0]);
            framerateGraphData[1].push(statistics[11]);
            framerateGraphData[2].push(statistics[10]);

            lastStatistics = statistics;
            lastGraphUpdate = now;
        }

        function redrawPerformanceGraphs(statistics) {
            const now = parseInt(new Date().getTime());

            if (now > lastGraphRedraw + 32) {
                latencyGraphData[0].pop();
                latencyGraphData[0].push(statistics[0]);

                latencyGraphData[0].shift();
                latencyGraphData[0].unshift(statistics[0] - duration);

                framerateGraphData[0].pop();
                framerateGraphData[0].push(statistics[0]);

                framerateGraphData[0].shift();
                framerateGraphData[0].unshift(statistics[0] - duration);

                const ldata = []
                    .concat(latencyGraphData[latencyGraphData.length - 1])
                    .filter((v, i) => latencyGraphData[0][i] > now - 10 * 1000)
                    .filter(Boolean);
                const lq1 = quantile(ldata, 0.25);
                const lq3 = quantile(ldata, 0.75);
                //const lq1 = 0;
                //const lq3 = quantile(ldata,0.5);
                latencyGraph.batch(() => {
                    latencyGraph.setScale("y", { min: 0, max: lq3 + (lq3 - lq1) * 3 });
                    //latencyGraph.setScale("y", {min: 0, max: lq3+(lq3-lq1)*1.5});
                    latencyGraph.setData(stack(latencyGraphData, (i) => false).data);
                });
                const fdata1 = []
                    .concat(framerateGraphData[1])
                    .filter((v, i) => latencyGraphData[0][i] > now - 10 * 1000)
                    .filter(Boolean);
                const fdata2 = []
                    .concat(framerateGraphData[2])
                    .filter((v, i) => latencyGraphData[0][i] > now - 10 * 1000)
                    .filter(Boolean);
                const fdata = fdata1.concat(fdata2);
                const fq1 = quantile(fdata, 0.25);
                const fq3 = quantile(fdata, 0.75);
                latencyGraph.batch(() => {
                    framerateGraph.setScale("y", {
                        min: fq1 - (fq3 - fq1) * 1.5,
                        max: fq3 + (fq3 - fq1) * 1.5,
                    });
                    framerateGraph.setData(framerateGraphData);
                });
                lastGraphRedraw = now;
            }
        }

        let lastStatistics = {};
        let statisticsUpdateStopped = true;
        let statisticsRedrawStopped = true;

        function fillPerformanceGraphs() {
            latencyGraph.setSize(getLatencyGraphSize());
            framerateGraph.setSize(getFramerateGraphSize());
            if (!statisticsRedrawStopped) {
                const now = parseInt(new Date().getTime());
                lastStatistics[0] = now;
                if ((now - 32 > lastGraphRedraw) & (now - 1000 < lastStatisticsUpdate)) {
                    if (now - 100 > lastGraphUpdate) {
                        if (!statisticsUpdateStopped) {
                            statisticsUpdateStopped = true;
                            lastStatistics.fill(null);
                            lastStatistics[0] = lastGraphUpdate + 20;
                            updatePerformanceGraphs(lastStatistics);
                        }
                    }
                    redrawPerformanceGraphs(lastStatistics);
                } else if (now - 1000 > lastStatisticsUpdate) statisticsRedrawStopped = true;
            }
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
                if ($("#connection").hasClass("active")) $("#connection").removeClass("active");
                // show statistics
                if (!$("#statisticsTab").hasClass("active")) $("#statisticsTab").addClass("active");
                if (!$("#statistics").hasClass("active")) $("#statistics").addClass("active");
                if (!$("#statistics").hasClass("show")) $("#statistics").addClass("show");
                // hide logging
                if ($("#loggingTab").hasClass("active")) $("#loggingTab").removeClass("active");
                if ($("#logging").hasClass("active")) $("#logging").removeClass("active");
                if ($("#logging").hasClass("show")) $("#logging").removeClass("show");
            }
            timeoutHandler = setTimeout(() => {
                // $("#connectionCard").show();
                // $("#statisticsCard").hide();
                clientConnected = false;
                // show connection
                if (!$("#connectionTab").hasClass("active")) $("#connectionTab").addClass("active");
                if (!$("#connection").hasClass("active")) $("#connection").addClass("active");
                // hide statistics
                if ($("#statisticsTab").hasClass("active"))
                    $("#statisticsTab").removeClass("active");
                if ($("#statistics").hasClass("active")) $("#statistics").removeClass("active");
                if ($("#statistics").hasClass("show")) $("#statistics").removeClass("show");
                // hide logging
                if ($("#loggingTab").hasClass("active")) $("#loggingTab").removeClass("active");
                if ($("#logging").hasClass("active")) $("#logging").removeClass("active");
                if ($("#logging").hasClass("show")) $("#logging").removeClass("show");
            }, 2000);

            for (const stat in statistics) {
                $("#statistic_" + stat).text(statistics[stat]);
            }
        }

        function updateGraphStatistics(statistics) {
            const now = parseInt(new Date().getTime());

            lastStatisticsUpdate = now;

            if (statisticsUpdateStopped) statisticsUpdateStopped = false;
            if (statisticsRedrawStopped) {
                lastStatistics[0] = now - 20;
                updatePerformanceGraphs(lastStatistics);
                statisticsRedrawStopped = false;
            }
            updatePerformanceGraphs(statistics);
            redrawPerformanceGraphs(statistics);
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
                    if (!(ch & 0x40) || !extra || index + extra > count) return null;

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
