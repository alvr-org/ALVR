#ifndef OVR_PLATFORMVERSION_H
#define OVR_PLATFORMVERSION_H

// Conventional string-ification macro.
#if !defined(OVR_STRINGIZE)
#define OVR_STRINGIZEIMPL(x) #x
#define OVR_STRINGIZE(x) OVR_STRINGIZEIMPL(x)
#endif

// Master version numbers
#define PLATFORM_PRODUCT_VERSION 1 // Product version doesn't participate in semantic versioning.
// If you change these values then you need to also make sure to change LibOVRPlatform.props in
// parallel.
#define PLATFORM_MAJOR_VERSION 1
#define PLATFORM_MINOR_VERSION 45
#define PLATFORM_PATCH_VERSION 0
#define PLATFORM_BUILD_NUMBER 0
#define PLATFORM_DRIVER_VERSION 0
// "Major.Minor.Patch.Build"
#if !defined(PLATFORM_VERSION_STRING)
#define PLATFORM_VERSION_STRING \
  OVR_STRINGIZE(                \
      PLATFORM_MAJOR_VERSION.PLATFORM_MINOR_VERSION.PLATFORM_PATCH_VERSION.PLATFORM_BUILD_NUMBER)
#endif

// This appears in the user-visible file properties.
// TeamCity builds will stamp PLATFORM_DESCRIPTION_STRING.
#if !defined(PLATFORM_DESCRIPTION_STRING)
#if defined(_DEBUG)
#define PLATFORM_DESCRIPTION_STRING "dev build debug"
#else
#define PLATFORM_DESCRIPTION_STRING "dev build"
#endif
#endif

#if !defined(OVR_JOIN)
#define OVR_JOIN(a, b) OVR_JOIN1(a, b)
#define OVR_JOIN1(a, b) OVR_JOIN2(a, b)
#define OVR_JOIN2(a, b) a##b
#endif

#endif
