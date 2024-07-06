#![allow(unused_variables)]

#[cfg(all(target_os = "android", feature = "use-cpp"))]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
