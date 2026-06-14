#pragma once

#include "ALVR-common/packet_types.h"
#include "bindings.h"
#include <string>

extern Settings g_settings;
extern bool g_settingsLoaded;

inline Settings& Settings_Instance() { return g_settings; }
inline bool Settings_isLoaded() { return g_settingsLoaded; }
void Settings_Load();
