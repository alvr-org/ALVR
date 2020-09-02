#ifndef OVR_PLATFORM_DEFS_H
#define OVR_PLATFORM_DEFS_H

//-----------------------------------------------------------------------------------
// ***** OVRP_CDECL
//
/// LibOVR calling convention for 32-bit Windows builds.
//
#if !defined(OVRP_CDECL)
#if defined(_WIN32)
#define OVRP_CDECL __cdecl
#else
#define OVRP_CDECL
#endif
#endif

//-----------------------------------------------------------------------------------
// ***** OVRP_EXTERN_C
//
/// Defined as extern "C" when built from C++ code.
//
#if !defined(OVRP_EXTERN_C)
#ifdef __cplusplus
#define OVRP_EXTERN_C extern "C"
#else
#define OVRP_EXTERN_C
#endif
#endif

//-----------------------------------------------------------------------------------
// ***** OVR_PUBLIC_FUNCTION / OVR_PRIVATE_FUNCTION
//
// OVRP_PUBLIC_FUNCTION  - Functions that externally visible from a shared library. Corresponds to
// Microsoft __dllexport. OVRP_PUBLIC_CLASS     - C++ structs and classes that are externally
// visible from a shared library. Corresponds to Microsoft __dllexport.
//
// OVRP_DLL_BUILD        - Used to indicate that the current compilation unit is of a shared
// library. OVRP_DLL_IMPORT       - Used to indicate that the current compilation unit is a user of
// the corresponding shared library (default) OVRP_DLL_BUILD        - used to indicate that the
// current compilation unit is not a shared library but rather statically linked code.
//
#if !defined(OVRP_PUBLIC_FUNCTION)
#if defined(OVRP_DLL_BUILD)
#if defined(_WIN32)
#define OVRP_PUBLIC_FUNCTION(rval) OVRP_EXTERN_C __declspec(dllexport) rval OVRP_CDECL
#define OVRP_PUBLIC_CLASS __declspec(dllexport)
#else
#define OVRP_PUBLIC_FUNCTION(rval) \
  OVRP_EXTERN_C __attribute__((visibility("default"))) rval OVRP_CDECL /* Requires GCC 4.0+ */
#define OVRP_PUBLIC_CLASS __attribute__((visibility("default"))) /* Requires GCC 4.0+ */
#endif
#elif defined(OVRP_STATIC_BUILD)
#define OVRP_PUBLIC_FUNCTION(rval) OVRP_EXTERN_C rval OVRP_CDECL
#define OVRP_PUBLIC_CLASS
#else
#if defined(_WIN32)
#define OVRP_PUBLIC_FUNCTION(rval) OVRP_EXTERN_C __declspec(dllimport) rval OVRP_CDECL
#define OVRP_PUBLIC_CLASS __declspec(dllimport)
#else
#define OVRP_PUBLIC_FUNCTION(rval) OVRP_EXTERN_C rval OVRP_CDECL
#define OVRP_PUBLIC_CLASS
#endif
#endif
#endif

/*
 * Declarations used for the static platform loader library
 *
 *  Besides the windows initialize call, a few functions below are stubbed by the
 * loader in order to ensure that they fail quickly and gracefully if the loader did
 * not manage to load the library (e.g. it was missing entirely, failed the signature
 * check, or whatever else).
 *  These functions are declared as OVRPL_PUBLIC_FUNCTION. This means the functions
 * are defined with dllexport when building the dll, but are defined without
 * dllexport when building against the static library. This means an app using only
 * the dll can call the dll's implementation, while an app using the static loader
 * will call the static loader's stubs, which will then call into the dll's
 * implementation if everything loaded OK.
 *  By default we want the expectation to be to use the loader, so an app that wants
 * to skip the loader needs to define OVRPL_DISABLED before loading the headers.
 */

#if !defined(OVRPL_PUBLIC_FUNCTION)
#if defined(OVRP_DLL_BUILD)
#define OVRPL_DISABLED 1
#endif

#if defined(OVRPL_DISABLED)
#define OVRPL_PUBLIC_FUNCTION(rval) OVRP_PUBLIC_FUNCTION(rval)
#else
#define OVRPL_PUBLIC_FUNCTION(rval) OVRP_EXTERN_C rval OVRP_CDECL
#endif
#endif

#endif
