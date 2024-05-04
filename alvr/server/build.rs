use std::{env, fs, path::PathBuf};

mod build_shaders;
use build_shaders::SHADERS;
#[cfg(target_os = "windows")]
use regex::{Captures, Regex};
#[cfg(target_os = "windows")]
use windows::{core::PCSTR, Win32::Graphics::Direct3D::Fxc::D3DCompile};

fn get_ffmpeg_path() -> PathBuf {
    let ffmpeg_path = alvr_filesystem::deps_dir()
        .join(if cfg!(target_os = "linux") {
            "linux"
        } else {
            "windows"
        })
        .join("ffmpeg");

    if cfg!(target_os = "linux") {
        ffmpeg_path.join("alvr_build")
    } else {
        ffmpeg_path
    }
}

#[cfg(all(target_os = "linux", feature = "gpl"))]
fn get_linux_x264_path() -> PathBuf {
    alvr_filesystem::deps_dir().join("linux/x264/alvr_build")
}

fn compile_shaders(platform_name: &str, platform_subpath: &str) {
    let shader_dir = PathBuf::from(platform_subpath).join("shader");

    match platform_name {
        #[cfg(target_os = "windows")]
        "windows" => {
            for shader in SHADERS {
                let source_path = shader_dir.join(shader.source_file);
                let source = fs::read_to_string(&source_path).unwrap();

                let re = Regex::new("#include \"(.*)\"").unwrap();

                let source = re.replace_all(source.as_str(), |caps: &Captures| {
                    let include_path = shader_dir.join(caps.get(1).unwrap().as_str());
                    let include_source = fs::read_to_string(include_path).unwrap();
                    include_source
                });

                fn to_pcstr(s: &str) -> PCSTR {
                    PCSTR::from_raw([s, "\0"].join("").as_ptr())
                }

                let mut out_shader = None;
                let mut out_errors = None;
                let out_shader = unsafe {
                    D3DCompile(
                        source.as_bytes().as_ptr() as _,
                        source.as_bytes().len(),
                        to_pcstr(shader.source_file),
                        None,
                        None,
                        shader.entry_point,
                        shader.profile,
                        0,
                        0,
                        &mut out_shader,
                        Some(&mut out_errors),
                    )
                }
                .map(|()| out_shader);

                match out_shader {
                    Err(err) => {
                        if let Some(out_errors) = out_errors {
                            let error_data = unsafe {
                                let ptr = out_errors.GetBufferPointer();
                                let size = out_errors.GetBufferSize();
                                std::slice::from_raw_parts(ptr as *const u8, size as usize)
                            };
                            println!(
                                "Shader compilation error for \"{}\": {}",
                                shader.source_file,
                                error_data.iter().map(|&b| b as char).collect::<String>()
                            );
                        }
                        panic!("Shader compilation failed: {}", err);
                    }
                    Ok(_) => {
                        let dxil = out_shader.unwrap().unwrap();
                        let out_buf = unsafe {
                            let ptr = dxil.GetBufferPointer();
                            let size = dxil.GetBufferSize();
                            std::slice::from_raw_parts(ptr as *const u8, size as usize)
                        };

                        let out_path = shader_dir.join(shader.out_file);
                        fs::write(out_path, out_buf).unwrap();
                    }
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        "linux" => {
            let compiler = shaderc::Compiler::new().unwrap();
            let options = shaderc::CompileOptions::new().unwrap();

            for shader in SHADERS {
                let source_path = shader_dir.join(shader.source_file);
                let source = fs::read_to_string(source_path).unwrap();

                let binary_result = compiler
                    .compile_into_spirv(
                        source.as_str(),
                        shader.kind,
                        shader.source_file,
                        shader.entry_point,
                        Some(&options),
                    )
                    .unwrap();
                let out_path = shader_dir.join(shader.out_file);
                fs::write(out_path, binary_result.as_binary_u8()).unwrap();
            }
        }
        "macos" => {}
        _ => panic!(),
    }
}

fn main() {
    let platform_name = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let cpp_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("cpp");

    let platform_subpath = match platform_name.as_str() {
        "windows" => "cpp/platform/win32",
        "linux" => "cpp/platform/linux",
        "macos" => "cpp/platform/macos",
        _ => panic!(),
    };

    let common_iter = walkdir::WalkDir::new("cpp")
        .into_iter()
        .filter_entry(|entry| {
            entry.file_name() != "tools"
                && entry.file_name() != "platform"
                && (platform_name != "macos" || entry.file_name() != "amf")
                && (platform_name != "linux" || entry.file_name() != "amf")
        });

    let platform_iter = walkdir::WalkDir::new(platform_subpath).into_iter();

    let cpp_paths = common_iter
        .chain(platform_iter)
        .filter_map(|maybe_entry| maybe_entry.ok())
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();

    let source_files_paths = cpp_paths.iter().filter(|path| {
        path.extension()
            .filter(|ext| {
                let ext_str = ext.to_string_lossy();
                ext_str == "c" || ext_str == "cpp"
            })
            .is_some()
    });

    // compile shaders
    compile_shaders(&platform_name, platform_subpath);

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .files(source_files_paths)
        .flag_if_supported("-isystemcpp/openvr/headers") // silences many warnings from openvr headers
        .flag_if_supported("-std=c++17")
        .include("cpp/openvr/headers")
        .include("cpp");

    if platform_name == "windows" {
        build
            .debug(false) // This is because we cannot link to msvcrtd (see below)
            .flag("/std:c++17")
            .flag("/permissive-")
            .define("NOMINMAX", None)
            .define("_WINSOCKAPI_", None)
            .define("_MBCS", None)
            .define("_MT", None);
    } else if platform_name == "macos" {
        build.define("__APPLE__", None);
    }

    // #[cfg(debug_assertions)]
    // build.define("ALVR_DEBUG_LOG", None);

    let gpl_or_linux = cfg!(feature = "gpl") || cfg!(target_os = "linux");

    if gpl_or_linux {
        let ffmpeg_path = get_ffmpeg_path();

        assert!(ffmpeg_path.join("include").exists());
        build.include(ffmpeg_path.join("include"));
    }

    #[cfg(all(target_os = "linux", feature = "gpl"))]
    {
        let x264_path = get_linux_x264_path();

        assert!(x264_path.join("include").exists());
        build.include(x264_path.join("include"));
    }

    #[cfg(feature = "gpl")]
    build.define("ALVR_GPL", None);

    build.compile("bindings");

    #[cfg(all(target_os = "linux", feature = "gpl"))]
    {
        let x264_path = get_linux_x264_path();
        let x264_lib_path = x264_path.join("lib");

        println!(
            "cargo:rustc-link-search=native={}",
            x264_lib_path.to_string_lossy()
        );

        let x264_pkg_path = x264_lib_path.join("pkgconfig");
        assert!(x264_pkg_path.exists());

        let x264_pkg_path = x264_pkg_path.to_string_lossy().to_string();
        env::set_var(
            "PKG_CONFIG_PATH",
            env::var("PKG_CONFIG_PATH").map_or(x264_pkg_path.clone(), |old| {
                format!("{x264_pkg_path}:{old}")
            }),
        );
        println!("cargo:rustc-link-lib=static=x264");

        pkg_config::Config::new()
            .statik(true)
            .probe("x264")
            .unwrap();
    }

    // ffmpeg
    if gpl_or_linux {
        let ffmpeg_path = get_ffmpeg_path();
        let ffmpeg_lib_path = ffmpeg_path.join("lib");

        assert!(ffmpeg_lib_path.exists());

        println!(
            "cargo:rustc-link-search=native={}",
            ffmpeg_lib_path.to_string_lossy()
        );

        #[cfg(target_os = "linux")]
        {
            let ffmpeg_pkg_path = ffmpeg_lib_path.join("pkgconfig");
            assert!(ffmpeg_pkg_path.exists());

            let ffmpeg_pkg_path = ffmpeg_pkg_path.to_string_lossy().to_string();
            env::set_var(
                "PKG_CONFIG_PATH",
                env::var("PKG_CONFIG_PATH").map_or(ffmpeg_pkg_path.clone(), |old| {
                    format!("{ffmpeg_pkg_path}:{old}")
                }),
            );

            let pkg = pkg_config::Config::new().statik(true).to_owned();

            for lib in ["libavutil", "libavfilter", "libavcodec"] {
                pkg.probe(lib).unwrap();
            }
        }
        #[cfg(windows)]
        for lib in ["avutil", "avfilter", "avcodec", "swscale"] {
            println!("cargo:rustc-link-lib={lib}");
        }
    }

    bindgen::builder()
        .clang_arg("-xc++")
        .header("cpp/alvr_server/bindings.h")
        .derive_default(true)
        .generate()
        .unwrap()
        .write_to_file(out_dir.join("bindings.rs"))
        .unwrap();

    if platform_name != "macos" {
        println!(
            "cargo:rustc-link-search=native={}",
            cpp_dir.join("openvr/lib").to_string_lossy()
        );
        println!("cargo:rustc-link-lib=openvr_api");
    }

    #[cfg(target_os = "linux")]
    {
        pkg_config::Config::new().probe("vulkan").unwrap();

        #[cfg(not(feature = "gpl"))]
        {
            pkg_config::Config::new().probe("x264").unwrap();
        }

        // fail build if there are undefined symbols in final library
        println!("cargo:rustc-cdylib-link-arg=-Wl,--no-undefined");
    }

    for path in cpp_paths {
        println!("cargo:rerun-if-changed={}", path.to_string_lossy());
    }
}
