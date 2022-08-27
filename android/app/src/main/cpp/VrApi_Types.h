/************************************************************************************

Filename    :   VrApi_Types.h
Content     :   Types for minimum necessary API for mobile VR
Created     :   April 30, 2015
Authors     :   J.M.P. van Waveren
Language    :   C99

Copyright   :   Copyright (c) Facebook Technologies, LLC and its affiliates. All rights reserved.

*************************************************************************************/
#ifndef OVR_VrApi_Types_h
#define OVR_VrApi_Types_h

#include <stdbool.h>
#include <stdint.h>
#include "VrApi_Config.h" // needed for VRAPI_EXPORT

//-----------------------------------------------------------------
// Java
//-----------------------------------------------------------------

#if defined(ANDROID)
#include <jni.h>
#elif defined(__cplusplus)
typedef struct _JNIEnv JNIEnv;
typedef struct _JavaVM JavaVM;
typedef class _jobject* jobject;
#else
typedef const struct JNINativeInterface* JNIEnv;
typedef const struct JNIInvokeInterface* JavaVM;
typedef void* jobject;
#endif

/// Java details about an activity
typedef struct ovrJava_ {
    JavaVM* Vm; //< Java Virtual Machine
    JNIEnv* Env; //< Thread specific environment
    jobject ActivityObject; //< Java activity object
} ovrJava;

OVR_VRAPI_ASSERT_TYPE_SIZE_32_BIT(ovrJava, 12);
OVR_VRAPI_ASSERT_TYPE_SIZE_64_BIT(ovrJava, 24);

//-----------------------------------------------------------------
// Basic Types
//-----------------------------------------------------------------

// All return codes for VrApi are reported via ovrResult.
// Possible return codes are split between successful completion
// codes (ovrSuccessResult) which are all positive values and
// error codes (ovrErrorResult) which are all negative values).
typedef signed int ovrResult;

typedef enum ovrSuccessResult_ {
    ovrSuccess = 0,
    ovrSuccess_BoundaryInvalid = 1001,
    ovrSuccess_EventUnavailable = 1002,
    ovrSuccess_Skipped = 1003,

} ovrSuccessResult;

typedef enum ovrErrorResult_ {
    ovrError_MemoryAllocationFailure = -1000,
    ovrError_NotInitialized = -1004,
    ovrError_InvalidParameter = -1005,
    ovrError_DeviceUnavailable = -1010, //< device is not connected,
                                        // or not connected as input device
    ovrError_InvalidOperation = -1015,

    // enums not in CAPI
    ovrError_UnsupportedDeviceType = -1050, //< specified device type isn't supported on GearVR
    ovrError_NoDevice = -1051, //< specified device ID does not map to any current device
    ovrError_NotImplemented = -1052, //< executed an incomplete code path - this should not be
                                     // possible in public releases.
    /// ovrError_NotReady is returned when a subsystem supporting an API is not yet ready.
    /// For some subsystems, vrapi_PollEvent will return a ready event once the sub-system is
    /// available.
    ovrError_NotReady = -1053,
    /// Data is unavailable
    ovrError_Unavailable = -1054,

    ovrResult_EnumSize = 0x7fffffff
} ovrErrorResult;

/// A 2D vector.
typedef struct ovrVector2f_ {
    float x, y;
} ovrVector2f;

OVR_VRAPI_ASSERT_TYPE_SIZE(ovrVector2f, 8);

/// A 3D vector.
typedef struct ovrVector3f_ {
    float x, y, z;
} ovrVector3f;

OVR_VRAPI_ASSERT_TYPE_SIZE(ovrVector3f, 12);

/// A 4D vector.
typedef struct ovrVector4f_ {
    float x, y, z, w;
} ovrVector4f;

OVR_VRAPI_ASSERT_TYPE_SIZE(ovrVector4f, 16);

typedef struct ovrVector4s_ {
    int16_t x, y, z, w;
} ovrVector4s;

OVR_VRAPI_ASSERT_TYPE_SIZE(ovrVector4s, 8);

/// Quaternion.
typedef struct ovrQuatf_ {
    float x, y, z, w;
} ovrQuatf;

OVR_VRAPI_ASSERT_TYPE_SIZE(ovrQuatf, 16);

/// Row-major 4x4 matrix.
typedef struct ovrMatrix4f_ {
    float M[4][4];
} ovrMatrix4f;

OVR_VRAPI_ASSERT_TYPE_SIZE(ovrMatrix4f, 64);

/// Position and orientation together.
typedef struct ovrPosef_ {
    ovrQuatf Orientation;
    union {
        ovrVector3f Position;
        ovrVector3f Translation;
    };
} ovrPosef;

OVR_VRAPI_ASSERT_TYPE_SIZE(ovrPosef, 28);

/// A rectangle with 2D size and position.
typedef struct ovrRectf_ {
    float x;
    float y;
    float width;
    float height;
} ovrRectf;

OVR_VRAPI_ASSERT_TYPE_SIZE(ovrRectf, 16);

/// True or false.
typedef enum ovrBooleanResult_ { VRAPI_FALSE = 0, VRAPI_TRUE = 1 } ovrBooleanResult;

/// One of the user's eyes.
typedef enum ovrEye_ { VRAPI_EYE_LEFT = 0, VRAPI_EYE_RIGHT = 1, VRAPI_EYE_COUNT = 2 } ovrEye;

//-----------------------------------------------------------------
// Structure Types
//-----------------------------------------------------------------

/// Defines a layout for ovrInitParms, ovrModeParms, or ovrFrameParms.
typedef enum ovrStructureType_ {
    VRAPI_STRUCTURE_TYPE_INIT_PARMS = 1,
    VRAPI_STRUCTURE_TYPE_MODE_PARMS = 2,
    VRAPI_STRUCTURE_TYPE_FRAME_PARMS = 3,
        VRAPI_STRUCTURE_TYPE_MODE_PARMS_VULKAN = 5,
    } ovrStructureType;

//-----------------------------------------------------------------
// System Properties and Status
//-----------------------------------------------------------------

/// A VR-capable device.
typedef enum ovrDeviceType_ {
        VRAPI_DEVICE_TYPE_OCULUSQUEST_START = 256,
        VRAPI_DEVICE_TYPE_OCULUSQUEST = VRAPI_DEVICE_TYPE_OCULUSQUEST_START + 3,
    VRAPI_DEVICE_TYPE_OCULUSQUEST_END = 319,
    VRAPI_DEVICE_TYPE_OCULUSQUEST2_START = 320,
    VRAPI_DEVICE_TYPE_OCULUSQUEST2 = VRAPI_DEVICE_TYPE_OCULUSQUEST2_START,
    VRAPI_DEVICE_TYPE_OCULUSQUEST2_END = 383,
                VRAPI_DEVICE_TYPE_UNKNOWN = -1,
} ovrDeviceType;

/// A geographic region authorized for certain hardware and content.
typedef enum ovrDeviceRegion_ {
    VRAPI_DEVICE_REGION_UNSPECIFIED = 0,
    VRAPI_DEVICE_REGION_JAPAN = 1,
    VRAPI_DEVICE_REGION_CHINA = 2,
} ovrDeviceRegion;

/// Emulation mode for applications developed on different devices
/// for determining if running in emulation mode at all test against !=
/// VRAPI_DEVICE_EMULATION_MODE_NONE
typedef enum ovrDeviceEmulationMode_ {
    VRAPI_DEVICE_EMULATION_MODE_NONE = 0,
    VRAPI_DEVICE_EMULATION_MODE_GO_ON_QUEST = 1,
} ovrDeviceEmulationMode;

