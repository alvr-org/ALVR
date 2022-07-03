cmake_minimum_required(VERSION 3.4.1)
set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY BOTH)

set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -std=c++17 -g")
set(CMAKE_CXX_FLAGS_DEBUG "${CMAKE_CXX_FLAGS} -O3 -std=c++17 -g")
set(CMAKE_CXX_FLAGS_RELEASE "${CMAKE_CXX_FLAGS} -O3 -std=c++17 -g")
set(ANDROID_STL c++_static)

link_directories(src/main/jniLibs/${ANDROID_ABI})
link_directories(../../build/alvr_client_core/${ANDROID_ABI})

add_library(native_lib SHARED
        src/main/cpp/cpp_main.cpp
        )

include_directories(src/main/cpp)
include_directories(../../build/alvr_client_core)

target_link_libraries(
        native_lib
        log
        GLESv3
        EGL
        android
        vrapi
        alvr_client_core
        )
