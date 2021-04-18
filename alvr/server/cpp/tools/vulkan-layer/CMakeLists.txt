# Copyright (c) 2019-2021 Arm Limited.
#
# SPDX-License-Identifier: MIT
#
# Permission is hereby granted, free of charge, to any person obtaining a copy
# of this software and associated documentation files (the "Software"), to
# deal in the Software without restriction, including without limitation the
# rights to use, copy, modify, merge, publish, distribute, sublicense, and/or
# sell copies of the Software, and to permit persons to whom the Software is
# furnished to do so, subject to the following conditions:
#
# The above copyright notice and this permission notice shall be included in all
# copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
# FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
# AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
# LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
# OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
# SOFTWARE.

cmake_minimum_required(VERSION 2.8.11)
project(VkLayer_window_system_integration CXX)

find_package(PkgConfig REQUIRED)
pkg_check_modules(VULKAN_PKG_CONFIG vulkan)

pkg_check_modules(libunwind REQUIRED IMPORTED_TARGET libunwind)

set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -pthread")
if (DEFINED DEBUG)
   set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -g -O0")
endif()

set(CMAKE_SHARED_LINKER_FLAGS "${CMAKE_SHARED_LINKER_FLAGS} -Wl,--no-undefined")

if(NOT DEFINED VULKAN_CXX_INCLUDE)
   set(VULKAN_CXX_INCLUDE ${VULKAN_PKG_CONFIG_INCLUDEDIR})
endif()

if(DEFINED VULKAN_CXX_INCLUDE)
   message(STATUS "Using Vulkan include directories: ${VULKAN_CXX_INCLUDE}")
   separate_arguments(VULKAN_CXX_INCLUDE)
else()
   message(FATAL_ERROR "Either vulkan.pc must be available or VULKAN_CXX_INCLUDE must be defined")
endif()


add_library(${PROJECT_NAME} SHARED
   layer/layer.cpp
   layer/private_data.cpp
   layer/surface_api.cpp
   layer/swapchain_api.cpp
   layer/device_api.cpp
   util/timed_semaphore.cpp
   util/custom_allocator.cpp
   util/extension_list.cpp
   util/fake_logger.cpp
   util/pose.cpp
   wsi/display.cpp
   wsi/swapchain_base.cpp
   wsi/wsi_factory.cpp
   wsi/headless/surface_properties.cpp
   wsi/headless/swapchain.cpp
   ../../alvr_server/Settings.cpp)

target_link_libraries(${PROJECT_NAME} PkgConfig::libunwind)
   
target_compile_definitions(${PROJECT_NAME} PRIVATE -DVK_USE_PLATFORM_XLIB_XRANDR_EXT)
set_target_properties(${PROJECT_NAME} PROPERTIES CXX_STANDARD 17)
target_include_directories(${PROJECT_NAME} PRIVATE ${PROJECT_SOURCE_DIR} ${VULKAN_CXX_INCLUDE})
target_include_directories(${PROJECT_NAME} PRIVATE ${PROJECT_SOURCE_DIR} ../../)

configure_file(layer/alvr_x86_64.json alvr_x86_64.json COPYONLY)
