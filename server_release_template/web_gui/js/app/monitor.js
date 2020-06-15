define([
    "lib/lodash",
    "i18n!app/nls/monitor",

], function (_, i18n) {
    return function (alvrSettings) {

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

        log_listener.addEventListener('message', function (e) { console.log(e.data) });

    }
});