# Common
ok = OK
cancel = Cancel
default = Default
custom = Custom
enabled = Enabled
disabled = Disabled
set = Set
unset = Unset
apply = Apply
applied = Applied
reset-prompt = Do you want to reset this setting to {$value}?


# Dashboard

connections = Connections
    .new-clients = New clients
    .trust = Trust
    .trusted-clients = Trusted clients
    .add-client-manually = Add client manually

statistics = Statistics

presets = Presets

settings = Settings
    .advanced-mode = Advanced
video = Video
video-resolution_dropdown = Video resolution
audio-microphone = Stream headset microphone
    .notice = Microphone streaming works only if VB-CABLE or VoiceMeeter is installed
headset = Headset
headset-controllers = Controllers
    .help = Allow the use of the controllers
headset-controllers-tracking_speed = Tracking speed
    .oculus_prediction = Oculus prediction
    .slow = Slow
    .medium = Medium
    .fast = Fast
connection = Connection
connection-stream_protocol = Stream protocol
    .Udp = UDP
    .ThrottledUdp = Throttled UDP
    .Tcp = TCP
connection-stream_protocol-ThrottledUdp-bitrate_multiplier = Bitrate multiplier

installation = Installation

logs = Logs

about = About

language = Language
    .prompt = Select a language


# Events

SessionSettingsExtrapolationFailed = 
    Failed to reconstruct the session. If you tried to import a preset it might be invalid.
ClientFoundInvalid = Error while identifying the client
ClientFoundWrongVersion = 
    The client that was trying to connect has version {$client-version}, which is incompatible with
    the current server version {$server-version}. Please update {$side ->
        [server] the server
        *[client] the client
    }.
