use shaderc::{Compiler, ShaderKind};
use std::{env, fs};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    let mut compiler = Compiler::new().unwrap();

    let spirv_slicing_shader = compiler
        .compile_into_spirv(
            &fs::read_to_string("resources/slicing.glsl").unwrap(),
            ShaderKind::Fragment,
            "slicing.glsl",
            "main",
            None,
        )
        .unwrap();

    fs::write(
        format!("{}/slicing.spv", out_dir),
        spirv_slicing_shader.as_binary_u8(),
    )
    .unwrap();
}