/// System configuration properties.
typedef enum ovrSystemProperty_ {
    VRAPI_SYS_PROP_DEVICE_TYPE = 0,
    VRAPI_SYS_PROP_MAX_FULLSPEED_FRAMEBUFFER_SAMPLES = 1,
    /// Physical width and height of the display in pixels.
    VRAPI_SYS_PROP_DISPLAY_PIXELS_WIDE = 2,
    VRAPI_SYS_PROP_DISPLAY_PIXELS_HIGH = 3,
    /// Returns the refresh rate of the display in cycles per second.
    VRAPI_SYS_PROP_DISPLAY_REFRESH_RATE = 4,
    /// With a display resolution of 2560x1440, the pixels at the center
    /// of each eye cover about 0.06 degrees of visual arc. To wrap a
    /// full 360 degrees, about 6000 pixels would be needed and about one
    /// quarter of that would be needed for ~90 degrees FOV. As such, Eye
    /// images with a resolution of 1536x1536 result in a good 1:1 mapping
    /// in the center, but they need mip-maps for off center pixels. To
    /// avoid the need for mip-maps and for significantly improved rendering
    /// performance this currently returns a conservative 1024x1024.
    VRAPI_SYS_PROP_SUGGESTED_EYE_TEXTURE_WIDTH = 5,
    VRAPI_SYS_PROP_SUGGESTED_EYE_TEXTURE_HEIGHT = 6,
    /// This is a product of the lens distortion and the screen size,
    /// but there is no truly correct answer.
    /// There is a tradeoff in resolution and coverage.
    /// Too small of an FOV will leave unrendered pixels visible, but too
    /// large wastes resolution or fill rate.  It is unreasonable to
    /// increase it until the corners are completely covered, but we do
    /// want most of the outside edges completely covered.
    /// Applications might choose to render a larger FOV when angular
    /// acceleration is high to reduce black pull in at the edges by
    /// the time warp.
    /// Currently symmetric 90.0 degrees.
    VRAPI_SYS_PROP_SUGGESTED_EYE_FOV_DEGREES_X = 7,
    VRAPI_SYS_PROP_SUGGESTED_EYE_FOV_DEGREES_Y = 8,
        VRAPI_SYS_PROP_DEVICE_REGION = 10,
        /// Returns an ovrHandedness enum indicating left or right hand.
    VRAPI_SYS_PROP_DOMINANT_HAND = 15,

    /// Returns VRAPI_TRUE if the system supports orientation tracking.
    VRAPI_SYS_PROP_HAS_ORIENTATION_TRACKING = 16,
    /// Returns VRAPI_TRUE if the system supports positional tracking.
    VRAPI_SYS_PROP_HAS_POSITION_TRACKING = 17,

    /// Returns the number of display refresh rates supported by the system.
    VRAPI_SYS_PROP_NUM_SUPPORTED_DISPLAY_REFRESH_RATES = 64,
    /// Returns an array of the supported display refresh rates.
    VRAPI_SYS_PROP_SUPPORTED_DISPLAY_REFRESH_RATES = 65,

    /// Returns the number of swapchain texture formats supported by the system.
    VRAPI_SYS_PROP_NUM_SUPPORTED_SWAPCHAIN_FORMATS = 66,
    /// Returns an array of the supported swapchain formats.
    /// Formats are platform specific. For GLES, this is an array of
    /// GL internal formats.
    VRAPI_SYS_PROP_SUPPORTED_SWAPCHAIN_FORMATS = 67,
        /// Returns VRAPI_TRUE if on-chip foveated rendering of swapchains is supported
    /// for this system, otherwise VRAPI_FALSE.
    VRAPI_SYS_PROP_FOVEATION_AVAILABLE = 130,
    } ovrSystemProperty;

/// Configurable VrApi properties.
typedef enum ovrProperty_ {
        VRAPI_FOVEATION_LEVEL = 15, //< Used by apps that want to control swapchain foveation levels.
    
    VRAPI_EAT_NATIVE_GAMEPAD_EVENTS =
        20, //< Used to tell the runtime not to eat gamepad events.  If this is false on a native
    // app, the app must be listening for the events.
        VRAPI_ACTIVE_INPUT_DEVICE_ID = 24, //< Used by apps to query which input device is most 'active'
                                       // or primary, a -1 means no active input device
        VRAPI_DEVICE_EMULATION_MODE = 29, //< Used by apps to determine if they are running in an
                                      // emulation mode. Is a ovrDeviceEmulationMode value

    VRAPI_DYNAMIC_FOVEATION_ENABLED =
        30, //< Used by apps to enable / disable dynamic foveation adjustments.
    } ovrProperty;

/// System status bits.
typedef enum ovrSystemStatus_ {
    // enum 0 used to be VRAPI_SYS_STATUS_DOCKED.
    VRAPI_SYS_STATUS_MOUNTED = 1, //< Device is mounted.
    VRAPI_SYS_STATUS_THROTTLED = 2, //< Device is in powersave mode.

    // enum  3 used to be VRAPI_SYS_STATUS_THROTTLED2.

    // enum  4 used to be VRAPI_SYS_STATUS_THROTTLED_WARNING_LEVEL.

    VRAPI_SYS_STATUS_RENDER_LATENCY_MILLISECONDS =
        5, //< Average time between render tracking sample and scanout.
    VRAPI_SYS_STATUS_TIMEWARP_LATENCY_MILLISECONDS =
        6, //< Average time between timewarp tracking sample and scanout.
    VRAPI_SYS_STATUS_SCANOUT_LATENCY_MILLISECONDS = 7, //< Average time between Vsync and scanout.
    VRAPI_SYS_STATUS_APP_FRAMES_PER_SECOND =
        8, //< Number of frames per second delivered through vrapi_SubmitFrame.
    VRAPI_SYS_STATUS_SCREEN_TEARS_PER_SECOND = 9, //< Number of screen tears per second (per eye).
    VRAPI_SYS_STATUS_EARLY_FRAMES_PER_SECOND =
        10, //< Number of frames per second delivered a whole display refresh early.
    VRAPI_SYS_STATUS_STALE_FRAMES_PER_SECOND = 11, //< Number of frames per second delivered late.

    // enum 12 used to be VRAPI_SYS_STATUS_HEADPHONES_PLUGGED_IN

    VRAPI_SYS_STATUS_RECENTER_COUNT = 13, //< Returns the current HMD recenter count. Defaults to 0.
    // enum 14 used to be VRAPI_SYS_STATUS_SYSTEM_UX_ACTIVE
    VRAPI_SYS_STATUS_USER_RECENTER_COUNT = 15, //< Returns the current HMD recenter count for user
                                               // initiated recenters only. Defaults to 0.

    
        VRAPI_SYS_STATUS_FRONT_BUFFER_SRGB =
        130, //< VRAPI_TRUE if the front buffer uses the sRGB color space.

    VRAPI_SYS_STATUS_SCREEN_CAPTURE_RUNNING =
        131, // VRAPI_TRUE if the screen is currently being recorded.

    } ovrSystemStatus;

//-----------------------------------------------------------------
// Initialization
//-----------------------------------------------------------------

/// Possible results of initialization.
typedef enum ovrInitializeStatus_ {
    VRAPI_INITIALIZE_SUCCESS = 0,
    VRAPI_INITIALIZE_UNKNOWN_ERROR = -1,
    VRAPI_INITIALIZE_PERMISSIONS_ERROR = -2,
    VRAPI_INITIALIZE_ALREADY_INITIALIZED = -3,
    VRAPI_INITIALIZE_SERVICE_CONNECTION_FAILED = -4,
    VRAPI_INITIALIZE_DEVICE_NOT_SUPPORTED = -5,
} ovrInitializeStatus;

/// Supported graphics APIs.
typedef enum ovrGraphicsAPI_ {
    VRAPI_GRAPHICS_API_TYPE_OPENGL_ES = 0x10000,
    VRAPI_GRAPHICS_API_OPENGL_ES_2 =
        (VRAPI_GRAPHICS_API_TYPE_OPENGL_ES | 0x0200), //< OpenGL ES 2.x context
    VRAPI_GRAPHICS_API_OPENGL_ES_3 =
        (VRAPI_GRAPHICS_API_TYPE_OPENGL_ES | 0x0300), //< OpenGL ES 3.x context

    VRAPI_GRAPHICS_API_TYPE_OPENGL = 0x20000,
    VRAPI_GRAPHICS_API_OPENGL_COMPAT =
        (VRAPI_GRAPHICS_API_TYPE_OPENGL | 0x0100), //< OpenGL Compatibility Profile
    VRAPI_GRAPHICS_API_OPENGL_CORE_3 =
        (VRAPI_GRAPHICS_API_TYPE_OPENGL | 0x0300), //< OpenGL Core Profile 3.x
    VRAPI_GRAPHICS_API_OPENGL_CORE_4 =
        (VRAPI_GRAPHICS_API_TYPE_OPENGL | 0x0400), //< OpenGL Core Profile 4.x

    VRAPI_GRAPHICS_API_TYPE_VULKAN = 0x40000,
    VRAPI_GRAPHICS_API_VULKAN_1 = (VRAPI_GRAPHICS_API_TYPE_VULKAN | 0x0100), //< Vulkan 1.x
} ovrGraphicsAPI;

