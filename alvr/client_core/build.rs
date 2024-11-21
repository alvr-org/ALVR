fn main() {
    let platform_name = std::env::var("CARGO_CFG_TARGET_OS").unwrap();

    if platform_name == "android" {
        println!("cargo:rustc-link-lib=log");
        println!("cargo:rustc-link-lib=EGL");
        println!("cargo:rustc-link-lib=GLESv3");
        println!("cargo:rustc-link-lib=android");

        #[cfg(feature = "link-stdcpp-shared")]
        println!("cargo:rustc-link-lib=c++_shared");
    }
}
