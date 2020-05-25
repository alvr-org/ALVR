# For more information about using CMake with Android Studio, read the
# documentation: https://d.android.com/studio/projects/add-native-code.html

# Sets the minimum version of CMake required to build the native library.

cmake_minimum_required(VERSION 3.4.1)
set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY BOTH)

# Creates and names a library, sets it as either STATIC
# or SHARED, and provides the relative paths to its source code.
# You can define multiple libraries, and CMake builds them for you.
# Gradle automatically packages shared libraries with your APK.
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -std=c++17 -g")
set(CMAKE_CXX_FLAGS_DEBUG  "${CMAKE_CXX_FLAGS} -O3 -std=c++17 -g")
set(CMAKE_CXX_FLAGS_RELEASE  "${CMAKE_CXX_FLAGS} -O3 -std=c++17 -g")
set (ANDROID_STL c++_static)

link_directories(src/main/jniLibs/${ANDROID_ABI})

add_definitions(-DOVR_SDK)
set(VRLIBS "vrapi")
add_library( # Sets the name of the library.
             native-lib

             # Sets the library as a shared library.
             SHARED

             # Provides a relative path to your source file(s).
        src/main/cpp/ServerConnectionNative.cpp
             src/main/cpp/UdpSocket.cpp
             src/main/cpp/nal.cpp
             src/main/cpp/render.cpp
             src/main/cpp/latency_collector.cpp
             src/main/cpp/fec.cpp
             src/main/cpp/ffr.cpp
             src/main/cpp/asset.cpp
             src/main/cpp/gltf_model.cpp
             src/main/cpp/utils.cpp
             src/main/cpp/gl_render_utils/render_pipeline.cpp
             src/main/cpp/gl_render_utils/render_utils.cpp
             ../ALVR-common/reedsolomon/rs.c
             ../ALVR-common/common-utils.cpp
             ../ALVR-common/exception.cpp
             "src/main/cpp/ovr_context.cpp"
             )

# Searches for a specified prebuilt library and stores the path as a
# variable. Because CMake includes system libraries in the search path by
# default, you only need to specify the name of the public NDK library
# you want to add. CMake verifies that the library exists before
# completing its build.

find_library( # Sets the name of the path variable.
              log-lib


              # Specifies the name of the NDK library that
              # you want CMake to locate.
              log
)

find_library(OVR_LIBRARY
        libovrplatformloader.so
        PATHS src/main/jniLibs/${ANDROID_ABI})
if (NOT OVR_LIBRARY)
    message(FATAL_ERROR "OVR lib not found!")
endif()

# Specifies libraries CMake should link to your target library. You
# can link multiple libraries, such as libraries you define in this
# build script, prebuilt third-party libraries, or system libraries.

target_link_libraries( # Specifies the target library.
                       native-lib

                       # Links the target library to the log library
                       # included in the NDK.
                       ${log-lib}
                       ${VRLIBS}
                       GLESv3
                       EGL
                       android
                       OpenSLES
                         libovrplatformloader.so
        )

include_directories(include)
include_directories(../ALVR-common)