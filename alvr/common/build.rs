fn main() {
    println!("cargo:rustc-env=VERSION={}", alvr_xtask::version());
}
