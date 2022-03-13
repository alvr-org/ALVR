use crate::{command, version};
use alvr_filesystem::{self as afs, Layout};
use fs_extra::{self as fsx, dir as dirx};
use std::{env, fs};

pub fn build_server(
    is_release: bool,
    gpl: bool,
    root: Option<String>,
    reproducible: bool,
    experiments: bool,
) {
    let build_layout = Layout::new(&afs::server_build_dir());

    let build_type = if is_release { "release" } else { "debug" };

    let release_flag = if is_release { "--release" } else { "" };
    let reproducible_flag = if reproducible { "--locked" } else { "" };
    let gpl_flag = if gpl { "--features gpl" } else { "" };

    if let Some(root) = root {
        env::set_var("ALVR_ROOT_DIR", root);
    }

    let target_dir = afs::target_dir();
    let artifacts_dir = target_dir.join(build_type);

    fs::remove_dir_all(&afs::server_build_dir()).ok();
    fs::create_dir_all(&afs::server_build_dir()).unwrap();
    fs::create_dir_all(&build_layout.openvr_driver_lib().parent().unwrap()).unwrap();
    fs::create_dir_all(&build_layout.launcher_exe().parent().unwrap()).unwrap();

    let mut copy_options = dirx::CopyOptions::new();
    copy_options.copy_inside = true;
    fsx::copy_items(
        &[afs::workspace_dir().join("alvr/xtask/resources/presets")],
        build_layout.presets_dir(),
        &copy_options,
    )
    .unwrap();

    if gpl && cfg!(target_os = "linux") {
        fs::create_dir_all(&build_layout.openvr_driver_root_dir).unwrap();
        for lib in walkdir::WalkDir::new(afs::deps_dir().join("linux/ffmpeg"))
            .into_iter()
            .filter_map(|maybe_entry| maybe_entry.ok())
            .map(|entry| entry.into_path())
            .filter(|path| path.file_name().unwrap().to_string_lossy().contains(".so."))
        {
            fs::copy(
                lib.clone(),
                build_layout
                    .openvr_driver_root_dir
                    .join(lib.file_name().unwrap()),
            )
            .unwrap();
        }
    }

    if gpl && cfg!(windows) {
        let bin_dir = &build_layout.openvr_driver_lib_dir();
        fs::create_dir_all(bin_dir).unwrap();
        for dll in walkdir::WalkDir::new(afs::deps_dir().join("windows/ffmpeg/bin"))
            .into_iter()
            .filter_map(|maybe_entry| maybe_entry.ok())
            .map(|entry| entry.into_path())
            .filter(|path| path.file_name().unwrap().to_string_lossy().contains(".dll"))
        {
            fs::copy(dll.clone(), bin_dir.join(dll.file_name().unwrap())).unwrap();
        }
    }

    if cfg!(target_os = "linux") {
        command::run_in(
            &afs::workspace_dir().join("alvr/vrcompositor-wrapper"),
            &format!("cargo build {release_flag} {reproducible_flag}"),
        )
        .unwrap();
        fs::create_dir_all(&build_layout.vrcompositor_wrapper_dir).unwrap();
        fs::copy(
            artifacts_dir.join("vrcompositor-wrapper"),
            build_layout.vrcompositor_wrapper(),
        )
        .unwrap();
    }

    command::run_in(
        &afs::workspace_dir().join("alvr/server"),
        &format!("cargo build {release_flag} {reproducible_flag} {gpl_flag}"),
    )
    .unwrap();
    fs::copy(
        artifacts_dir.join(afs::dynlib_fname("alvr_server")),
        build_layout.openvr_driver_lib(),
    )
    .unwrap();

    command::run_in(
        &afs::workspace_dir().join("alvr/launcher"),
        &format!("cargo build {release_flag} {reproducible_flag}"),
    )
    .unwrap();
    fs::copy(
        artifacts_dir.join(afs::exec_fname("alvr_launcher")),
        build_layout.launcher_exe(),
    )
    .unwrap();

    if experiments {
        let dir_content = dirx::get_dir_content2(
            "alvr/experiments/gui/resources/languages",
            &dirx::DirOptions { depth: 1 },
        )
        .unwrap();
        let items: Vec<&String> = dir_content.directories[1..]
            .iter()
            .chain(dir_content.files.iter())
            .collect();

        let destination = afs::server_build_dir().join("languages");
        fs::create_dir_all(&destination).unwrap();
        fsx::copy_items(&items, destination, &dirx::CopyOptions::new()).unwrap();

        command::run_in(
            &afs::workspace_dir().join("alvr/experiments/launcher"),
            &format!("cargo build {release_flag}"),
        )
        .unwrap();
        fs::copy(
            artifacts_dir.join(afs::exec_fname("launcher")),
            build_layout
                .executables_dir
                .join(afs::exec_fname("experimental_launcher")),
        )
        .unwrap();
    }

    fs::copy(
        afs::workspace_dir().join("alvr/xtask/resources/driver.vrdrivermanifest"),
        build_layout.openvr_driver_manifest(),
    )
    .unwrap();

    if cfg!(windows) {
        let dir_content = dirx::get_dir_content("alvr/server/cpp/bin/windows").unwrap();
        fsx::copy_items(
            &dir_content.files,
            build_layout.openvr_driver_lib().parent().unwrap(),
            &dirx::CopyOptions::new(),
        )
        .unwrap();
    }

    let dir_content = dirx::get_dir_content2(
        afs::workspace_dir().join("alvr/dashboard"),
        &dirx::DirOptions { depth: 1 },
    )
    .unwrap();
    let items: Vec<&String> = dir_content.directories[1..]
        .iter()
        .chain(dir_content.files.iter())
        .collect();

    fs::create_dir_all(&build_layout.dashboard_dir()).unwrap();
    fsx::copy_items(
        &items,
        build_layout.dashboard_dir(),
        &dirx::CopyOptions::new(),
    )
    .unwrap();

    if cfg!(target_os = "linux") {
        command::run_in(
            &afs::workspace_dir().join("alvr/vulkan-layer"),
            &format!("cargo build {release_flag} {reproducible_flag}"),
        )
        .unwrap();

        fs::create_dir_all(&build_layout.vulkan_layer_manifest_dir).unwrap();
        fs::create_dir_all(&build_layout.libraries_dir).unwrap();
        fs::copy(
            afs::workspace_dir().join("alvr/vulkan-layer/layer/alvr_x86_64.json"),
            build_layout
                .vulkan_layer_manifest_dir
                .join("alvr_x86_64.json"),
        )
        .unwrap();
        fs::copy(
            artifacts_dir.join(afs::dynlib_fname("alvr_vulkan_layer")),
            build_layout
                .libraries_dir
                .join(afs::dynlib_fname("alvr_vulkan_layer")),
        )
        .unwrap();
    }
}

