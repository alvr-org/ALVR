#!/bin/sh -e

export ADB_VENDOR_KEYS=~/.android/adbkey.pub
flatpak override --user --filesystem=~/.android com.valvesoftware.Steam.Utility.alvr
flatpak run --env=ADB_VENDOR_KEYS=$ADB_VENDOR_KEYS --env=QT_QPA_PLATFORM=xcb --command=alvr_launcher com.valvesoftware.Steam