/// Configuration details specified at initialization.
typedef struct ovrInitParms_ {
    ovrStructureType Type;
    int ProductVersion;
    int MajorVersion;
    int MinorVersion;
    int PatchVersion;
    ovrGraphicsAPI GraphicsAPI;
    ovrJava Java;
} ovrInitParms;

OVR_VRAPI_ASSERT_TYPE_SIZE_32_BIT(ovrInitParms, 36);
OVR_VRAPI_ASSERT_TYPE_SIZE_64_BIT(ovrInitParms, 48);


//-----------------------------------------------------------------
// VR Mode
//-----------------------------------------------------------------

/// \note the first two flags use the first two bytes for backwards compatibility on little endian
/// systems.
typedef enum ovrModeFlags_ {
        /// When an application moves backwards on the activity stack,
    /// the activity window it returns to is no longer flagged as fullscreen.
    /// As a result, Android will also render the decor view, which wastes a
    /// significant amount of bandwidth.
    /// By setting this flag, the fullscreen flag is reset on the window.
    /// Unfortunately, this causes Android life cycle events that mess up
    /// several NativeActivity codebases like Stratum and UE4, so this
    /// flag should only be set for specific applications.
    /// Use "adb shell dumpsys SurfaceFlinger" to verify
    /// that there is only one HWC next to the FB_TARGET.
    VRAPI_MODE_FLAG_RESET_WINDOW_FULLSCREEN = 0x0000FF00,

    /// The WindowSurface passed in is an ANativeWindow.
    VRAPI_MODE_FLAG_NATIVE_WINDOW = 0x00010000,

        /// Create a front buffer using the sRGB color space.
    VRAPI_MODE_FLAG_FRONT_BUFFER_SRGB = 0x00080000,

    /// enum 0x00100000 used to be VRAPI_MODE_FLAG_CREATE_CONTEXT_NO_ERROR.

    
    /// If set, phase Sync mode will be enabled for the application.
    /// When Phase sync mode is enabled, prediction latency will be managed adaptively
    /// such that when the applications's workload is low, the prediction latency will also be low.
    /// Note: Phase Sync mode should only be enabled if the application is using the
    /// vrapi_WaitFrame / vrapi_BeginFrame / vrapi_SubmitFrame frame call pattern.
    /// If an application only calls vrapi_SubmitFrame, the mode can't be enabled.
    VRAPI_MODE_FLAG_PHASE_SYNC = 0x00400000,

} ovrModeFlags;

/// Configuration details that stay constant between a vrapi_EnterVrMode()/vrapi_LeaveVrMode() pair.
typedef struct ovrModeParms_ {
    ovrStructureType Type;

    /// Combination of ovrModeFlags flags.
    unsigned int Flags;

    /// The Java VM is needed for the time warp thread to create a Java environment.
    /// A Java environment is needed to access various system services. The thread
    /// that enters VR mode is responsible for attaching and detaching the Java
    /// environment. The Java Activity object is needed to get the windowManager,
    /// packageName, systemService, etc.
    ovrJava Java;

    OVR_VRAPI_PADDING_32_BIT(4)

    /// Display to use for asynchronous time warp rendering.
    /// Using EGL this is an EGLDisplay.
    unsigned long long Display;

    /// The ANativeWIndow associated with the application's Surface (requires
    /// VRAPI_MODE_FLAG_NATIVE_WINDOW). The ANativeWIndow is used for asynchronous time warp
    /// rendering.
    unsigned long long WindowSurface;

    /// The resources from this context will be shared with the asynchronous time warp.
    /// Using EGL this is an EGLContext.
    unsigned long long ShareContext;
} ovrModeParms;

OVR_VRAPI_ASSERT_TYPE_SIZE_32_BIT(ovrModeParms, 48);
OVR_VRAPI_ASSERT_TYPE_SIZE_64_BIT(ovrModeParms, 56);

/// Vulkan-specific mode paramaters.
typedef struct ovrModeParmsVulkan_ {
    ovrModeParms ModeParms;

    /// For Vulkan, this should be the VkQueue created on the same Device as specified
    /// by vrapi_CreateSystemVulkan. An internally created VkFence object will be signaled
    /// by the completion of commands on the queue.
    unsigned long long SynchronizationQueue;
} ovrModeParmsVulkan;

OVR_VRAPI_ASSERT_TYPE_SIZE_32_BIT(ovrModeParmsVulkan, 56);
OVR_VRAPI_ASSERT_TYPE_SIZE_64_BIT(ovrModeParmsVulkan, 64);

/// VR context
/// To allow multiple Android activities that live in the same address space
/// to cooperatively use the VrApi, each activity needs to maintain its own
/// separate contexts for a lot of the video related systems.
typedef struct ovrMobile ovrMobile;

//-----------------------------------------------------------------
// Tracking
//-----------------------------------------------------------------

/// Full rigid body pose with first and second derivatives.
typedef struct ovrRigidBodyPosef_ {
    ovrPosef Pose;
    ovrVector3f AngularVelocity;
    ovrVector3f LinearVelocity;
    ovrVector3f AngularAcceleration;
    ovrVector3f LinearAcceleration;
    OVR_VRAPI_PADDING(4)
    double TimeInSeconds; //< Absolute time of this pose.
    double PredictionInSeconds; //< Seconds this pose was predicted ahead.
} ovrRigidBodyPosef;

OVR_VRAPI_ASSERT_TYPE_SIZE(ovrRigidBodyPosef, 96);

/// Bit flags describing the current status of sensor tracking.
typedef enum ovrTrackingStatus_ {
    VRAPI_TRACKING_STATUS_ORIENTATION_TRACKED = 1 << 0, //< Orientation is currently tracked.
    VRAPI_TRACKING_STATUS_POSITION_TRACKED = 1 << 1, //< Position is currently tracked.
    VRAPI_TRACKING_STATUS_ORIENTATION_VALID = 1 << 2, //< Orientation reported is valid.
    VRAPI_TRACKING_STATUS_POSITION_VALID = 1 << 3, //< Position reported is valid.
        VRAPI_TRACKING_STATUS_HMD_CONNECTED = 1 << 7 //< HMD is available & connected.
} ovrTrackingStatus;

/// Tracking state at a given absolute time.
typedef struct ovrTracking2_ {
    /// Sensor status described by ovrTrackingStatus flags.
    unsigned int Status;

    OVR_VRAPI_PADDING(4)

    /// Predicted head configuration at the requested absolute time.
    /// The pose describes the head orientation and center eye position.
    ovrRigidBodyPosef HeadPose;
    struct {
        ovrMatrix4f ProjectionMatrix;
        ovrMatrix4f ViewMatrix;
    } Eye[VRAPI_EYE_COUNT];
} ovrTracking2;

OVR_VRAPI_ASSERT_TYPE_SIZE(ovrTracking2, 360);


/// Reports the status and pose of a motion tracker.
typedef struct ovrTracking_ {
    /// Sensor status described by ovrTrackingStatus flags.
    unsigned int Status;

    OVR_VRAPI_PADDING(4)

    /// Predicted head configuration at the requested absolute time.
    /// The pose describes the head orientation and center eye position.
    ovrRigidBodyPosef HeadPose;
} ovrTracking;

OVR_VRAPI_ASSERT_TYPE_SIZE(ovrTracking, 104);

/// Specifies a reference frame for motion tracking data.
typedef enum ovrTrackingTransform_ {
    VRAPI_TRACKING_TRANSFORM_IDENTITY = 0,
    VRAPI_TRACKING_TRANSFORM_CURRENT = 1,
    VRAPI_TRACKING_TRANSFORM_SYSTEM_CENTER_EYE_LEVEL = 2,
    VRAPI_TRACKING_TRANSFORM_SYSTEM_CENTER_FLOOR_LEVEL = 3,
    } ovrTrackingTransform;

typedef enum ovrTrackingSpace_ {
    VRAPI_TRACKING_SPACE_LOCAL = 0, // Eye level origin - controlled by system recentering
    VRAPI_TRACKING_SPACE_LOCAL_FLOOR = 1, // Floor level origin - controlled by system recentering
    VRAPI_TRACKING_SPACE_LOCAL_TILTED =
        2, // Tilted pose for "bed mode" - controlled by system recentering
    VRAPI_TRACKING_SPACE_STAGE = 3, // Floor level origin - controlled by Guardian setup
        VRAPI_TRACKING_SPACE_LOCAL_FIXED_YAW = 7, // Position of local space, but yaw stays constant
} ovrTrackingSpace;

