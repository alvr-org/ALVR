#include "Paths.h"
#include "bindings.h"

uint64_t HEAD_PATH;
uint64_t LEFT_HAND_PATH;
uint64_t RIGHT_HAND_PATH;
uint64_t LEFT_CONTROLLER_HAPTIC_PATH;
uint64_t RIGHT_CONTROLLER_HAPTIC_PATH;

uint64_t OCULUS_CONTROLLER_PROFILE_PATH;
uint64_t INDEX_CONTROLLER_PROFILE_PATH;
uint64_t VIVE_CONTROLLER_PROFILE_PATH;

uint64_t MENU_CLICK;
uint64_t A_CLICK;
uint64_t A_TOUCH;
uint64_t B_CLICK;
uint64_t B_TOUCH;
uint64_t X_CLICK;
uint64_t X_TOUCH;
uint64_t Y_CLICK;
uint64_t Y_TOUCH;
uint64_t LEFT_SQUEEZE_VALUE;
uint64_t LEFT_TRIGGER_VALUE;
uint64_t LEFT_TRIGGER_TOUCH;
uint64_t LEFT_THUMBSTICK_X;
uint64_t LEFT_THUMBSTICK_Y;
uint64_t LEFT_THUMBSTICK_CLICK;
uint64_t LEFT_THUMBSTICK_TOUCH;
uint64_t LEFT_THUMBREST_TOUCH;
uint64_t RIGHT_SQUEEZE_VALUE;
uint64_t RIGHT_TRIGGER_VALUE;
uint64_t RIGHT_TRIGGER_TOUCH;
uint64_t RIGHT_THUMBSTICK_X;
uint64_t RIGHT_THUMBSTICK_Y;
uint64_t RIGHT_THUMBSTICK_CLICK;
uint64_t RIGHT_THUMBSTICK_TOUCH;
uint64_t RIGHT_THUMBREST_TOUCH;

std::vector<uint64_t> LEFT_CONTROLLER_BUTTONS;
std::vector<uint64_t> RIGHT_CONTROLLER_BUTTONS;

