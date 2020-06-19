define([
    "text!app/templates/addClientModal.html",
    "text!app/templates/monitor.html",
    "lib/lodash",
    "i18n!app/nls/monitor",
    "css!app/templates/monitor.css"

], function (addClientModalTemplate, monitorTemplate, _, i18n) {
    return function (alvrSettings) {

        function logInit() {
            var url = window.location.href
            var arr = url.split("/");

            const log_listener = new WebSocket("ws://" + arr[2] + "/log");

            log_listener.onopen = (ev) => {
                console.log("log listener started")
            }

            log_listener.onerror = (ev) => {
                console.log("log error", ev)
            }

            log_listener.onclose = (ev) => {
                console.log("log closed", ev)
            }

            log_listener.addEventListener('message', function (e) { addLogLine(e.data) });

        }

        function init() {
            var compiledTemplate = _.template(monitorTemplate);
            var template = compiledTemplate(i18n);

            compiledTemplate = _.template(addClientModalTemplate);
            var template2 = compiledTemplate(i18n);

            $("#monitor").append(template);

            $(document).ready(() => {
                logInit();

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


        function addNewClient(type, ip) {
            const id = ip.replace(/\./g, '');

            if ($("#newClient_" + id).length > 0) {
                console.log("client already in new list");
                return;
            }

            if ($("#trustedClient_" + id).length > 0) {
                console.log("client already in trusted list");
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
                console.log("client already in new list");
                return;
            }

            if ($("#trustedClient_" + id).length > 0) {
                console.log("client already in trusted list");
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
            var split = line.split(" ");
            line = line.replace(split[0] + " " + split[1], "");

            var row = `<tr><td>${split[0]}</td><td>${split[1]}</td><td>${line.trim()}</td></tr>`;
            $("#loggingTable").append(row);
            if ($("#loggingTable").children().length > 500) {
                $("#loggingTable tr").first().remove();
            }
        }

        init();

    }
});