/// Tracked device type id used to simplify interaction checks with Guardian
typedef enum ovrTrackedDeviceTypeId_ {
    VRAPI_TRACKED_DEVICE_NONE = -1,
    VRAPI_TRACKED_DEVICE_HMD = 0, //< Headset
    VRAPI_TRACKED_DEVICE_HAND_LEFT = 1, //< Left controller
    VRAPI_TRACKED_DEVICE_HAND_RIGHT = 2, //< Right controller
    VRAPI_NUM_TRACKED_DEVICES = 3,
} ovrTrackedDeviceTypeId;

/// Guardian boundary trigger state information based on a given tracked device type
typedef struct ovrBoundaryTriggerResult_ {
    /// Closest point on the boundary surface.
    ovrVector3f ClosestPoint;

    /// Normal of the closest point on the boundary surface.
    ovrVector3f ClosestPointNormal;

    /// Distance to the closest guardian boundary surface.
    float ClosestDistance;

    /// True if the boundary system is being triggered. Note that due to fade in/out effects this
    /// may not exactly match visibility.
    bool IsTriggering;
} ovrBoundaryTriggerResult;

OVR_VRAPI_ASSERT_TYPE_SIZE(ovrBoundaryTriggerResult, 32);


//-----------------------------------------------------------------
// Texture Swap Chain
//-----------------------------------------------------------------

/// A texture type, such as 2D, array, or cubemap.
typedef enum ovrTextureType_ {
    VRAPI_TEXTURE_TYPE_2D = 0, //< 2D textures.
        VRAPI_TEXTURE_TYPE_2D_ARRAY = 2, //< Texture array.
    VRAPI_TEXTURE_TYPE_CUBE = 3, //< Cube maps.
    VRAPI_TEXTURE_TYPE_MAX = 4,
    } ovrTextureType;

/// A texture format.
/// DEPRECATED in favor of passing platform-specific formats to vrapi_CreateTextureSwapChain3.
typedef enum ovrTextureFormat_ {
    VRAPI_TEXTURE_FORMAT_NONE = 0,
    VRAPI_TEXTURE_FORMAT_565 = 1,
    VRAPI_TEXTURE_FORMAT_5551 = 2,
    VRAPI_TEXTURE_FORMAT_4444 = 3,
    VRAPI_TEXTURE_FORMAT_8888 = 4,
    VRAPI_TEXTURE_FORMAT_8888_sRGB = 5,
    VRAPI_TEXTURE_FORMAT_RGBA16F = 6,
    VRAPI_TEXTURE_FORMAT_DEPTH_16 = 7,
    VRAPI_TEXTURE_FORMAT_DEPTH_24 = 8,
    VRAPI_TEXTURE_FORMAT_DEPTH_24_STENCIL_8 = 9,
    VRAPI_TEXTURE_FORMAT_RG16 = 10,

    } ovrTextureFormat;

typedef enum ovrTextureFilter_ {
    VRAPI_TEXTURE_FILTER_NEAREST = 0,
    VRAPI_TEXTURE_FILTER_LINEAR = 1,
    VRAPI_TEXTURE_FILTER_NEAREST_MIPMAP_LINEAR = 2,
    VRAPI_TEXTURE_FILTER_LINEAR_MIPMAP_NEAREST = 3,
    VRAPI_TEXTURE_FILTER_LINEAR_MIPMAP_LINEAR = 4,
    VRAPI_TEXTURE_FILTER_CUBIC = 5,
    VRAPI_TEXTURE_FILTER_CUBIC_MIPMAP_NEAREST = 6,
    VRAPI_TEXTURE_FILTER_CUBIC_MIPMAP_LINEAR = 7,
} ovrTextureFilter;

typedef enum ovrTextureWrapMode_ {
    VRAPI_TEXTURE_WRAP_MODE_REPEAT = 0,
    VRAPI_TEXTURE_WRAP_MODE_CLAMP_TO_EDGE = 1,
    VRAPI_TEXTURE_WRAP_MODE_CLAMP_TO_BORDER = 2,
} ovrTextureWrapMode;

typedef struct ovrTextureSamplerState_ {
    ovrTextureFilter MinFilter;
    ovrTextureFilter MagFilter;
    ovrTextureWrapMode WrapModeS;
    ovrTextureWrapMode WrapModeT;
    float BorderColor[4];
    float MaxAnisotropy;
} ovrTextureSamplerState;

OVR_VRAPI_ASSERT_TYPE_SIZE(ovrTextureSamplerState, 36);

/// Flags supported by vrapi_CreateAndroidSurfaceSwapChain3
typedef enum ovrAndroidSurfaceSwapChainFlags_ {
    /// Create a protected surface, suitable for secure video playback.
    VRAPI_ANDROID_SURFACE_SWAP_CHAIN_FLAG_PROTECTED = 0x1,
    /// Create the underlying BufferQueue in synchronous mode, allowing multiple buffers to be
    /// queued instead of always replacing the last buffer.  Buffers are retired in order, and
    /// the producer may block until a new buffer is available.
    VRAPI_ANDROID_SURFACE_SWAP_CHAIN_FLAG_SYNCHRONOUS = 0x2,
    /// Indicates that the compositor should acquire the most recent buffer whose presentation
    /// timestamp is not greater than the expected display time of the final composited frame.
    /// Together with FLAG_SYNCHRONOUS, this flag is suitable for video surfaces where several
    /// frames can be queued ahead of time.
    VRAPI_ANDROID_SURFACE_SWAP_CHAIN_FLAG_USE_TIMESTAMPS = 0x4,
} ovrAndroidSurfaceSwapChainFlags;

/// Built-in convenience swapchains.
typedef enum ovrDefaultTextureSwapChain_ {
    VRAPI_DEFAULT_TEXTURE_SWAPCHAIN = 0x1,
    VRAPI_DEFAULT_TEXTURE_SWAPCHAIN_LOADING_ICON = 0x2
} ovrDefaultTextureSwapChain;

typedef struct ovrTextureSwapChain ovrTextureSwapChain;

typedef enum ovrSwapChainCreateFlags_ {
    /// Image is in subsampled layout.
    VRAPI_SWAPCHAIN_CREATE_SUBSAMPLED_BIT = 0x1,
} ovrSwapChainCreateFlags;

typedef enum ovrSwapChainUsageFlags_ {
    /// Image may be a color rendering target.
    VRAPI_SWAPCHAIN_USAGE_COLOR_ATTACHMENT_BIT = 0x1,

    /// Image may be a depth/stencil rendering target.
    VRAPI_SWAPCHAIN_USAGE_DEPTH_STENCIL_ATTACHMENT_BIT = 0x2,
} ovrSwapChainUsageFlags;

typedef struct ovrSwapChainCreateInfo_ {
    /// GL/Vulkan format of the texture, e.g. GL_RGBA or VK_FORMAT_R8G8B8A8_UNORM),
    /// depending on GraphicsAPI used.
    int64_t Format;

    /// Width in pixels.
    int Width;

    /// Height in pixels.
    int Height;

    /// The number of levels of detail available for minified sampling of the image.
    int Levels;

    /// Number of faces, which can be either 6 (for cubemaps) or 1.
    int FaceCount;

    /// Number of array layers, 1 for 2D texture, 2 for texture 2D array (multiview case).
    int ArraySize;

    /// Number of buffers in the texture swap chain.
    int BufferCount;

    /// A bitmask of ovrSwapChainCreateFlags describing additional properties of
    /// the swapchain.
    uint64_t CreateFlags;

    /// A bitmask of ovrSwapChainUsageFlags describing intended usage of the
    /// swapchain's images.
    uint64_t UsageFlags;
} ovrSwapChainCreateInfo;

OVR_VRAPI_ASSERT_TYPE_SIZE_32_BIT(ovrSwapChainCreateInfo, 48);
OVR_VRAPI_ASSERT_TYPE_SIZE_64_BIT(ovrSwapChainCreateInfo, 48);

//-----------------------------------------------------------------
// Frame Submission
//-----------------------------------------------------------------

