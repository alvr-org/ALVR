use alvr_xtask::*;

fn main() {
    println!("cargo:rustc-env=SERVER_VERSION={}", server_version());
    println!("cargo:rustc-env=CLIENT_VERSION={}", client_version());
}
