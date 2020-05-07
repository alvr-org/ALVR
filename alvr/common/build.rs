fn main() {

    println!(
        "cargo:rustc-env=SERVER_VERSION={}",
        alvr_xtask::server_version()
    );

    // println!(
    //     "cargo:rustc-env=CLIENT_VERSION={}",
    //     alvr_xtask::client_version()
    // );
}