/// Per-frame configuration options.
typedef enum ovrFrameFlags_ {
    // enum 1 << 0 used to be VRAPI_FRAME_FLAG_INHIBIT_SRGB_FRAMEBUFFER. See per-layer
    // flag VRAPI_FRAME_LAYER_FLAG_INHIBIT_SRGB_FRAMEBUFFER.

    /// Flush the warp swap pipeline so the images show up immediately.
    /// This is expensive and should only be used when an immediate transition
    /// is needed like displaying black when resetting the HMD orientation.
    VRAPI_FRAME_FLAG_FLUSH = 1 << 1,
    /// This is the final frame. Do not accept any more frames after this.
    VRAPI_FRAME_FLAG_FINAL = 1 << 2,

    /// enum 1 << 3 used to be VRAPI_FRAME_FLAG_TIMEWARP_DEBUG_GRAPH_SHOW.
    /// enum 1 << 4 used to be VRAPI_FRAME_FLAG_TIMEWARP_DEBUG_GRAPH_FREEZE.
    /// enum 1 << 5 used to be VRAPI_FRAME_FLAG_TIMEWARP_DEBUG_GRAPH_LATENCY_MODE.

    /// Don't show the volume layer when set.
    VRAPI_FRAME_FLAG_INHIBIT_VOLUME_LAYER = 1 << 6,

    /// enum 1 << 7 used to be VRAPI_FRAME_FLAG_SHOW_LAYER_COMPLEXITY.

    /// enum 1 << 8 used to be VRAPI_FRAME_FLAG_SHOW_TEXTURE_DENSITY.

    } ovrFrameFlags;

/// Per-frame configuration options that apply to a particular layer.
typedef enum ovrFrameLayerFlags_ {
    
    /// NOTE: On Oculus standalone devices, chromatic aberration correction is enabled
    /// by default.
    /// For non Oculus standalone devices, this must be explicitly enabled by specifying the layer
    /// flag as it is a quality / performance trade off.
    VRAPI_FRAME_LAYER_FLAG_CHROMATIC_ABERRATION_CORRECTION = 1 << 1,
    /// Used for some HUDs, but generally considered bad practice.
    VRAPI_FRAME_LAYER_FLAG_FIXED_TO_VIEW = 1 << 2,
    /// \deprecated Spin the layer - for loading icons
    VRAPI_FRAME_LAYER_FLAG_SPIN = 1 << 3,
    /// Clip fragments outside the layer's TextureRect
    VRAPI_FRAME_LAYER_FLAG_CLIP_TO_TEXTURE_RECT = 1 << 4,
    
    /// To get gamma correct sRGB filtering of the eye textures, the textures must be
    /// allocated with GL_SRGB8_ALPHA8 format and the window surface must be allocated
    /// with these attributes:
    /// EGL_GL_COLORSPACE_KHR,  EGL_GL_COLORSPACE_SRGB_KHR
    ///
    /// While we can reallocate textures easily enough, we can't change the window
    /// colorspace without relaunching the entire application, so if you want to
    /// be able to toggle between gamma correct and incorrect, you must allocate
    /// the framebuffer as sRGB, then inhibit that processing when using normal
    /// textures.
    ///
    /// If the texture being read isn't an sRGB texture, the conversion
    /// on write must be inhibited or the colors are washed out.
    /// This is necessary for using external images on an sRGB framebuffer.
    VRAPI_FRAME_LAYER_FLAG_INHIBIT_SRGB_FRAMEBUFFER = 1 << 8,

    
    /// Allow Layer to use an expensive filtering mode. Only useful for 2D layers that are high
    /// resolution (e.g. a remote desktop layer), typically double or more the target resolution.
    VRAPI_FRAME_LAYER_FLAG_FILTER_EXPENSIVE = 1 << 19,

    
} ovrFrameLayerFlags;


/// The user's eye (left or right) that can see a layer.
typedef enum ovrFrameLayerEye_ {
    VRAPI_FRAME_LAYER_EYE_LEFT = 0,
    VRAPI_FRAME_LAYER_EYE_RIGHT = 1,
    VRAPI_FRAME_LAYER_EYE_MAX = 2
} ovrFrameLayerEye;

/// Selects an operation for alpha blending two images.
typedef enum ovrFrameLayerBlend_ {
    VRAPI_FRAME_LAYER_BLEND_ZERO = 0,
    VRAPI_FRAME_LAYER_BLEND_ONE = 1,
    VRAPI_FRAME_LAYER_BLEND_SRC_ALPHA = 2,
        VRAPI_FRAME_LAYER_BLEND_ONE_MINUS_SRC_ALPHA = 5
} ovrFrameLayerBlend;

/// Extra latency mode pipelines app CPU work a frame ahead of VR composition.
typedef enum ovrExtraLatencyMode_ {
    VRAPI_EXTRA_LATENCY_MODE_OFF = 0,
    VRAPI_EXTRA_LATENCY_MODE_ON = 1,
    VRAPI_EXTRA_LATENCY_MODE_DYNAMIC = 2
} ovrExtraLatencyMode;

//-------------------------------------
// Legacy monolithic FrameParm submission structures for vrapi_SubmitFrame.
//-------------------------------------

/// \deprecated The vrapi_SubmitFrame2 path with flexible layer types
/// should be used instead.
OVR_VRAPI_DEPRECATED(typedef enum ovrFrameLayerType_{
    VRAPI_FRAME_LAYER_TYPE_MAX = 4} ovrFrameLayerType);

/// A compositor layer.
/// \note Any layer textures that are dynamic must be triple buffered.
/// \deprecated The vrapi_SubmitFrame2 path with flexible layer types
/// should be used instead.
typedef struct ovrFrameLayerTexture_ {
    /// Because OpenGL ES does not support clampToBorder, it is the
    /// application's responsibility to make sure that all mip levels
    /// of the primary eye texture have a black border that will show
    /// up when time warp pushes the texture partially off screen.
    ovrTextureSwapChain* ColorTextureSwapChain;

    /// \deprecated The depth texture is optional for positional time warp.
    ovrTextureSwapChain* DepthTextureSwapChain;

    /// Index to the texture from the set that should be displayed.
    int TextureSwapChainIndex;

    /// Points on the screen are mapped by a distortion correction
    /// function into ( TanX, TanY, -1, 1 ) vectors that are transformed
    /// by this matrix to get ( S, T, Q, _ ) vectors that are looked
    /// up with texture2dproj() to get texels.
    ovrMatrix4f TexCoordsFromTanAngles;

    /// Only texels within this range should be drawn.
    /// This is a sub-rectangle of the [(0,0)-(1,1)] texture coordinate range.
    ovrRectf TextureRect;

    OVR_VRAPI_PADDING(4)

    /// The tracking state for which ModelViewMatrix is correct.
    /// It is ok to update the orientation for each eye, which
    /// can help minimize black edge pull-in, but the position
    /// must remain the same for both eyes, or the position would
    /// seem to judder "backwards in time" if a frame is dropped.
    ovrRigidBodyPosef HeadPose;

            /// \unused parameter.
        unsigned char Pad[8];
        } ovrFrameLayerTexture;

OVR_VRAPI_ASSERT_TYPE_SIZE_32_BIT(ovrFrameLayerTexture, 200);
OVR_VRAPI_ASSERT_TYPE_SIZE_64_BIT(ovrFrameLayerTexture, 208);

/// Per-frame state of a compositor layer.
/// \deprecated The vrapi_SubmitFrame2 path with flexible layer types
/// should be used instead.
typedef struct ovrFrameLayer_ {
    /// Image used for each eye.
    ovrFrameLayerTexture Textures[VRAPI_FRAME_LAYER_EYE_MAX];

    /// Speed and scale of rotation when VRAPI_FRAME_LAYER_FLAG_SPIN is set in ovrFrameLayer::Flags
    float SpinSpeed; //< Radians/Second
    float SpinScale;

    /// Color scale for this layer (including alpha)
    float ColorScale;

    /// padding for deprecated variable.
    OVR_VRAPI_PADDING(4)

    /// Layer blend function.
    ovrFrameLayerBlend SrcBlend;
    ovrFrameLayerBlend DstBlend;

    /// Combination of ovrFrameLayerFlags flags.
    int Flags;

    /// explicit padding for x86
    OVR_VRAPI_PADDING_32_BIT(4)
} ovrFrameLayer;

OVR_VRAPI_ASSERT_TYPE_SIZE_32_BIT(ovrFrameLayer, 432);
OVR_VRAPI_ASSERT_TYPE_SIZE_64_BIT(ovrFrameLayer, 448);

