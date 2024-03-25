#include "Paths.h"
#include "bindings.h"

uint64_t HEAD_ID;
uint64_t HAND_LEFT_ID;
uint64_t HAND_RIGHT_ID;

std::map<uint64_t, ButtonInfo> LEFT_CONTROLLER_BUTTON_MAPPING;
std::map<uint64_t, ButtonInfo> RIGHT_CONTROLLER_BUTTON_MAPPING;
std::map<uint64_t, std::vector<uint64_t>> ALVR_TO_STEAMVR_PATH_IDS;

uint64_t LEFT_A_TOUCH_ID;
uint64_t LEFT_B_TOUCH_ID;
uint64_t LEFT_X_TOUCH_ID;
uint64_t LEFT_Y_TOUCH_ID;
uint64_t LEFT_TRACKPAD_TOUCH_ID;
uint64_t LEFT_THUMBSTICK_TOUCH_ID;
uint64_t LEFT_THUMBREST_TOUCH_ID;
uint64_t LEFT_TRIGGER_TOUCH_ID;
uint64_t LEFT_TRIGGER_VALUE_ID;
uint64_t LEFT_SQUEEZE_VALUE_ID;
uint64_t RIGHT_A_TOUCH_ID;
uint64_t RIGHT_B_TOUCH_ID;
uint64_t RIGHT_TRACKPAD_TOUCH_ID;
uint64_t RIGHT_THUMBSTICK_TOUCH_ID;
uint64_t RIGHT_THUMBREST_TOUCH_ID;
uint64_t RIGHT_TRIGGER_TOUCH_ID;
uint64_t RIGHT_TRIGGER_VALUE_ID;
uint64_t RIGHT_SQUEEZE_VALUE_ID;