pub fn build_client(is_release: bool, headset_name: &str) {
    let is_nightly = version::version().contains("nightly");
    let is_release = if is_nightly { false } else { is_release };

    let headset_type = match headset_name {
        "oculus_go" => "OculusGo",
        "oculus_quest" => "OculusQuest",
        _ => {
            panic!("Unrecognized platform.");
        }
    };
    let package_type = if is_nightly { "Nightly" } else { "Stable" };
    let build_type = if is_release { "release" } else { "debug" };

    let build_task = format!("assemble{headset_type}{package_type}{build_type}");

    let client_dir = afs::workspace_dir().join("alvr/client/android");
    let command_name = if cfg!(not(windows)) {
        "./gradlew"
    } else {
        "gradlew.bat"
    };

    let artifact_name = format!("alvr_client_{headset_name}");
    fs::create_dir_all(&afs::build_dir().join(&artifact_name)).unwrap();

    env::set_current_dir(&client_dir).unwrap();
    command::run(&format!("{command_name} {build_task}")).unwrap();
    env::set_current_dir(afs::workspace_dir()).unwrap();

    fs::copy(
        client_dir
            .join("app/build/outputs/apk")
            .join(format!("{headset_type}{package_type}"))
            .join(build_type)
            .join(format!(
                "app-{headset_type}-{package_type}-{build_type}.apk",
            )),
        afs::build_dir()
            .join(&artifact_name)
            .join(format!("{artifact_name}.apk")),
    )
    .unwrap();
}