/// Configuration parameters that affect system performance and scheduling behavior.
/// \deprecated The vrapi_SubmitFrame2 path with flexible layer types
/// should be used instead.
typedef struct ovrPerformanceParms_ {
    /// These are fixed clock levels in the range [0, 3].
    int CpuLevel;
    int GpuLevel;

    /// These threads will get SCHED_FIFO.
    int MainThreadTid;
    int RenderThreadTid;
} ovrPerformanceParms;

OVR_VRAPI_ASSERT_TYPE_SIZE(ovrPerformanceParms, 16);

/// Per-frame details.
/// \deprecated The vrapi_SubmitFrame2 path with flexible layer types
/// should be used instead.
OVR_VRAPI_DEPRECATED(typedef struct ovrFrameParms_ {
    ovrStructureType Type;

    OVR_VRAPI_PADDING(4)

    /// Layers composited in the time warp.
    ovrFrameLayer Layers[VRAPI_FRAME_LAYER_TYPE_MAX];
    int LayerCount;

    /// Combination of ovrFrameFlags flags.
    int Flags;

    /// Application controlled frame index that uniquely identifies this particular frame.
    /// This must be the same frame index that was passed to vrapi_GetPredictedDisplayTime()
    /// when synthesis of this frame started.
    long long FrameIndex;

    /// WarpSwap will not return until at least this many V-syncs have
    /// passed since the previous WarpSwap returned.
    /// Setting to 2 will reduce power consumption and may make animation
    /// more regular for applications that can't hold full frame rate.
    int SwapInterval;

    /// Latency Mode.
    ovrExtraLatencyMode ExtraLatencyMode;

        /// \unused parameter.
    ovrMatrix4f Reserved;

        /// \unused parameter.
    void* Reserved1;

    /// CPU/GPU performance parameters.
    ovrPerformanceParms PerformanceParms;

    /// For handling HMD events and power level state changes.
    ovrJava Java;
} ovrFrameParms);

// OVR_VRAPI_ASSERT_TYPE_SIZE_32_BIT(ovrFrameParms, 1856);
// OVR_VRAPI_ASSERT_TYPE_SIZE_64_BIT(ovrFrameParms, 1936);

//-------------------------------------
// Flexible Layer Type structures for vrapi_SubmitFrame2.
//-------------------------------------

enum { ovrMaxLayerCount = 16 };

/// A layer type.
typedef enum ovrLayerType2_ {
    VRAPI_LAYER_TYPE_PROJECTION2 = 1,
        VRAPI_LAYER_TYPE_CYLINDER2 = 3,
    VRAPI_LAYER_TYPE_CUBE2 = 4,
    VRAPI_LAYER_TYPE_EQUIRECT2 = 5,
    VRAPI_LAYER_TYPE_LOADING_ICON2 = 6,
    VRAPI_LAYER_TYPE_FISHEYE2 = 7,
        VRAPI_LAYER_TYPE_EQUIRECT3 = 10,
    } ovrLayerType2;


/// Properties shared by any type of layer.
typedef struct ovrLayerHeader2_ {
    ovrLayerType2 Type;
    /// Combination of ovrFrameLayerFlags flags.
    uint32_t Flags;

    ovrVector4f ColorScale;
    ovrFrameLayerBlend SrcBlend;
    ovrFrameLayerBlend DstBlend;
        /// \unused parameter.
    void* Reserved;
} ovrLayerHeader2;

OVR_VRAPI_ASSERT_TYPE_SIZE_32_BIT(ovrLayerHeader2, 36);
OVR_VRAPI_ASSERT_TYPE_SIZE_64_BIT(ovrLayerHeader2, 40);

/// ovrLayerProjection2 provides support for a typical world view layer.
/// \note Any layer textures that are dynamic must be triple buffered.
typedef struct ovrLayerProjection2_ {
    /// Header.Type must be VRAPI_LAYER_TYPE_PROJECTION2.
    ovrLayerHeader2 Header;
    OVR_VRAPI_PADDING_32_BIT(4)

    ovrRigidBodyPosef HeadPose;

    struct {
        ovrTextureSwapChain* ColorSwapChain;
        int SwapChainIndex;
        ovrMatrix4f TexCoordsFromTanAngles;
        ovrRectf TextureRect;
    } Textures[VRAPI_FRAME_LAYER_EYE_MAX];
} ovrLayerProjection2;

OVR_VRAPI_ASSERT_TYPE_SIZE_32_BIT(ovrLayerProjection2, 312);
OVR_VRAPI_ASSERT_TYPE_SIZE_64_BIT(ovrLayerProjection2, 328);






/// ovrLayerCylinder2 provides support for a single 2D texture projected onto a cylinder shape.
///
/// For Cylinder, the vertex coordinates will be transformed as if the texture type was CUBE.
/// Additionally, the interpolated vec3 will be remapped to vec2 by a direction-to-hemicyl mapping.
/// This mapping is currently hard-coded to 180 degrees around and 60 degrees vertical FOV.
///
/// After the mapping to 2D, an optional textureMatrix is applied. In the monoscopic case, the
/// matrix will typically be the identity matrix (ie no scale, bias). In the stereo case, when the
/// image source comes from a single image, the transform is necessary to map the [0.0,1.0] output
/// to a different (sub)rect.
///
/// Regardless of how the textureMatrix transforms the vec2 output of the equirect transform, each
/// TextureRect clamps the resulting texture coordinates so that no coordinates are beyond the
/// specified extents. No guarantees are made about whether fragments will be shaded outside the
/// rect, so it is important that the subrect have a transparent border.
///
typedef struct ovrLayerCylinder2_ {
    /// Header.Type must be VRAPI_LAYER_TYPE_CYLINDER2.
    ovrLayerHeader2 Header;
    OVR_VRAPI_PADDING_32_BIT(4)

    ovrRigidBodyPosef HeadPose;

    struct {
        /// Texture type used to create the swapchain must be a 2D target (VRAPI_TEXTURE_TYPE_2D_*).
        ovrTextureSwapChain* ColorSwapChain;
        int SwapChainIndex;
        ovrMatrix4f TexCoordsFromTanAngles;
        ovrRectf TextureRect;
        /// \note textureMatrix is set up like the following:
        /// sx,  0, tx, 0
        /// 0,  sy, ty, 0
        ///	0,   0,  1, 0
        ///	0,   0,  0, 1
        /// since we do not need z coord for mapping to 2d texture.
        ovrMatrix4f TextureMatrix;
    } Textures[VRAPI_FRAME_LAYER_EYE_MAX];
} ovrLayerCylinder2;

OVR_VRAPI_ASSERT_TYPE_SIZE_32_BIT(ovrLayerCylinder2, 440);
OVR_VRAPI_ASSERT_TYPE_SIZE_64_BIT(ovrLayerCylinder2, 456);

/// ovrLayerCube2 provides support for a single timewarped cubemap at infinity
/// with optional Offset vector (provided in normalized [-1.0,1.0] space).
///
/// Cube maps are an omni-directional layer source that are directly supported
/// by the graphics hardware. The nature of the cube map definition results in
/// higher resolution (in pixels per solid angle) at the corners and edges of
/// the cube and lower resolution at the center of each face. While the cube map
/// does have variability in sample density, the variability is spread symmetrically
/// around the sphere.
///
/// Sometimes it is valuable to have an omni-directional format that has a
/// directional bias where quality and sample density is better in a particular
/// direction or over a particular region. If we changed the cube map sampling
///
/// from:
///   color = texture( cubeLayerSampler, direction );
/// to:
///   color = texture( cubeLayerSampler, normalize( direction ) + offset );
///
/// we can provide a remapping of the cube map sample distribution such that
/// samples in the "offset" direction map to a smaller region of the cube map
/// (and are thus higher resolution).
///
/// A normal high resolution cube map can be resampled using the inverse of this
/// mapping to retain high resolution for one direction while signficantly reducing
/// the required size of the cube map.
///
typedef struct ovrLayerCube2_ {
    /// Header.Type must be VRAPI_LAYER_TYPE_CUBE2.
    ovrLayerHeader2 Header;
    OVR_VRAPI_PADDING_32_BIT(4)

    ovrRigidBodyPosef HeadPose;
    ovrMatrix4f TexCoordsFromTanAngles;

    ovrVector3f Offset;

    struct {
        /// Texture type used to create the swapchain must be a cube target
        /// (VRAPI_TEXTURE_TYPE_CUBE).
        ovrTextureSwapChain* ColorSwapChain;
        int SwapChainIndex;
    } Textures[VRAPI_FRAME_LAYER_EYE_MAX];
#ifdef __i386__
    uint32_t Padding;
#endif
} ovrLayerCube2;

