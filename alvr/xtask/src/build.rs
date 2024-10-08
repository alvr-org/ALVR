use crate::command;
use alvr_filesystem::{self as afs, Layout};
use std::{
    env,
    fmt::{self, Display, Formatter},
    fs,
    path::PathBuf,
    vec,
};
use xshell::{cmd, Shell};

#[derive(Clone, Copy)]
pub enum Profile {
    Debug,
    Release,
    Distribution,
}

impl Display for Profile {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let string = match self {
            Profile::Distribution => "distribution",
            Profile::Release => "release",
            Profile::Debug => "debug",
        };
        write!(f, "{string}")
    }
}

pub fn build_server_lib(
    profile: Profile,
    enable_messagebox: bool,
    root: Option<String>,
    reproducible: bool,
) {
    let sh = Shell::new().unwrap();

    let mut flags = vec![];
    match profile {
        Profile::Distribution => {
            flags.push("--profile");
            flags.push("distribution");
        }
        Profile::Release => flags.push("--release"),
        Profile::Debug => (),
    }
    if enable_messagebox {
        flags.push("--features");
        flags.push("alvr_common/enable-messagebox");
    }
    if reproducible {
        flags.push("--locked");
    }
    let flags_ref = &flags;

    let artifacts_dir = afs::target_dir().join(profile.to_string());

    let build_dir = afs::build_dir().join("alvr_server_core");
    sh.create_dir(&build_dir).unwrap();

    if let Some(root) = root {
        sh.set_var("ALVR_ROOT_DIR", root);
    }

    let _push_guard = sh.push_dir(afs::crate_dir("server_core"));
    cmd!(sh, "cargo build {flags_ref...}").run().unwrap();

    sh.copy_file(
        artifacts_dir.join(afs::dynlib_fname("alvr_server_core")),
        &build_dir,
    )
    .unwrap();

    if cfg!(windows) {
        sh.copy_file(artifacts_dir.join("alvr_server_core.pdb"), &build_dir)
            .unwrap();
    }

    let out = build_dir.join("alvr_server_core.h");
    cmd!(sh, "cbindgen --output {out}").run().unwrap();
}

