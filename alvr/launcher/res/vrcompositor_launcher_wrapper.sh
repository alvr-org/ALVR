#!/usr/bin/env sh

export VK_LAYER_PATH=$(cat $XDG_RUNTIME_DIR/alvr_dir.txt | rev | cut -d'/' -f3- | rev)/build/vulkan-layer
export VK_INSTANCE_LAYERS=VK_LAYER_ALVR_capture
export DISABLE_VK_LAYER_VALVE_steam_fossilize_1=1
export DISABLE_VK_LAYER_VALVE_steam_overlay_1=1

exec "$0".real "$@"
