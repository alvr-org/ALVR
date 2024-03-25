#pragma once

#include <cstdint>
#include <vector>
#include <map>

#include "openvr_driver.h"

extern uint64_t HEAD_ID;
extern uint64_t HAND_LEFT_ID;
extern uint64_t HAND_RIGHT_ID;

enum class ButtonType {
    Binary,
    ScalarOneSided,
    ScalarTwoSided,
};

struct ButtonInfo {
    std::vector<const char *> steamvr_paths;
    ButtonType type;
};

// Map button ID to SteamVR button info
extern std::map<uint64_t, ButtonInfo> LEFT_CONTROLLER_BUTTON_MAPPING;
extern std::map<uint64_t, ButtonInfo> RIGHT_CONTROLLER_BUTTON_MAPPING;
extern std::map<uint64_t, std::vector<uint64_t>> ALVR_TO_STEAMVR_PATH_IDS;

void init_paths();

// These values are needed to determine the hand skeleton when holding a controller.
// todo: move inferred hand skeleton to rust
extern uint64_t LEFT_A_TOUCH_ID;
extern uint64_t LEFT_B_TOUCH_ID;
extern uint64_t LEFT_X_TOUCH_ID;
extern uint64_t LEFT_Y_TOUCH_ID;
extern uint64_t LEFT_TRACKPAD_TOUCH_ID;
extern uint64_t LEFT_THUMBSTICK_TOUCH_ID;
extern uint64_t LEFT_THUMBREST_TOUCH_ID;
extern uint64_t LEFT_TRIGGER_TOUCH_ID;
extern uint64_t LEFT_TRIGGER_VALUE_ID;
extern uint64_t LEFT_SQUEEZE_TOUCH_ID;
extern uint64_t LEFT_SQUEEZE_VALUE_ID;
extern uint64_t RIGHT_A_TOUCH_ID;
extern uint64_t RIGHT_B_TOUCH_ID;
extern uint64_t RIGHT_TRACKPAD_TOUCH_ID;
extern uint64_t RIGHT_THUMBSTICK_TOUCH_ID;
extern uint64_t RIGHT_THUMBREST_TOUCH_ID;
extern uint64_t RIGHT_TRIGGER_TOUCH_ID;
extern uint64_t RIGHT_TRIGGER_VALUE_ID;
extern uint64_t RIGHT_SQUEEZE_TOUCH_ID;
extern uint64_t RIGHT_SQUEEZE_VALUE_ID;