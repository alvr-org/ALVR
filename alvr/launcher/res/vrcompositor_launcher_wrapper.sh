#!/usr/bin/env sh

export VK_LAYER_PATH=$(cat $XDG_RUNTIME_DIR/alvr_dir.txt)/share/vulkan/explicit_layer.d
export VK_INSTANCE_LAYERS=VK_LAYER_ALVR_capture
export DISABLE_VK_LAYER_VALVE_steam_fossilize_1=1

exec "$0".real "$@"
