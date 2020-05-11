const log_listener = new EventSource('log');
log_listener.addEventListener('message', e => alert(e.data))