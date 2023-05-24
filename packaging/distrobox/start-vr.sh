#!/bin/bash

# credits to original script go to galister#4182

source ./helper-functions.sh

# go to installation folder in case we aren't already there
cd installation || echog "Already at needed folder"

STEAMVR_PATH="$HOME/.local/share/Steam/steamapps/common/SteamVR"

# add your tools here
run_additional_stuff() {
  echog Starting additional software
  ./alvr/usr/bin/alvr_dashboard &
  # WIP:
  #./SlimeVR-amd64.appimage &
  ./arch-alvr/oscalibrator-fork/OpenVR-SpaceCalibrator/openvr-spacecalibrator &
}

run_vrstartup() {
  "$STEAMVR_PATH/bin/vrstartup.sh" >/dev/null 2>&1 &
}

if pidof vrmonitor >/dev/null; then
  # we're started with vr already running so just start the extras
  run_additional_stuff
fi

trap 'echo SIGINT!; cleanup_alvr; exit 0' INT
trap 'echo SIGTERM!; cleanup_alvr; exit 0' TERM

while true; do

  if ! pidof vrmonitor >/dev/null; then
    cleanup_alvr

    run_vrstartup
    sleep 12
    run_additional_stuff
  fi

  sleep 1
done
