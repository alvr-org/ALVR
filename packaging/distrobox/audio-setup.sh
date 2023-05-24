#!/bin/bash

function get_alvr_playback_sink_id() {
  local last_node_name=''
  local last_node_id=''
  pactl list sink-inputs | while read -r line; do
    node_id=$(echo "$line" | grep -oP 'Sink Input #\K.+' | sed -e 's/^[ \t]*//')
    node_name=$(echo "$line" | grep -oP 'node.name = "\K[^"]+' | sed -e 's/^[ \t]*//')
    if [[ "$node_id" != '' ]] && [[ "$last_node_id" != "$node_id" ]]; then
      last_node_id="$node_id"
    fi
    if [[ -n "$node_name" ]] && [[ "$last_node_name" != "$node_name" ]]; then
      last_node_name="$node_name"
      if [[ "$last_node_name" == "alsa_playback.vrserver" ]]; then
        echo "$last_node_id"
        return
      fi
    fi
  done
}

function get_alvr_sink_id() {
  pactl list short sinks | grep ALVR-MIC-Sink | cut -d$'\t' -f1
}

function setup_mic() {
  echo "Creating microphone sink & source and linking alvr playback to it"
  local sink_mic_id
  # This sink is required so that it persistently auto-connects to alvr playback later
  sink_mic_id=$(pactl load-module module-null-sink sink_name=ALVR-MIC-Sink media.class=Audio/Sink)
  local source_mic_id
  # This source is required so that any app can use it as microphone
  source_mic_id=$(pactl load-module module-null-sink sink_name=ALVR-MIC-Source media.class=Audio/Source/Virtual)
  # We link them together
  pw-link ALVR-MIC-Sink ALVR-MIC-Source
  # And we assign playback of pipewire alsa playback to created alvr sink
  pactl move-sink-input "$(get_alvr_playback_sink_id)" "$(get_alvr_sink_id)"
  echo "$sink_mic_id|$source_mic_id" | tee "/tmp/alvr-mic-ids"
}

function unload_mic() {
  echo "Unloading microphone sink & source"
  local sink_mic_id
  sink_mic_id="$(cut -d '|' -f1 /tmp/alvr-mic-ids)"
  local source_mic_id
  source_mic_id="$(cut -d '|' -f2 /tmp/alvr-mic-ids)"
  pactl unload-module "$sink_mic_id"
  pactl unload-module "$source_mic_id"
}

case $ACTION in
	connect)
		pactl set-sink-mute @DEFAULT_SINK@ 1
		sleep 1
		setup_mic
		;;
        disconnect)
                pactl set-sink-mute @DEFAULT_SINK@ 0
		unload_mic;;
esac