void init_paths() {
    HEAD_ID = PathStringToHash("/user/head");
    HAND_LEFT_ID = PathStringToHash("/user/hand/left");
    HAND_RIGHT_ID = PathStringToHash("/user/hand/right");

    HEAD_ID = PathStringToHash("/user/head");
    HAND_LEFT_ID = PathStringToHash("/user/hand/left");
    HAND_RIGHT_ID = PathStringToHash("/user/hand/right");

    LEFT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/left/input/system/click"),
                                           {{"/input/system/click"}, ButtonType::Binary}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/left/input/system/touch"),
                                           {{"/input/system/touch"}, ButtonType::Binary}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert(
        {PathStringToHash("/user/hand/left/input/menu/click"),
         {{"/input/system/click", "/input/application_menu/click"}, ButtonType::Binary}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/left/input/a/click"),
                                           {{"/input/a/click"}, ButtonType::Binary}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/left/input/a/touch"),
                                           {{"/input/a/touch"}, ButtonType::Binary}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/left/input/b/click"),
                                           {{"/input/b/click"}, ButtonType::Binary}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/left/input/b/touch"),
                                           {{"/input/b/touch"}, ButtonType::Binary}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/left/input/x/click"),
                                           {{"/input/x/click"}, ButtonType::Binary}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/left/input/x/touch"),
                                           {{"/input/x/touch"}, ButtonType::Binary}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/left/input/y/click"),
                                           {{"/input/y/click"}, ButtonType::Binary}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/left/input/y/touch"),
                                           {{"/input/y/touch"}, ButtonType::Binary}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/left/input/squeeze/click"),
                                           {{"/input/grip/click"}, ButtonType::Binary}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/left/input/squeeze/touch"),
                                           {{"/input/grip/touch"}, ButtonType::Binary}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/left/input/squeeze/value"),
                                           {{"/input/grip/value"}, ButtonType::ScalarOneSided}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/left/input/squeeze/force"),
                                           {{"/input/grip/force"}, ButtonType::ScalarOneSided}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/left/input/trigger/click"),
                                           {{"/input/trigger/click"}, ButtonType::Binary}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/left/input/trigger/value"),
                                           {{"/input/trigger/value"}, ButtonType::ScalarOneSided}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/left/input/trigger/touch"),
                                           {{"/input/trigger/touch"}, ButtonType::Binary}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert(
        {PathStringToHash("/user/hand/left/input/thumbstick/x"),
         {{"/input/joystick/x", "/input/thumbstick/x"}, ButtonType::ScalarTwoSided}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert(
        {PathStringToHash("/user/hand/left/input/thumbstick/y"),
         {{"/input/joystick/y", "/input/thumbstick/y"}, ButtonType::ScalarTwoSided}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert(
        {PathStringToHash("/user/hand/left/input/thumbstick/click"),
         {{"/input/joystick/click", "/input/thumbstick/click"}, ButtonType::Binary}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert(
        {PathStringToHash("/user/hand/left/input/thumbstick/touch"),
         {{"/input/joystick/touch", "/input/thumbstick/touch"}, ButtonType::Binary}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/left/input/trackpad/x"),
                                           {{"/input/trackpad/x"}, ButtonType::ScalarTwoSided}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/left/input/trackpad/y"),
                                           {{"/input/trackpad/y"}, ButtonType::ScalarTwoSided}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/left/input/trackpad/click"),
                                           {{"/input/trackpad/click"}, ButtonType::Binary}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert(
        {PathStringToHash("/user/hand/left/input/trackpad/force"),
         {{"/input/trackpad/force"}, ButtonType::ScalarOneSided}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/left/input/trackpad/touch"),
                                           {{"/input/trackpad/touch"}, ButtonType::Binary}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert(
        {PathStringToHash("/user/hand/left/input/thumbrest/touch"),
         {{"/input/thumbrest/touch"}, ButtonType::Binary}});

    RIGHT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/right/input/system/click"),
                                            {{"/input/system/click"}, ButtonType::Binary}});
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/right/input/system/touch"),
                                            {{"/input/system/touch"}, ButtonType::Binary}});
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        {PathStringToHash("/user/hand/right/input/menu/click"),
         {{"/input/system/click", "/input/application_menu/click"}, ButtonType::Binary}});
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/right/input/a/click"),
                                            {{"/input/a/click"}, ButtonType::Binary}});
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/right/input/a/touch"),
                                            {{"/input/a/touch"}, ButtonType::Binary}});
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/right/input/b/click"),
                                            {{"/input/b/click"}, ButtonType::Binary}});
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/right/input/b/touch"),
                                            {{"/input/b/touch"}, ButtonType::Binary}});
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        {PathStringToHash("/user/hand/right/input/squeeze/click"),
         {{"/input/grip/click"}, ButtonType::Binary}});
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        {PathStringToHash("/user/hand/right/input/squeeze/touch"),
         {{"/input/grip/touch"}, ButtonType::Binary}});
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        {PathStringToHash("/user/hand/right/input/squeeze/value"),
         {{"/input/grip/value"}, ButtonType::ScalarOneSided}});
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        {PathStringToHash("/user/hand/right/input/squeeze/force"),
         {{"/input/grip/force"}, ButtonType::ScalarOneSided}});
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        {PathStringToHash("/user/hand/right/input/trigger/click"),
         {{"/input/trigger/click"}, ButtonType::Binary}});
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        {PathStringToHash("/user/hand/right/input/trigger/value"),
         {{"/input/trigger/value"}, ButtonType::ScalarOneSided}});
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        {PathStringToHash("/user/hand/right/input/trigger/touch"),
         {{"/input/trigger/touch"}, ButtonType::Binary}});
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        {PathStringToHash("/user/hand/right/input/thumbstick/x"),
         {{"/input/joystick/x", "/input/thumbstick/x"}, ButtonType::ScalarTwoSided}});
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        {PathStringToHash("/user/hand/right/input/thumbstick/y"),
         {{"/input/joystick/y", "/input/thumbstick/y"}, ButtonType::ScalarTwoSided}});
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        {PathStringToHash("/user/hand/right/input/thumbstick/click"),
         {{"/input/joystick/click", "/input/thumbstick/click"}, ButtonType::Binary}});
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        {PathStringToHash("/user/hand/right/input/thumbstick/touch"),
         {{"/input/joystick/touch", "/input/thumbstick/touch"}, ButtonType::Binary}});
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/right/input/trackpad/x"),
                                            {{"/input/trackpad/x"}, ButtonType::ScalarTwoSided}});
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert({PathStringToHash("/user/hand/right/input/trackpad/y"),
                                            {{"/input/trackpad/y"}, ButtonType::ScalarTwoSided}});
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        {PathStringToHash("/user/hand/right/input/trackpad/click"),
         {{"/input/trackpad/click"}, ButtonType::Binary}});
    LEFT_CONTROLLER_BUTTON_MAPPING.insert(
        {PathStringToHash("/user/hand/right/input/trackpad/force"),
         {{"/input/trackpad/force"}, ButtonType::ScalarOneSided}});
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        {PathStringToHash("/user/hand/right/input/trackpad/touch"),
         {{"/input/trackpad/touch"}, ButtonType::Binary}});
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        {PathStringToHash("/user/hand/right/input/thumbrest/touch"),
         {{"/input/thumbrest/touch"}, ButtonType::Binary}});

    for (auto hand : {LEFT_CONTROLLER_BUTTON_MAPPING, RIGHT_CONTROLLER_BUTTON_MAPPING}) {
        for (auto info : hand) {
            std::vector<uint64_t> ids;
            for (auto path : info.second.steamvr_paths) {
                ids.push_back(PathStringToHash(path));
            }
            ALVR_TO_STEAMVR_PATH_IDS.insert({info.first, ids});
        }
    }

    LEFT_A_TOUCH_ID = PathStringToHash("/user/hand/left/input/a/touch");
    LEFT_B_TOUCH_ID = PathStringToHash("/user/hand/left/input/b/touch");
    LEFT_X_TOUCH_ID = PathStringToHash("/user/hand/left/input/x/touch");
    LEFT_Y_TOUCH_ID = PathStringToHash("/user/hand/left/input/y/touch");
    LEFT_TRACKPAD_TOUCH_ID = PathStringToHash("/user/hand/left/input/trackpad/touch");
    LEFT_THUMBSTICK_TOUCH_ID = PathStringToHash("/user/hand/left/input/thumbstick/touch");
    LEFT_THUMBREST_TOUCH_ID = PathStringToHash("/user/hand/left/input/thumbrest/touch");
    LEFT_TRIGGER_TOUCH_ID = PathStringToHash("/user/hand/left/input/trigger/touch");
    LEFT_TRIGGER_VALUE_ID = PathStringToHash("/user/hand/left/input/trigger/value");
    LEFT_SQUEEZE_VALUE_ID = PathStringToHash("/user/hand/left/input/squeeze/value");
    RIGHT_A_TOUCH_ID = PathStringToHash("/user/hand/right/input/a/touch");
    RIGHT_B_TOUCH_ID = PathStringToHash("/user/hand/right/input/b/touch");
    RIGHT_TRACKPAD_TOUCH_ID = PathStringToHash("/user/hand/right/input/trackpad/touch");
    RIGHT_THUMBSTICK_TOUCH_ID = PathStringToHash("/user/hand/right/input/thumbstick/touch");
    RIGHT_THUMBREST_TOUCH_ID = PathStringToHash("/user/hand/right/input/thumbrest/touch");
    RIGHT_TRIGGER_TOUCH_ID = PathStringToHash("/user/hand/right/input/trigger/touch");
    RIGHT_TRIGGER_VALUE_ID = PathStringToHash("/user/hand/right/input/trigger/value");
    RIGHT_SQUEEZE_VALUE_ID = PathStringToHash("/user/hand/right/input/squeeze/value");
}