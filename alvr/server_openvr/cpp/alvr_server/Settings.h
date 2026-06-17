#pragma once

#include "ALVR-common/packet_types.h"
#include "bindings.h"
#include <string>

extern Settings g_settings;

inline Settings& Settings_Instance() { return g_settings; }