OVR_VRAPI_ASSERT_TYPE_SIZE_32_BIT(ovrLayerCube2, 232);
OVR_VRAPI_ASSERT_TYPE_SIZE_64_BIT(ovrLayerCube2, 248);

/// ovrLayerEquirect2 provides support for a single Equirectangular texture at infinity.
///
/// For Equirectangular, the vertex coordinates will be transformed as if the texture type was CUBE,
/// and in the fragment shader, the interpolated vec3 will be remapped to vec2 by a
/// direction-to-equirect mapping.
///
/// After the mapping to 2D, an optional textureMatrix is applied. In the monoscopic case, the
/// matrix will typically be the identity matrix (ie no scale, bias). In the stereo case, when the
/// image source come from a single image, the transform is necessary to map the [0.0,1.0] output to
/// a different (sub)rect.
///
/// Regardless of how the textureMatrix transforms the vec2 output of the equirect transform, each
/// TextureRect clamps the resulting texture coordinates so that no coordinates are beyond the
/// specified extents. No guarantees are made about whether fragments will be shaded outside the
/// rect, so it is important that the subrect have a transparent border.
///
typedef struct ovrLayerEquirect2_ {
    /// Header.Type must be VRAPI_LAYER_TYPE_EQUIRECT2.
    ovrLayerHeader2 Header;
    OVR_VRAPI_PADDING_32_BIT(4)

    ovrRigidBodyPosef HeadPose;
    ovrMatrix4f TexCoordsFromTanAngles;

    struct {
        /// Texture type used to create the swapchain must be a 2D target (VRAPI_TEXTURE_TYPE_2D_*).
        ovrTextureSwapChain* ColorSwapChain;
        int SwapChainIndex;
        ovrRectf TextureRect;
        /// \note textureMatrix is set up like the following:
        ///	sx,  0, tx, 0
        ///	0,  sy, ty, 0
        ///	0,   0,  1, 0
        ///	0,   0,  0, 1
        /// since we do not need z coord for mapping to 2d texture.
        ovrMatrix4f TextureMatrix;
    } Textures[VRAPI_FRAME_LAYER_EYE_MAX];
} ovrLayerEquirect2;

OVR_VRAPI_ASSERT_TYPE_SIZE_32_BIT(ovrLayerEquirect2, 376);
OVR_VRAPI_ASSERT_TYPE_SIZE_64_BIT(ovrLayerEquirect2, 392);

/// ovrLayerEquirect3 provides support for a single Equirectangular texture at infinity or
/// with non-infinite radius at a specific location.
///
/// This layer is very similar to ovrLayerEquirect2; the main difference is that it allows
/// for the specification of TexCoordsFromTanAngles per-eye as well as a translation
/// (in meters) which is applied to the equirect's center and radius (in meters).
///
/// TexCoordsFromTanAngles.M[3][0..2] represent the translation of the equirect's center;
/// TexCoordsFromTanAngles.M[3][3] represents the radius of the equirect layer in meters
/// (0.0f is used for the infinite radius).
/// An example of setting the local equrect layer at 2 meters in front of the viewer with the
/// radius 1.5 meters is as follows:
///
///    ovrLayerEquirect3 layer = vrapi_DefaultLayerEquirect3();
///
///    const float radius = 1.5; // 1.5 m radius
///    layer.HeadPose = tracking->HeadPose;
///    ovrPosef pose = {};
///    pose.Position.x = 0.0f;
///    pose.Position.y = 0.0f;
///    pose.Position.z = -2.0f;
///    pose.Orientation.x = 0.0f;
///    pose.Orientation.y = 0.0f;
///    pose.Orientation.z = 0.0f;
///    pose.Orientation.w = 1.0f;
///
///    const ovrMatrix4f poseM = vrapi_GetTransformFromPose(&pose);
///
///    for (int eye = 0; eye < VRAPI_FRAME_LAYER_EYE_MAX; eye++) {
///        const ovrMatrix4f modelViewMatrix =
///            ovrMatrix4f_Multiply(&tracking->Eye[eye].ViewMatrix, &poseM);
///        ovrMatrix4f tex_coords_matrix = ovrMatrix4f_Inverse(&modelViewMatrix);
///        tex_coords_matrix.M[3][3] = radius;
///        layer.Textures[eye].TexCoordsFromTanAngles = tex_coords_matrix;
///        ....
typedef struct ovrLayerEquirect3_ {
    /// Header.Type must be VRAPI_LAYER_TYPE_EQUIRECT3.
    ovrLayerHeader2 Header;
    OVR_VRAPI_PADDING_32_BIT(4)

    ovrRigidBodyPosef HeadPose;

    struct {
        /// Texture type used to create the swapchain must be a 2D target (VRAPI_TEXTURE_TYPE_2D_*).
        ovrTextureSwapChain* ColorSwapChain;
        int SwapChainIndex;
        ovrMatrix4f TexCoordsFromTanAngles;
        ovrRectf TextureRect;
        /// \note textureMatrix is set up like the following:
        ///	sx,  0, tx, 0
        ///	0,  sy, ty, 0
        ///	0,   0,  1, 0
        ///	0,   0,  0, 1
        /// since we do not need z coord for mapping to 2d texture.
        ovrMatrix4f TextureMatrix;
    } Textures[VRAPI_FRAME_LAYER_EYE_MAX];
} ovrLayerEquirect3;

OVR_VRAPI_ASSERT_TYPE_SIZE_32_BIT(ovrLayerEquirect3, 440);
OVR_VRAPI_ASSERT_TYPE_SIZE_64_BIT(ovrLayerEquirect3, 456);

/// ovrLayerLoadingIcon2 provides support for a monoscopic spinning layer.
///
typedef struct ovrLayerLoadingIcon2_ {
    /// Header.Type must be VRAPI_LAYER_TYPE_LOADING_ICON2.
    ovrLayerHeader2 Header;

    float SpinSpeed; //< radians per second
    float SpinScale;

    /// Only monoscopic texture supported for spinning layer.
    ovrTextureSwapChain* ColorSwapChain;
    int SwapChainIndex;
} ovrLayerLoadingIcon2;

OVR_VRAPI_ASSERT_TYPE_SIZE_32_BIT(ovrLayerLoadingIcon2, 52);
OVR_VRAPI_ASSERT_TYPE_SIZE_64_BIT(ovrLayerLoadingIcon2, 64);

/// An "equiangular fisheye" or "f-theta" lens can be used to capture photos or video
/// of around 180 degrees without stitching.
///
/// The cameras probably aren't exactly vertical, so a transformation may need to be applied
/// before performing the fisheye calculation.
/// A stereo fisheye camera rig will usually have slight misalignments between the two
/// cameras, so they need independent transformations.
///
/// Once in lens space, the ray is transformed into an ideal fisheye projection, where the
/// 180 degree hemisphere is mapped to a -1 to 1 2D space.
///
/// From there it can be mapped into actual texture coordinates, possibly two to an image for
/// stereo.
///
typedef struct ovrLayerFishEye2_ {
    /// Header.Type must be VRAPI_LAYER_TYPE_FISHEYE2.
    ovrLayerHeader2 Header;
    OVR_VRAPI_PADDING_32_BIT(4)

    ovrRigidBodyPosef HeadPose;

    struct {
        ovrTextureSwapChain* ColorSwapChain;
        int SwapChainIndex;
        ovrMatrix4f LensFromTanAngles; //< transforms a tanAngle ray into lens space
        ovrRectf TextureRect; //< packed stereo images will need to clamp at the mid border
        ovrMatrix4f TextureMatrix; //< transform from a -1 to 1 ideal fisheye to the texture
        ovrVector4f Distortion; //< Not currently used.
    } Textures[VRAPI_FRAME_LAYER_EYE_MAX];
} ovrLayerFishEye2;

OVR_VRAPI_ASSERT_TYPE_SIZE_32_BIT(ovrLayerFishEye2, 472);
OVR_VRAPI_ASSERT_TYPE_SIZE_64_BIT(ovrLayerFishEye2, 488);


/// Union that combines ovrLayer types in a way that allows them
/// to be used in a polymorphic way.
typedef union ovrLayer_Union2_ {
    ovrLayerHeader2 Header;
    ovrLayerProjection2 Projection;
        ovrLayerCylinder2 Cylinder;
    ovrLayerCube2 Cube;
    ovrLayerEquirect2 Equirect;
    ovrLayerEquirect3 Equirect3;
    ovrLayerLoadingIcon2 LoadingIcon;
    ovrLayerFishEye2 FishEye;
    } ovrLayer_Union2;

