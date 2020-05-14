const log_listener = new WebSocket("ws://127.0.0.1:8080/log");
log_listener.addEventListener('message', function (e) { alert(e.data) });