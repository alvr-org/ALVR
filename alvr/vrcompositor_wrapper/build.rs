#[cfg(target_os = "linux")]
fn main() {
    use std::{env, path::PathBuf};
    use xshell::{cmd, Shell};

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target_dir = out_dir.join("../../..");

    let sh = Shell::new().unwrap();
    let command = format!("g++ -shared -fPIC $(pkg-config --cflags libdrm) drm-lease-shim.cpp -o {}/alvr_drm_lease_shim.so", target_dir.display());
    cmd!(sh, "bash -c {command}").run().unwrap();
}

#[cfg(not(target_os = "linux"))]
fn main() {}
