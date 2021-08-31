#[cfg(target_os = "android")]
mod mediacodec;
#[cfg(not(target_os = "android"))]
mod vulkan;

#[cfg(target_os = "android")]
pub use mediacodec::*;
#[cfg(not(target_os = "android"))]
pub use vulkan::*;