pub fn build_streamer(
    profile: Profile,
    enable_messagebox: bool,
    gpl: bool,
    root: Option<String>,
    reproducible: bool,
    profiling: bool,
    keep_config: bool,
) {
    let sh = Shell::new().unwrap();

    let build_layout = Layout::new(&afs::streamer_build_dir());

    let mut common_flags = vec![];
    match profile {
        Profile::Distribution => {
            common_flags.push("--profile");
            common_flags.push("distribution");
        }
        Profile::Release => common_flags.push("--release"),
        Profile::Debug => (),
    }
    if enable_messagebox {
        common_flags.push("--features");
        common_flags.push("alvr_common/enable-messagebox");
    }
    if reproducible {
        common_flags.push("--locked");
    }

    let artifacts_dir = if cfg!(all(windows, target_arch = "aarch64")) {
        // Fix for cross compilation
        const TARGET: &str = "x86_64-pc-windows-msvc";

        common_flags.push("--target");
        common_flags.push(TARGET);

        afs::target_dir().join(TARGET).join(profile.to_string())
    } else {
        afs::target_dir().join(profile.to_string())
    };

    let common_flags_ref = &common_flags;

    let maybe_config = if keep_config {
        fs::read_to_string(build_layout.session()).ok()
    } else {
        None
    };

    sh.remove_path(afs::streamer_build_dir()).ok();
    sh.create_dir(build_layout.openvr_driver_lib_dir()).unwrap();
    sh.create_dir(&build_layout.executables_dir).unwrap();

    if let Some(config) = maybe_config {
        fs::write(build_layout.session(), config).ok();
    }

    if let Some(root) = root {
        sh.set_var("ALVR_ROOT_DIR", root);
    }

    // build server
    {
        let gpl_flag = gpl.then(|| vec!["--features", "gpl"]).unwrap_or_default();
        let profiling_flag = profiling
            .then(|| vec!["--features", "alvr_server_core/trace-performance"])
            .unwrap_or_default();

        let _push_guard = sh.push_dir(afs::crate_dir("server_openvr"));
        cmd!(
            sh,
            "cargo build {common_flags_ref...} {gpl_flag...} {profiling_flag...}"
        )
        .run()
        .unwrap();

        sh.copy_file(
            artifacts_dir.join(afs::dynlib_fname("alvr_server_openvr")),
            build_layout.openvr_driver_lib(),
        )
        .unwrap();

        if cfg!(windows) {
            sh.copy_file(
                artifacts_dir.join("alvr_server_openvr.pdb"),
                build_layout
                    .openvr_driver_lib_dir()
                    .join("alvr_server_openvr.pdb"),
            )
            .unwrap();
        }
    }

    // Build dashboard
    {
        let _push_guard = sh.push_dir(afs::crate_dir("dashboard"));
        cmd!(sh, "cargo build {common_flags_ref...}").run().unwrap();

        sh.copy_file(
            artifacts_dir.join(afs::exec_fname("alvr_dashboard")),
            build_layout.dashboard_exe(),
        )
        .unwrap();
    }

    // copy dependencies
    if cfg!(windows) {
        sh.copy_file(
            afs::workspace_dir().join("openvr/bin/win64/openvr_api.dll"),
            build_layout.openvr_driver_lib_dir(),
        )
        .unwrap();

        // Bring along the c++ runtime
        command::copy_recursive(
            &sh,
            &afs::crate_dir("server_openvr").join("cpp/bin/windows"),
            &build_layout.openvr_driver_lib_dir(),
        )
        .unwrap();

        // copy ffmpeg binaries
        if gpl {
            let bin_dir = &build_layout.openvr_driver_lib_dir();
            sh.create_dir(bin_dir).unwrap();
            for lib_path in sh
                .read_dir(afs::deps_dir().join("windows/ffmpeg/bin"))
                .unwrap()
                .into_iter()
                .filter(|path| path.file_name().unwrap().to_string_lossy().contains(".dll"))
            {
                sh.copy_file(lib_path.clone(), bin_dir).unwrap();
            }
        }
    } else if cfg!(target_os = "linux") {
        // build compositor wrapper
        let _push_guard = sh.push_dir(afs::crate_dir("vrcompositor_wrapper"));
        cmd!(sh, "cargo build {common_flags_ref...}").run().unwrap();
        sh.create_dir(&build_layout.vrcompositor_wrapper_dir)
            .unwrap();
        sh.copy_file(
            artifacts_dir.join("alvr_vrcompositor_wrapper"),
            build_layout.vrcompositor_wrapper(),
        )
        .unwrap();
        sh.copy_file(
            artifacts_dir.join("alvr_drm_lease_shim.so"),
            build_layout.drm_lease_shim(),
        )
        .unwrap();

        // build vulkan layer
        let _push_guard = sh.push_dir(afs::crate_dir("vulkan_layer"));
        cmd!(sh, "cargo build {common_flags_ref...}").run().unwrap();
        sh.create_dir(&build_layout.libraries_dir).unwrap();
        sh.copy_file(
            artifacts_dir.join(afs::dynlib_fname("alvr_vulkan_layer")),
            build_layout.vulkan_layer(),
        )
        .unwrap();

        // copy vulkan layer manifest
        sh.create_dir(&build_layout.vulkan_layer_manifest_dir)
            .unwrap();
        sh.copy_file(
            afs::crate_dir("vulkan_layer").join("layer/alvr_x86_64.json"),
            build_layout.vulkan_layer_manifest(),
        )
        .unwrap();

        sh.copy_file(
            afs::workspace_dir().join("openvr/bin/linux64/libopenvr_api.so"),
            build_layout.openvr_driver_lib_dir(),
        )
        .unwrap();

        let firewall_script = afs::crate_dir("xtask").join("firewall/alvr_fw_config.sh");
        let firewalld = afs::crate_dir("xtask").join("firewall/alvr-firewalld.xml");
        let ufw = afs::crate_dir("xtask").join("firewall/ufw-alvr");

        // copy linux specific firewalls
        sh.copy_file(firewall_script, build_layout.firewall_script())
            .unwrap();
        sh.copy_file(firewalld, build_layout.firewalld_config())
            .unwrap();
        sh.copy_file(ufw, build_layout.ufw_config()).unwrap();
    }

    // copy static resources
    {
        // copy driver manifest
        sh.copy_file(
            afs::crate_dir("xtask").join("resources/driver.vrdrivermanifest"),
            build_layout.openvr_driver_manifest(),
        )
        .unwrap();
    }
}