/// Parameters for frame submission.
typedef struct ovrSubmitFrameDescription2_ {
    /// Combination of ovrFrameFlags flags.
    uint32_t Flags;
    uint32_t SwapInterval;
    uint64_t FrameIndex;
    double DisplayTime;
            /// \unused parameter.
        unsigned char Pad[8];
            uint32_t LayerCount;
    const ovrLayerHeader2* const* Layers;
} ovrSubmitFrameDescription2;

OVR_VRAPI_ASSERT_TYPE_SIZE_32_BIT(ovrSubmitFrameDescription2, 40);
OVR_VRAPI_ASSERT_TYPE_SIZE_64_BIT(ovrSubmitFrameDescription2, 48);

//-----------------------------------------------------------------
// Performance
//-----------------------------------------------------------------

/// Identifies a VR-related application thread.
typedef enum ovrPerfThreadType_ {
    VRAPI_PERF_THREAD_TYPE_MAIN = 0,
    VRAPI_PERF_THREAD_TYPE_RENDERER = 1,
} ovrPerfThreadType;


//-----------------------------------------------------------------
// Color Space Management
//-----------------------------------------------------------------
/// Color space types for HMDs
///
/// Until vrapi_SetClientColorDesc is called, the client will default to Rec2020 for Quest and
/// Rec709 for Go HMDs.
///
/// This API only handles color-space remapping. Unless specified, all color spaces use D65 white
/// point. It will not affect brightness, contrast or gamma curves. Some of these aspects such as
/// gamma, is handled by the texture format being used. From the GPU samplers' point-of-view, each
/// texture will continue to be treated as linear luminance including sRGB which is converted to
/// linear by the texture sampler.
///
/// 'VRAPI_COLORSPACE_UNMANAGED' will force the runtime to skip color correction for the provided
/// content. This is *not* recommended unless the app developer is sure about what they're doing.
/// 'VRAPI_COLORSPACE_UNMANAGED' is mostly useful for research & experimentation, but not for
/// software distribution. This is because unless the client is applying the necessary corrections
/// for each HMD type, the results seen in the HMD will be uncalibrated. This is especially true for
/// future HMDs where the color space is not yet known or defined, which could lead to colors that
/// look too dull, too saturated, or hue shifted.
///
/// Although native Quest and Rift CV1 color spaces are provided as options, they are not
/// standardized color spaces. While we provide the exact color space primary coordinates, for
/// better standardized visualized of authored content, it's recommended that the developers master
/// using a well-defined color space in the provided in the options such as Rec.2020.
///
/// It is also recommended that content be authored for the wider color spaces instead of Rec.709 to
/// prevent visuals from looking "washed out", "dull" or "desaturated" on wider gamut devices like
/// the Quest.
///
/// Unique Color Space Details with Chromaticity Primaries in CIE 1931 xy:
///
/// Color Space: P3, similar to DCI-P3, but using D65 white point instead.
/// Red  : (0.680, 0.320)
/// Green: (0.265, 0.690)
/// Blue : (0.150, 0.060)
/// White: (0.313, 0.329)
///
/// Color Space: Rift CV1 between P3 & Adobe RGB using D75 white point
/// Red  : (0.666, 0.334)
/// Green: (0.238, 0.714)
/// Blue : (0.139, 0.053)
/// White: (0.298, 0.318)
///
/// Color Space: Quest similar to Rift CV1 using D75 white point
/// Red  : (0.661, 0.338)
/// Green: (0.228, 0.718)
/// Blue : (0.142, 0.042)
/// White: (0.298, 0.318)
///
/// Color Space: Rift S similar to Rec 709 using D75
/// Red  : (0.640, 0.330)
/// Green: (0.292, 0.586)
/// Blue : (0.156, 0.058)
/// White: (0.298, 0.318)
///
/// Note: Due to LCD limitations, the Go display will not be able to meaningfully differentiate
/// brightness levels below 13 out of 255 for 8-bit sRGB or 0.0015 out of 1.0 max for linear-RGB
/// shader output values. To that end, it is recommended that reliance on a dark and narrow gamut is
/// avoided, and the content is instead spread across a larger brightness range when possible.
///
typedef enum ovrColorSpace_ {
    /// No color correction, not recommended for production use. See notes above for more info
    VRAPI_COLORSPACE_UNMANAGED = 0,
    /// Preferred color space for standardized color across all Oculus HMDs with D65 white point
    VRAPI_COLORSPACE_REC_2020 = 1,
    /// Rec. 709 is used on Oculus Go and shares the same primary color coordinates as sRGB
    VRAPI_COLORSPACE_REC_709 = 2,
    /// Oculus Rift CV1 uses a unique color space, see enum description for more info
    VRAPI_COLORSPACE_RIFT_CV1 = 3,
    /// Oculus Rift S uses a unique color space, see enum description for more info
    VRAPI_COLORSPACE_RIFT_S = 4,
    /// Oculus Quest's native color space is slightly different than Rift CV1
    VRAPI_COLORSPACE_QUEST = 5,
    /// Similar to DCI-P3. See notes above for more details on P3
    VRAPI_COLORSPACE_P3 = 6,
    /// Similar to sRGB but with deeper greens using D65 white point
    VRAPI_COLORSPACE_ADOBE_RGB = 7,
} ovrColorSpace;

typedef struct ovrHmdColorDesc_ {
    /// See ovrColorSpace for more info.
    ovrColorSpace ColorSpace;
    OVR_VRAPI_PADDING(4)
} ovrHmdColorDesc;

//-----------------------------------------------------------------
// Events
//-----------------------------------------------------------------

typedef enum ovrEventType_ {
    // No event. This is returned if no events are pending.
    VRAPI_EVENT_NONE = 0,
    // Events were lost due to event queue overflow.
    VRAPI_EVENT_DATA_LOST = 1,
    // The application's frames are visible to the user.
    VRAPI_EVENT_VISIBILITY_GAINED = 2,
    // The application's frames are no longer visible to the user.
    VRAPI_EVENT_VISIBILITY_LOST = 3,
    // The current activity is in the foreground and has input focus.
    VRAPI_EVENT_FOCUS_GAINED = 4,
    // The current activity is in the background (but possibly still visible) and has lost input
    // focus.
    VRAPI_EVENT_FOCUS_LOST = 5,
            // The display refresh rate has changed
    VRAPI_EVENT_DISPLAY_REFRESH_RATE_CHANGE = 11,
} ovrEventType;

typedef struct ovrEventHeader_ {
    ovrEventType EventType;
} ovrEventHeader;

// Event structure for VRAPI_EVENT_DATA_LOST
typedef struct ovrEventDataLost_ {
    ovrEventHeader EventHeader;
} ovrEventDataLost;

// Event structure for VRAPI_EVENT_VISIBILITY_GAINED
typedef struct ovrEventVisibilityGained_ {
    ovrEventHeader EventHeader;
} ovrEventVisibilityGained;

// Event structure for VRAPI_EVENT_VISIBILITY_LOST
typedef struct ovrEventVisibilityLost_ {
    ovrEventHeader EventHeader;
} ovrEventVisibilityLost;

// Event structure for VRAPI_EVENT_FOCUS_GAINED
typedef struct ovrEventFocusGained_ {
    ovrEventHeader EventHeader;
} ovrEventFocusGained;

// Event structure for VRAPI_EVENT_FOCUS_LOST
typedef struct ovrEventFocusLost_ {
    ovrEventHeader EventHeader;
} ovrEventFocusLost;

// Event structure for VRAPI_EVENT_DISPLAY_REFRESH_RATE_CHANGE
typedef struct ovrEventDisplayRefreshRateChange_ {
    ovrEventHeader EventHeader;
    float fromDisplayRefreshRate;
    float toDisplayRefreshRate;
} ovrEventDisplayRefreshRateChange;



typedef struct ovrEventDataBuffer_ {
    ovrEventHeader EventHeader;
    unsigned char EventData[4000];
} ovrEventDataBuffer;


#define VRAPI_LARGEST_EVENT_TYPE ovrEventDataBuffer

typedef enum ovrEventSize_ { VRAPI_MAX_EVENT_SIZE = sizeof(VRAPI_LARGEST_EVENT_TYPE) } ovrEventSize;

#endif // OVR_VrApi_Types_h
