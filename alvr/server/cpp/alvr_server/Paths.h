#pragma once

#include <cstdint>
#include <vector>

extern uint64_t HEAD_ID;
extern uint64_t LEFT_HAND_ID;
extern uint64_t RIGHT_HAND_ID;

extern uint64_t MENU_CLICK_ID;
extern uint64_t A_CLICK_ID;
extern uint64_t A_TOUCH_ID;
extern uint64_t B_CLICK_ID;
extern uint64_t B_TOUCH_ID;
extern uint64_t X_CLICK_ID;
extern uint64_t X_TOUCH_ID;
extern uint64_t Y_CLICK_ID;
extern uint64_t Y_TOUCH_ID;
extern uint64_t LEFT_SQUEEZE_CLICK_ID;
extern uint64_t LEFT_SQUEEZE_VALUE_ID;
extern uint64_t LEFT_TRIGGER_CLICK_ID;
extern uint64_t LEFT_TRIGGER_VALUE_ID;
extern uint64_t LEFT_TRIGGER_TOUCH_ID;
extern uint64_t LEFT_THUMBSTICK_X_ID;
extern uint64_t LEFT_THUMBSTICK_Y_ID;
extern uint64_t LEFT_THUMBSTICK_CLICK_ID;
extern uint64_t LEFT_THUMBSTICK_TOUCH_ID;
extern uint64_t LEFT_THUMBREST_TOUCH_ID;
extern uint64_t RIGHT_SQUEEZE_CLICK_ID;
extern uint64_t RIGHT_SQUEEZE_VALUE_ID;
extern uint64_t RIGHT_TRIGGER_CLICK_ID;
extern uint64_t RIGHT_TRIGGER_VALUE_ID;
extern uint64_t RIGHT_TRIGGER_TOUCH_ID;
extern uint64_t RIGHT_THUMBSTICK_X_ID;
extern uint64_t RIGHT_THUMBSTICK_Y_ID;
extern uint64_t RIGHT_THUMBSTICK_CLICK_ID;
extern uint64_t RIGHT_THUMBSTICK_TOUCH_ID;
extern uint64_t RIGHT_THUMBREST_TOUCH_ID;

extern std::vector<uint64_t> LEFT_CONTROLLER_BUTTON_IDS;
extern std::vector<uint64_t> RIGHT_CONTROLLER_BUTTON_IDS;

void init_paths();