pub fn build_launcher(profile: Profile, enable_messagebox: bool, reproducible: bool) {
    let sh = Shell::new().unwrap();

    let mut common_flags = vec![];
    match profile {
        Profile::Distribution => {
            common_flags.push("--profile");
            common_flags.push("distribution");
        }
        Profile::Release => common_flags.push("--release"),
        Profile::Debug => (),
    }
    if enable_messagebox {
        common_flags.push("--features");
        common_flags.push("alvr_common/enable-messagebox");
    }
    if reproducible {
        common_flags.push("--locked");
    }
    let common_flags_ref = &common_flags;

    sh.create_dir(afs::launcher_build_dir()).unwrap();

    cmd!(sh, "cargo build -p alvr_launcher {common_flags_ref...}")
        .run()
        .unwrap();

    sh.copy_file(
        afs::target_dir()
            .join(profile.to_string())
            .join(afs::exec_fname("alvr_launcher")),
        afs::launcher_build_exe_path(),
    )
    .unwrap();
}

fn build_android_lib_impl(dir_name: &str, profile: Profile, link_stdcpp: bool, all_targets: bool) {
    let sh = Shell::new().unwrap();

    let mut ndk_flags = vec!["--no-strip", "-p", "26", "-t", "arm64-v8a"];

    if all_targets {
        ndk_flags.extend(["-t", "armeabi-v7a", "-t", "x86_64", "-t", "x86"]);
    }

    let mut rust_flags = vec![];
    match profile {
        Profile::Distribution => {
            rust_flags.push("--profile");
            rust_flags.push("distribution")
        }
        Profile::Release => rust_flags.push("--release"),
        Profile::Debug => (),
    }
    if !link_stdcpp {
        rust_flags.push("--no-default-features");
    }
    let rust_flags_ref = &rust_flags;

    let build_dir = afs::build_dir().join(format!("alvr_{dir_name}"));
    sh.create_dir(&build_dir).unwrap();

    let _push_guard = sh.push_dir(afs::crate_dir(dir_name));
    cmd!(
        sh,
        "cargo ndk {ndk_flags...} -o {build_dir} build {rust_flags_ref...}"
    )
    .run()
    .unwrap();

    let out = build_dir.join(format!("alvr_{dir_name}.h"));
    cmd!(sh, "cbindgen --output {out}").run().unwrap();
}

pub fn build_android_client_core_lib(profile: Profile, link_stdcpp: bool, all_targets: bool) {
    build_android_lib_impl("client_core", profile, link_stdcpp, all_targets)
}

pub fn build_android_client_openxr_lib(profile: Profile, link_stdcpp: bool) {
    build_android_lib_impl("client_openxr", profile, link_stdcpp, false)
}

pub fn build_android_client(profile: Profile) {
    let sh = Shell::new().unwrap();

    let mut flags = vec![];
    match profile {
        Profile::Distribution => {
            flags.push("--profile");
            flags.push("distribution")
        }
        Profile::Release => flags.push("--release"),
        Profile::Debug => (),
    }
    let flags_ref = &flags;

    const ARTIFACT_NAME: &str = "alvr_client_android";

    let target_dir = afs::target_dir();
    let build_dir = afs::build_dir().join(ARTIFACT_NAME);
    sh.create_dir(&build_dir).unwrap();

    // Create debug keystore (signing will be overwritten by CI)
    if env::var(format!(
        "CARGO_APK_{}_KEYSTORE",
        profile.to_string().to_uppercase()
    ))
    .is_err()
        && matches!(profile, Profile::Release | Profile::Distribution)
    {
        let keystore_path = build_dir.join("debug.keystore");
        if !keystore_path.exists() {
            let keytool = PathBuf::from(env::var("JAVA_HOME").unwrap())
                .join("bin")
                .join(afs::exec_fname("keytool"));
            let pass = "alvrclient";

            let other = vec![
                "-genkey",
                "-v",
                "-alias",
                "androiddebugkey",
                "-dname",
                "CN=Android Debug,O=Android,C=US",
                "-keyalg",
                "RSA",
                "-keysize",
                "2048",
                "-validity",
                "10000",
            ];

            cmd!(
                sh,
                "{keytool} -keystore {keystore_path} -storepass {pass} -keypass {pass} {other...}"
            )
            .run()
            .unwrap();
        }
    }

    let _push_guard = sh.push_dir(afs::crate_dir("client_openxr"));
    cmd!(
        sh,
        "cargo apk build --target-dir={target_dir} {flags_ref...}"
    )
    .run()
    .unwrap();

    sh.copy_file(
        afs::target_dir()
            .join(profile.to_string())
            .join("apk/alvr_client_openxr.apk"),
        build_dir.join(format!("{ARTIFACT_NAME}.apk")),
    )
    .unwrap();
}
