#include "paths.h"
#include "bindings.h"

uint64_t HEAD_PATH;
uint64_t LEFT_HAND_PATH;
uint64_t RIGHT_HAND_PATH;

uint64_t OCULUS_CONTROLLER_PROFILE_PATH;
uint64_t INDEX_CONTROLLER_PROFILE_PATH;
uint64_t VIVE_CONTROLLER_PROFILE_PATH;

void init_paths() {
    HEAD_PATH = PathStringToHash("/user/head");
    LEFT_HAND_PATH = PathStringToHash("/user/hand/left");
    RIGHT_HAND_PATH = PathStringToHash("/user/hand/right");

    OCULUS_CONTROLLER_PROFILE_PATH =
        PathStringToHash("/interaction_profiles/oculus/touch_controller");
    INDEX_CONTROLLER_PROFILE_PATH =
        PathStringToHash("/interaction_profiles/valve/index_controller");
    VIVE_CONTROLLER_PROFILE_PATH = PathStringToHash("/interaction_profiles/htc/vive_controller");
}