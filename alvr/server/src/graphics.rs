static FRAME_RENDER_VS_CSO: &[u8] = include_bytes!("../cpp/platform/win32/FrameRenderVS.cso");
static FRAME_RENDER_PS_CSO: &[u8] = include_bytes!("../cpp/platform/win32/FrameRenderPS.cso");
static QUAD_SHADER_CSO: &[u8] = include_bytes!("../cpp/platform/win32/QuadVertexShader.cso");
static COMPRESS_AXIS_ALIGNED_CSO: &[u8] =
    include_bytes!("../cpp/platform/win32/CompressAxisAlignedPixelShader.cso");
static COLOR_CORRECTION_CSO: &[u8] =
    include_bytes!("../cpp/platform/win32/ColorCorrectionPixelShader.cso");
static RGBTOYUV420_CSO: &[u8] = include_bytes!("../cpp/platform/win32/rgbtoyuv420.cso");

static QUAD_SHADER_COMP_SPV: &[u8] = include_bytes!("../cpp/platform/linux/shader/quad.comp.spv");
static COLOR_SHADER_COMP_SPV: &[u8] = include_bytes!("../cpp/platform/linux/shader/color.comp.spv");
static FFR_SHADER_COMP_SPV: &[u8] = include_bytes!("../cpp/platform/linux/shader/ffr.comp.spv");
static RGBTOYUV420_SHADER_COMP_SPV: &[u8] =
    include_bytes!("../cpp/platform/linux/shader/rgbtoyuv420.comp.spv");

pub fn initialize_shaders() {
    unsafe {
        crate::FRAME_RENDER_VS_CSO_PTR = FRAME_RENDER_VS_CSO.as_ptr();
        crate::FRAME_RENDER_VS_CSO_LEN = FRAME_RENDER_VS_CSO.len() as _;
        crate::FRAME_RENDER_PS_CSO_PTR = FRAME_RENDER_PS_CSO.as_ptr();
        crate::FRAME_RENDER_PS_CSO_LEN = FRAME_RENDER_PS_CSO.len() as _;
        crate::QUAD_SHADER_CSO_PTR = QUAD_SHADER_CSO.as_ptr();
        crate::QUAD_SHADER_CSO_LEN = QUAD_SHADER_CSO.len() as _;
        crate::COMPRESS_AXIS_ALIGNED_CSO_PTR = COMPRESS_AXIS_ALIGNED_CSO.as_ptr();
        crate::COMPRESS_AXIS_ALIGNED_CSO_LEN = COMPRESS_AXIS_ALIGNED_CSO.len() as _;
        crate::COLOR_CORRECTION_CSO_PTR = COLOR_CORRECTION_CSO.as_ptr();
        crate::COLOR_CORRECTION_CSO_LEN = COLOR_CORRECTION_CSO.len() as _;
        crate::RGBTOYUV420_CSO_PTR = RGBTOYUV420_CSO.as_ptr();
        crate::RGBTOYUV420_CSO_LEN = RGBTOYUV420_CSO.len() as _;
        crate::QUAD_SHADER_COMP_SPV_PTR = QUAD_SHADER_COMP_SPV.as_ptr();
        crate::QUAD_SHADER_COMP_SPV_LEN = QUAD_SHADER_COMP_SPV.len() as _;
        crate::COLOR_SHADER_COMP_SPV_PTR = COLOR_SHADER_COMP_SPV.as_ptr();
        crate::COLOR_SHADER_COMP_SPV_LEN = COLOR_SHADER_COMP_SPV.len() as _;
        crate::FFR_SHADER_COMP_SPV_PTR = FFR_SHADER_COMP_SPV.as_ptr();
        crate::FFR_SHADER_COMP_SPV_LEN = FFR_SHADER_COMP_SPV.len() as _;
        crate::RGBTOYUV420_SHADER_COMP_SPV_PTR = RGBTOYUV420_SHADER_COMP_SPV.as_ptr();
        crate::RGBTOYUV420_SHADER_COMP_SPV_LEN = RGBTOYUV420_SHADER_COMP_SPV.len() as _;
    }
}
