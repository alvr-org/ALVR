#include "Paths.h"
#include "bindings.h"

uint64_t HEAD_ID;
uint64_t LEFT_HAND_ID;
uint64_t RIGHT_HAND_ID;

uint64_t MENU_CLICK_ID;
uint64_t A_CLICK_ID;
uint64_t A_TOUCH_ID;
uint64_t B_CLICK_ID;
uint64_t B_TOUCH_ID;
uint64_t X_CLICK_ID;
uint64_t X_TOUCH_ID;
uint64_t Y_CLICK_ID;
uint64_t Y_TOUCH_ID;
uint64_t LEFT_SQUEEZE_CLICK_ID;
uint64_t LEFT_SQUEEZE_VALUE_ID;
uint64_t LEFT_TRIGGER_CLICK_ID;
uint64_t LEFT_TRIGGER_VALUE_ID;
uint64_t LEFT_TRIGGER_TOUCH_ID;
uint64_t LEFT_THUMBSTICK_X_ID;
uint64_t LEFT_THUMBSTICK_Y_ID;
uint64_t LEFT_THUMBSTICK_CLICK_ID;
uint64_t LEFT_THUMBSTICK_TOUCH_ID;
uint64_t LEFT_THUMBREST_TOUCH_ID;
uint64_t RIGHT_SQUEEZE_CLICK_ID;
uint64_t RIGHT_SQUEEZE_VALUE_ID;
uint64_t RIGHT_TRIGGER_CLICK_ID;
uint64_t RIGHT_TRIGGER_VALUE_ID;
uint64_t RIGHT_TRIGGER_TOUCH_ID;
uint64_t RIGHT_THUMBSTICK_X_ID;
uint64_t RIGHT_THUMBSTICK_Y_ID;
uint64_t RIGHT_THUMBSTICK_CLICK_ID;
uint64_t RIGHT_THUMBSTICK_TOUCH_ID;
uint64_t RIGHT_THUMBREST_TOUCH_ID;

std::vector<uint64_t> LEFT_CONTROLLER_BUTTON_IDS;
std::vector<uint64_t> RIGHT_CONTROLLER_BUTTON_IDS;

