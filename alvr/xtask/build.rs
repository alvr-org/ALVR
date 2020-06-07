fn main() {
    // for runas
    #[cfg(windows)]
    println!("cargo:rustc-link-lib=shell32");
}
