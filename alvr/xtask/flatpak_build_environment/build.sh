#!/usr/bin/env bash

flatpak run org.flatpak.Builder --user --install --force-clean .flatpak-build-dir com.valvesoftware.Steam.Utility.alvr_builder.json