void init_paths() {
    HEAD_ID = PathStringToHash("/user/head");
    LEFT_HAND_ID = PathStringToHash("/user/hand/left");
    RIGHT_HAND_ID = PathStringToHash("/user/hand/right");

    MENU_CLICK_ID = PathStringToHash("/user/hand/left/input/menu/click");
    A_CLICK_ID = PathStringToHash("/user/hand/right/input/a/click");
    A_TOUCH_ID = PathStringToHash("/user/hand/right/input/a/touch");
    B_CLICK_ID = PathStringToHash("/user/hand/right/input/b/click");
    B_TOUCH_ID = PathStringToHash("/user/hand/right/input/b/touch");
    X_CLICK_ID = PathStringToHash("/user/hand/left/input/x/click");
    X_TOUCH_ID = PathStringToHash("/user/hand/left/input/x/touch");
    Y_CLICK_ID = PathStringToHash("/user/hand/left/input/y/click");
    Y_TOUCH_ID = PathStringToHash("/user/hand/left/input/y/touch");
    LEFT_SQUEEZE_CLICK_ID = PathStringToHash("/user/hand/left/input/squeeze/click");
    LEFT_SQUEEZE_VALUE_ID = PathStringToHash("/user/hand/left/input/squeeze/value");
    LEFT_TRIGGER_CLICK_ID = PathStringToHash("/user/hand/left/input/trigger/click");
    LEFT_TRIGGER_VALUE_ID = PathStringToHash("/user/hand/left/input/trigger/value");
    LEFT_TRIGGER_TOUCH_ID = PathStringToHash("/user/hand/left/input/trigger/touch");
    LEFT_THUMBSTICK_X_ID = PathStringToHash("/user/hand/left/input/thumbstick/x");
    LEFT_THUMBSTICK_Y_ID = PathStringToHash("/user/hand/left/input/thumbstick/y");
    LEFT_THUMBSTICK_CLICK_ID = PathStringToHash("/user/hand/left/input/thumbstick/click");
    LEFT_THUMBSTICK_TOUCH_ID = PathStringToHash("/user/hand/left/input/thumbstick/touch");
    LEFT_THUMBREST_TOUCH_ID = PathStringToHash("/user/hand/left/input/thumbrest/touch");
    RIGHT_SQUEEZE_CLICK_ID = PathStringToHash("/user/hand/right/input/squeeze/click");
    RIGHT_SQUEEZE_VALUE_ID = PathStringToHash("/user/hand/right/input/squeeze/value");
    RIGHT_TRIGGER_CLICK_ID = PathStringToHash("/user/hand/right/input/trigger/click");
    RIGHT_TRIGGER_VALUE_ID = PathStringToHash("/user/hand/right/input/trigger/value");
    RIGHT_TRIGGER_TOUCH_ID = PathStringToHash("/user/hand/right/input/trigger/touch");
    RIGHT_THUMBSTICK_X_ID = PathStringToHash("/user/hand/right/input/thumbstick/x");
    RIGHT_THUMBSTICK_Y_ID = PathStringToHash("/user/hand/right/input/thumbstick/y");
    RIGHT_THUMBSTICK_CLICK_ID = PathStringToHash("/user/hand/right/input/thumbstick/click");
    RIGHT_THUMBSTICK_TOUCH_ID = PathStringToHash("/user/hand/right/input/thumbstick/touch");
    RIGHT_THUMBREST_TOUCH_ID = PathStringToHash("/user/hand/right/input/thumbrest/touch");

    LEFT_CONTROLLER_BUTTON_IDS.push_back(MENU_CLICK_ID);
    LEFT_CONTROLLER_BUTTON_IDS.push_back(X_CLICK_ID);
    LEFT_CONTROLLER_BUTTON_IDS.push_back(X_TOUCH_ID);
    LEFT_CONTROLLER_BUTTON_IDS.push_back(Y_CLICK_ID);
    LEFT_CONTROLLER_BUTTON_IDS.push_back(Y_TOUCH_ID);
    LEFT_CONTROLLER_BUTTON_IDS.push_back(LEFT_SQUEEZE_CLICK_ID);
    LEFT_CONTROLLER_BUTTON_IDS.push_back(LEFT_SQUEEZE_VALUE_ID);
    LEFT_CONTROLLER_BUTTON_IDS.push_back(LEFT_TRIGGER_CLICK_ID);
    LEFT_CONTROLLER_BUTTON_IDS.push_back(LEFT_TRIGGER_VALUE_ID);
    LEFT_CONTROLLER_BUTTON_IDS.push_back(LEFT_TRIGGER_TOUCH_ID);
    LEFT_CONTROLLER_BUTTON_IDS.push_back(LEFT_THUMBSTICK_X_ID);
    LEFT_CONTROLLER_BUTTON_IDS.push_back(LEFT_THUMBSTICK_Y_ID);
    LEFT_CONTROLLER_BUTTON_IDS.push_back(LEFT_THUMBSTICK_CLICK_ID);
    LEFT_CONTROLLER_BUTTON_IDS.push_back(LEFT_THUMBSTICK_TOUCH_ID);
    LEFT_CONTROLLER_BUTTON_IDS.push_back(LEFT_THUMBREST_TOUCH_ID);
    RIGHT_CONTROLLER_BUTTON_IDS.push_back(A_CLICK_ID);
    RIGHT_CONTROLLER_BUTTON_IDS.push_back(A_TOUCH_ID);
    RIGHT_CONTROLLER_BUTTON_IDS.push_back(B_CLICK_ID);
    RIGHT_CONTROLLER_BUTTON_IDS.push_back(B_TOUCH_ID);
    RIGHT_CONTROLLER_BUTTON_IDS.push_back(RIGHT_SQUEEZE_CLICK_ID);
    RIGHT_CONTROLLER_BUTTON_IDS.push_back(RIGHT_SQUEEZE_VALUE_ID);
    RIGHT_CONTROLLER_BUTTON_IDS.push_back(RIGHT_TRIGGER_CLICK_ID);
    RIGHT_CONTROLLER_BUTTON_IDS.push_back(RIGHT_TRIGGER_VALUE_ID);
    RIGHT_CONTROLLER_BUTTON_IDS.push_back(RIGHT_TRIGGER_TOUCH_ID);
    RIGHT_CONTROLLER_BUTTON_IDS.push_back(RIGHT_THUMBSTICK_X_ID);
    RIGHT_CONTROLLER_BUTTON_IDS.push_back(RIGHT_THUMBSTICK_Y_ID);
    RIGHT_CONTROLLER_BUTTON_IDS.push_back(RIGHT_THUMBSTICK_CLICK_ID);
    RIGHT_CONTROLLER_BUTTON_IDS.push_back(RIGHT_THUMBSTICK_TOUCH_ID);
    RIGHT_CONTROLLER_BUTTON_IDS.push_back(RIGHT_THUMBREST_TOUCH_ID);
}