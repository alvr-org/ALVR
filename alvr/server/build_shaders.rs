#[cfg(not(target_os = "windows"))]
use shaderc::ShaderKind;
#[cfg(target_os = "windows")]
use windows::core::{s, PCSTR};

#[cfg(not(target_os = "windows"))]
pub(crate) struct Shader {
    pub(crate) source_file: &'static str,
    pub(crate) out_file: &'static str,
    pub(crate) entry_point: &'static str,
    pub(crate) kind: ShaderKind,
}

#[cfg(target_os = "linux")]
pub const SHADERS: [Shader; 4] = [
    Shader {
        source_file: "color.comp",
        out_file: "color.comp.spv",
        entry_point: "main",
        kind: ShaderKind::Compute,
    },
    Shader {
        source_file: "ffr.comp",
        out_file: "ffr.comp.spv",
        entry_point: "main",
        kind: ShaderKind::Compute,
    },
    Shader {
        source_file: "quad.comp",
        out_file: "quad.comp.spv",
        entry_point: "main",
        kind: ShaderKind::Compute,
    },
    Shader {
        source_file: "rgbtoyuv420.comp",
        out_file: "rgbtoyuv420.comp.spv",
        entry_point: "main",
        kind: ShaderKind::Compute,
    },
];

#[cfg(target_os = "windows")]
pub(crate) struct Shader {
    pub(crate) source_file: &'static str,
    pub(crate) out_file: &'static str,
    pub(crate) entry_point: PCSTR,
    pub(crate) profile: PCSTR,
}

#[cfg(target_os = "windows")]
pub const SHADERS: [Shader; 6] = [
    Shader {
        source_file: "ColorCorrectionPixelShader.hlsl",
        out_file: "ColorCorrectionPixelShader.cso",
        entry_point: s!("main"),
        profile: s!("ps_5_0"),
    },
    Shader {
        source_file: "CompressAxisAlignedPixelShader.hlsl",
        out_file: "CompressAxisAlignedPixelShader.cso",
        entry_point: s!("main"),
        profile: s!("ps_5_0"),
    },
    Shader {
        source_file: "FrameRenderPS.hlsl",
        out_file: "FrameRenderPS.cso",
        entry_point: s!("PS"),
        profile: s!("ps_5_0"),
    },
    Shader {
        source_file: "FrameRenderVS.hlsl",
        out_file: "FrameRenderVS.cso",
        entry_point: s!("VS"),
        profile: s!("vs_5_0"),
    },
    Shader {
        source_file: "QuadVertexShader.hlsl",
        out_file: "QuadVertexShader.cso",
        entry_point: s!("main"),
        profile: s!("vs_5_0"),
    },
    Shader {
        source_file: "rgbtoyuv420.hlsl",
        out_file: "rgbtoyuv420.cso",
        entry_point: s!("main"),
        profile: s!("ps_5_0"),
    },
];

#[cfg(target_os = "macos")]
pub const SHADERS: [Shader; 0] = [];
