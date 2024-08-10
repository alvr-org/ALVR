#!/bin/sh -e
flatpak-builder --user --install --force-clean build-dir com.valvesoftware.Steam.Utility.alvr.json
#flatpak --user remote-add --no-gpg-verify --if-not-exists steam steam
#flatpak install com.valvesoftware.Steam.Utility.alvr
#flatpak update com.valvesoftware.Steam.Utility.alvr
