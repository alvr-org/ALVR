#pragma once

#include <cstdint>
#include <vector>

extern uint64_t HEAD_PATH;
extern uint64_t LEFT_HAND_PATH;
extern uint64_t RIGHT_HAND_PATH;
extern uint64_t LEFT_CONTROLLER_HAPTIC_PATH;
extern uint64_t RIGHT_CONTROLLER_HAPTIC_PATH;

extern uint64_t OCULUS_CONTROLLER_PROFILE_PATH;
extern uint64_t INDEX_CONTROLLER_PROFILE_PATH;
extern uint64_t VIVE_CONTROLLER_PROFILE_PATH;

extern uint64_t MENU_CLICK;
extern uint64_t A_CLICK;
extern uint64_t A_TOUCH;
extern uint64_t B_CLICK;
extern uint64_t B_TOUCH;
extern uint64_t X_CLICK;
extern uint64_t X_TOUCH;
extern uint64_t Y_CLICK;
extern uint64_t Y_TOUCH;
extern uint64_t LEFT_SQUEEZE_CLICK;
extern uint64_t LEFT_SQUEEZE_VALUE;
extern uint64_t LEFT_TRIGGER_CLICK;
extern uint64_t LEFT_TRIGGER_VALUE;
extern uint64_t LEFT_TRIGGER_TOUCH;
extern uint64_t LEFT_THUMBSTICK_X;
extern uint64_t LEFT_THUMBSTICK_Y;
extern uint64_t LEFT_THUMBSTICK_CLICK;
extern uint64_t LEFT_THUMBSTICK_TOUCH;
extern uint64_t LEFT_THUMBREST_TOUCH;
extern uint64_t RIGHT_SQUEEZE_CLICK;
extern uint64_t RIGHT_SQUEEZE_VALUE;
extern uint64_t RIGHT_TRIGGER_CLICK;
extern uint64_t RIGHT_TRIGGER_VALUE;
extern uint64_t RIGHT_TRIGGER_TOUCH;
extern uint64_t RIGHT_THUMBSTICK_X;
extern uint64_t RIGHT_THUMBSTICK_Y;
extern uint64_t RIGHT_THUMBSTICK_CLICK;
extern uint64_t RIGHT_THUMBSTICK_TOUCH;
extern uint64_t RIGHT_THUMBREST_TOUCH;

extern std::vector<uint64_t> LEFT_CONTROLLER_BUTTONS;
extern std::vector<uint64_t> RIGHT_CONTROLLER_BUTTONS;

void init_paths();