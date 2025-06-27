#include "Paths.h"
#include "bindings.h"

uint64_t HEAD_ID;
uint64_t HAND_LEFT_ID;
uint64_t HAND_RIGHT_ID;
uint64_t HAND_TRACKER_LEFT_ID;
uint64_t HAND_TRACKER_RIGHT_ID;
uint64_t BODY_CHEST_ID;
uint64_t BODY_HIPS_ID;
uint64_t BODY_LEFT_ELBOW_ID;
uint64_t BODY_RIGHT_ELBOW_ID;
uint64_t BODY_LEFT_KNEE_ID;
uint64_t BODY_LEFT_FOOT_ID;
uint64_t BODY_RIGHT_KNEE_ID;
uint64_t BODY_RIGHT_FOOT_ID;

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

std::set<uint64_t> BODY_IDS;
std::map<uint64_t, ButtonInfo> LEFT_CONTROLLER_BUTTON_MAPPING;
std::map<uint64_t, ButtonInfo> RIGHT_CONTROLLER_BUTTON_MAPPING;
std::map<uint64_t, std::vector<uint64_t>> ALVR_TO_STEAMVR_PATH_IDS;

void init_paths() {
    HEAD_ID = PathStringToHash("/user/head");
    HAND_LEFT_ID = PathStringToHash("/user/hand/left");
    HAND_RIGHT_ID = PathStringToHash("/user/hand/right");
    HAND_TRACKER_LEFT_ID = PathStringToHash("/user/hand_tracker/left");
    HAND_TRACKER_RIGHT_ID = PathStringToHash("/user/hand_tracker/right");
    BODY_CHEST_ID = PathStringToHash("/user/body/chest");
    BODY_HIPS_ID = PathStringToHash("/user/body/waist");
    BODY_LEFT_ELBOW_ID = PathStringToHash("/user/body/left_elbow");
    BODY_RIGHT_ELBOW_ID = PathStringToHash("/user/body/right_elbow");
    BODY_LEFT_KNEE_ID = PathStringToHash("/user/body/left_knee");
    BODY_LEFT_FOOT_ID = PathStringToHash("/user/body/left_foot");
    BODY_RIGHT_KNEE_ID = PathStringToHash("/user/body/right_knee");
    BODY_RIGHT_FOOT_ID = PathStringToHash("/user/body/right_foot");

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

    BODY_IDS.insert(BODY_CHEST_ID);
    BODY_IDS.insert(BODY_HIPS_ID);
    BODY_IDS.insert(BODY_LEFT_ELBOW_ID);
    BODY_IDS.insert(BODY_RIGHT_ELBOW_ID);
    BODY_IDS.insert(BODY_LEFT_KNEE_ID);
    BODY_IDS.insert(BODY_LEFT_FOOT_ID);
    BODY_IDS.insert(BODY_RIGHT_KNEE_ID);
    BODY_IDS.insert(BODY_RIGHT_FOOT_ID);

    LEFT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/left/input/system/click"),
                                            { { "/input/system/click", "/input/left_ps/click" },
                                              ButtonType::Binary } });
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/left/input/system/touch"),
                                            { { "/input/system/touch", "/input/left_ps/touch" },
                                              ButtonType::Binary } });
    LEFT_CONTROLLER_BUTTON_MAPPING.insert(
        { PathStringToHash("/user/hand/left/input/menu/click"),
          { { "/input/system/click", "/input/application_menu/click", "/input/create/click" },
            ButtonType::Binary } }
    );
    LEFT_CONTROLLER_BUTTON_MAPPING.insert(
        { PathStringToHash("/user/hand/left/input/menu/touch"),
          { { "/input/system/touch", "/input/application_menu/touch", "/input/create/touch" },
            ButtonType::Binary } }
    );
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/left/input/a/click"),
                                            { { "/input/a/click", "/input/cross/click" },
                                              ButtonType::Binary } });
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/left/input/a/touch"),
                                            { { "/input/a/touch", "/input/cross/touch" },
                                              ButtonType::Binary } });
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/left/input/b/click"),
                                            { { "/input/b/click", "/input/circle/click" },
                                              ButtonType::Binary } });
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/left/input/b/touch"),
                                            { { "/input/b/touch", "/input/circle/touch" },
                                              ButtonType::Binary } });
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/left/input/x/click"),
                                            { { "/input/x/click", "/input/square/click" },
                                              ButtonType::Binary } });
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/left/input/x/touch"),
                                            { { "/input/x/touch", "/input/square/touch" },
                                              ButtonType::Binary } });
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/left/input/y/click"),
                                            { { "/input/y/click", "/input/triangle/click" },
                                              ButtonType::Binary } });
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/left/input/y/touch"),
                                            { { "/input/y/touch", "/input/triangle/touch" },
                                              ButtonType::Binary } });
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/left/input/squeeze/click"),
                                            { { "/input/grip/click", "/input/l1/click" },
                                              ButtonType::Binary } });
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/left/input/squeeze/touch"),
                                            { { "/input/grip/touch", "/input/l1/touch" },
                                              ButtonType::Binary } });
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/left/input/squeeze/value"),
                                            { { "/input/grip/value", "/input/l1/value" },
                                              ButtonType::ScalarOneSided } });
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/left/input/squeeze/force"),
                                            { { "/input/grip/force" },
                                              ButtonType::ScalarOneSided } });
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/left/input/trigger/click"),
                                            { { "/input/trigger/click", "/input/l2/click" },
                                              ButtonType::Binary } });
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/left/input/trigger/value"),
                                            { { "/input/trigger/value", "/input/l2/value" },
                                              ButtonType::ScalarOneSided } });
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/left/input/trigger/touch"),
                                            { { "/input/trigger/touch", "/input/l2/touch" },
                                              ButtonType::Binary } });
    LEFT_CONTROLLER_BUTTON_MAPPING.insert(
        { PathStringToHash("/user/hand/left/input/thumbstick/x"),
          { { "/input/joystick/x", "/input/thumbstick/x", "/input/left_stick/x" },
            ButtonType::ScalarTwoSided } }
    );
    LEFT_CONTROLLER_BUTTON_MAPPING.insert(
        { PathStringToHash("/user/hand/left/input/thumbstick/y"),
          { { "/input/joystick/y", "/input/thumbstick/y", "/input/left_stick/y" },
            ButtonType::ScalarTwoSided } }
    );
    LEFT_CONTROLLER_BUTTON_MAPPING.insert(
        { PathStringToHash("/user/hand/left/input/thumbstick/click"),
          { { "/input/joystick/click", "/input/thumbstick/click", "/input/left_stick/click" },
            ButtonType::Binary } }
    );
    LEFT_CONTROLLER_BUTTON_MAPPING.insert(
        { PathStringToHash("/user/hand/left/input/thumbstick/touch"),
          { { "/input/joystick/touch", "/input/thumbstick/touch", "/input/left_stick/touch" },
            ButtonType::Binary } }
    );
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/left/input/trackpad/x"),
                                            { { "/input/trackpad/x" },
                                              ButtonType::ScalarTwoSided } });
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/left/input/trackpad/y"),
                                            { { "/input/trackpad/y" },
                                              ButtonType::ScalarTwoSided } });
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/left/input/trackpad/click"
                                            ),
                                            { { "/input/trackpad/click" }, ButtonType::Binary } });
    LEFT_CONTROLLER_BUTTON_MAPPING.insert(
        { PathStringToHash("/user/hand/left/input/trackpad/force"),
          { { "/input/trackpad/force" }, ButtonType::ScalarOneSided } }
    );
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/left/input/trackpad/touch"
                                            ),
                                            { { "/input/trackpad/touch" }, ButtonType::Binary } });
    LEFT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/left/input/thumbrest/touch"
                                            ),
                                            { { "/input/thumbrest/touch" }, ButtonType::Binary } });

    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        { PathStringToHash("/user/hand/right/input/system/click"),
          { { "/input/system/click", "/input/right_ps/click" }, ButtonType::Binary } }
    );
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        { PathStringToHash("/user/hand/right/input/system/touch"),
          { { "/input/system/touch", "/input/right_ps/touch" }, ButtonType::Binary } }
    );
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        { PathStringToHash("/user/hand/right/input/menu/click"),
          { { "/input/system/click", "/input/application_menu/click", "/input/options/click" },
            ButtonType::Binary } }
    );
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        { PathStringToHash("/user/hand/right/input/menu/touch"),
          { { "/input/system/touch", "/input/application_menu/touch", "/input/options/touch" },
            ButtonType::Binary } }
    );
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/right/input/a/click"),
                                             { { "/input/a/click", "/input/cross/click" },
                                               ButtonType::Binary } });
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/right/input/a/touch"),
                                             { { "/input/a/touch", "/input/cross/touch" },
                                               ButtonType::Binary } });
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/right/input/b/click"),
                                             { { "/input/b/click", "/input/circle/click" },
                                               ButtonType::Binary } });
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/right/input/b/touch"),
                                             { { "/input/b/touch", "/input/circle/touch" },
                                               ButtonType::Binary } });
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        { PathStringToHash("/user/hand/right/input/squeeze/click"),
          { { "/input/grip/click", "/input/r1/click" }, ButtonType::Binary } }
    );
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        { PathStringToHash("/user/hand/right/input/squeeze/touch"),
          { { "/input/grip/touch", "/input/r1/touch" }, ButtonType::Binary } }
    );
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        { PathStringToHash("/user/hand/right/input/squeeze/value"),
          { { "/input/grip/value", "/input/r1/value" }, ButtonType::ScalarOneSided } }
    );
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        { PathStringToHash("/user/hand/right/input/squeeze/force"),
          { { "/input/grip/force" }, ButtonType::ScalarOneSided } }
    );
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        { PathStringToHash("/user/hand/right/input/trigger/click"),
          { { "/input/trigger/click", "/input/r2/click" }, ButtonType::Binary } }
    );
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        { PathStringToHash("/user/hand/right/input/trigger/value"),
          { { "/input/trigger/value", "/input/r2/value" }, ButtonType::ScalarOneSided } }
    );
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        { PathStringToHash("/user/hand/right/input/trigger/touch"),
          { { "/input/trigger/touch", "/input/r2/touch" }, ButtonType::Binary } }
    );
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        { PathStringToHash("/user/hand/right/input/thumbstick/x"),
          { { "/input/joystick/x", "/input/thumbstick/x", "/input/right_stick/x" },
            ButtonType::ScalarTwoSided } }
    );
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        { PathStringToHash("/user/hand/right/input/thumbstick/y"),
          { { "/input/joystick/y", "/input/thumbstick/y", "/input/right_stick/y" },
            ButtonType::ScalarTwoSided } }
    );
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        { PathStringToHash("/user/hand/right/input/thumbstick/click"),
          { { "/input/joystick/click", "/input/thumbstick/click", "/input/right_stick/click" },
            ButtonType::Binary } }
    );
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        { PathStringToHash("/user/hand/right/input/thumbstick/touch"),
          { { "/input/joystick/touch", "/input/thumbstick/touch", "/input/right_stick/touch" },
            ButtonType::Binary } }
    );
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/right/input/trackpad/x"),
                                             { { "/input/trackpad/x" },
                                               ButtonType::ScalarTwoSided } });
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert({ PathStringToHash("/user/hand/right/input/trackpad/y"),
                                             { { "/input/trackpad/y" },
                                               ButtonType::ScalarTwoSided } });
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        { PathStringToHash("/user/hand/right/input/trackpad/click"),
          { { "/input/trackpad/click" }, ButtonType::Binary } }
    );
    LEFT_CONTROLLER_BUTTON_MAPPING.insert(
        { PathStringToHash("/user/hand/right/input/trackpad/force"),
          { { "/input/trackpad/force" }, ButtonType::ScalarOneSided } }
    );
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        { PathStringToHash("/user/hand/right/input/trackpad/touch"),
          { { "/input/trackpad/touch" }, ButtonType::Binary } }
    );
    RIGHT_CONTROLLER_BUTTON_MAPPING.insert(
        { PathStringToHash("/user/hand/right/input/thumbrest/touch"),
          { { "/input/thumbrest/touch" }, ButtonType::Binary } }
    );

    for (auto hand : { LEFT_CONTROLLER_BUTTON_MAPPING, RIGHT_CONTROLLER_BUTTON_MAPPING }) {
        for (auto info : hand) {
            std::vector<uint64_t> ids;
            for (auto path : info.second.steamvr_paths) {
                ids.push_back(PathStringToHash(path));
            }
            ALVR_TO_STEAMVR_PATH_IDS.insert({ info.first, ids });
        }
    }
}