void init_paths() {
    HEAD_PATH = PathStringToHash("/user/head");
    LEFT_HAND_PATH = PathStringToHash("/user/hand/left");
    RIGHT_HAND_PATH = PathStringToHash("/user/hand/right");
    LEFT_CONTROLLER_HAPTIC_PATH = PathStringToHash("/user/hand/left/output/haptic");
    RIGHT_CONTROLLER_HAPTIC_PATH = PathStringToHash("/user/hand/right/output/haptic");

    OCULUS_CONTROLLER_PROFILE_PATH =
        PathStringToHash("/interaction_profiles/oculus/touch_controller");
    INDEX_CONTROLLER_PROFILE_PATH =
        PathStringToHash("/interaction_profiles/valve/index_controller");
    VIVE_CONTROLLER_PROFILE_PATH = PathStringToHash("/interaction_profiles/htc/vive_controller");

    MENU_CLICK = PathStringToHash("/user/hand/left/input/menu/click");
    A_CLICK = PathStringToHash("/user/hand/right/input/a/click");
    A_TOUCH = PathStringToHash("/user/hand/right/input/a/touch");
    B_CLICK = PathStringToHash("/user/hand/right/input/b/click");
    B_TOUCH = PathStringToHash("/user/hand/right/input/b/touch");
    X_CLICK = PathStringToHash("/user/hand/left/input/x/click");
    X_TOUCH = PathStringToHash("/user/hand/left/input/x/touch");
    Y_CLICK = PathStringToHash("/user/hand/left/input/y/click");
    Y_TOUCH = PathStringToHash("/user/hand/left/input/y/touch");
    LEFT_SQUEEZE_VALUE = PathStringToHash("/user/hand/left/input/squeeze/value");
    LEFT_TRIGGER_VALUE = PathStringToHash("/user/hand/left/input/trigger/value");
    LEFT_TRIGGER_TOUCH = PathStringToHash("/user/hand/left/input/trigger/touch");
    LEFT_THUMBSTICK_X = PathStringToHash("/user/hand/left/input/thumbstick/x");
    LEFT_THUMBSTICK_Y = PathStringToHash("/user/hand/left/input/thumbstick/y");
    LEFT_THUMBSTICK_CLICK = PathStringToHash("/user/hand/left/input/thumbstick/click");
    LEFT_THUMBSTICK_TOUCH = PathStringToHash("/user/hand/left/input/thumbstick/touch");
    LEFT_THUMBREST_TOUCH = PathStringToHash("/user/hand/left/input/thumbrest/touch");
    RIGHT_SQUEEZE_VALUE = PathStringToHash("/user/hand/right/input/squeeze/value");
    RIGHT_TRIGGER_VALUE = PathStringToHash("/user/hand/right/input/trigger/value");
    RIGHT_TRIGGER_TOUCH = PathStringToHash("/user/hand/right/input/trigger/touch");
    RIGHT_THUMBSTICK_X = PathStringToHash("/user/hand/right/input/thumbstick/x");
    RIGHT_THUMBSTICK_Y = PathStringToHash("/user/hand/right/input/thumbstick/y");
    RIGHT_THUMBSTICK_CLICK = PathStringToHash("/user/hand/right/input/thumbstick/click");
    RIGHT_THUMBSTICK_TOUCH = PathStringToHash("/user/hand/right/input/thumbstick/touch");
    RIGHT_THUMBREST_TOUCH = PathStringToHash("/user/hand/right/input/thumbrest/touch");

    LEFT_CONTROLLER_BUTTONS.push_back(MENU_CLICK);
    LEFT_CONTROLLER_BUTTONS.push_back(X_CLICK);
    LEFT_CONTROLLER_BUTTONS.push_back(X_TOUCH);
    LEFT_CONTROLLER_BUTTONS.push_back(Y_CLICK);
    LEFT_CONTROLLER_BUTTONS.push_back(Y_TOUCH);
    LEFT_CONTROLLER_BUTTONS.push_back(LEFT_SQUEEZE_VALUE);
    LEFT_CONTROLLER_BUTTONS.push_back(LEFT_TRIGGER_VALUE);
    LEFT_CONTROLLER_BUTTONS.push_back(LEFT_TRIGGER_TOUCH);
    LEFT_CONTROLLER_BUTTONS.push_back(LEFT_THUMBSTICK_X);
    LEFT_CONTROLLER_BUTTONS.push_back(LEFT_THUMBSTICK_Y);
    LEFT_CONTROLLER_BUTTONS.push_back(LEFT_THUMBSTICK_CLICK);
    LEFT_CONTROLLER_BUTTONS.push_back(LEFT_THUMBSTICK_TOUCH);
    LEFT_CONTROLLER_BUTTONS.push_back(LEFT_THUMBREST_TOUCH);
    RIGHT_CONTROLLER_BUTTONS.push_back(A_CLICK);
    RIGHT_CONTROLLER_BUTTONS.push_back(A_TOUCH);
    RIGHT_CONTROLLER_BUTTONS.push_back(B_CLICK);
    RIGHT_CONTROLLER_BUTTONS.push_back(B_TOUCH);
    RIGHT_CONTROLLER_BUTTONS.push_back(RIGHT_SQUEEZE_VALUE);
    RIGHT_CONTROLLER_BUTTONS.push_back(RIGHT_TRIGGER_VALUE);
    RIGHT_CONTROLLER_BUTTONS.push_back(RIGHT_TRIGGER_TOUCH);
    RIGHT_CONTROLLER_BUTTONS.push_back(RIGHT_THUMBSTICK_X);
    RIGHT_CONTROLLER_BUTTONS.push_back(RIGHT_THUMBSTICK_Y);
    RIGHT_CONTROLLER_BUTTONS.push_back(RIGHT_THUMBSTICK_CLICK);
    RIGHT_CONTROLLER_BUTTONS.push_back(RIGHT_THUMBSTICK_TOUCH);
    RIGHT_CONTROLLER_BUTTONS.push_back(RIGHT_THUMBREST_TOUCH);
}