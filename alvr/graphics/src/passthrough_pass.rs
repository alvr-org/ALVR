// use crate::GL_TEXTURE_EXTERNAL_OES;
// use alvr_common::{
//     anyhow::{anyhow, bail, Result},
//     glam::UVec2,
//     ToAny,
// };
// use ash::vk;
// use std::rc::Rc;
// use wgpu::hal;

// fn to_any<T>(res: Result<T, String>) -> Result<T> {
//     res.map_err(|e| anyhow!(e))
// }

// // fn create_shader(gl: &gl::Context, ty: u32, source: &str) -> Result<gl::Shader> {
// //     unsafe {
// //         let shader = to_any(gl.create_shader(ty))?;
// //         gl.shader_source(shader, source);
// //         gl.compile_shader(shader);

// //         if !gl.get_shader_compile_status(shader) {
// //             let log = gl.get_shader_info_log(shader);
// //             bail!("Shader compilation failed: {}", log);
// //         }

// //         Ok(shader)
// //     }
// // }

// pub struct Resources {
//     input: vk::Image,
//     _output: wgpu::TextureView,
// }

// pub struct PassthroughPass {
//     // gl: Rc<gl::Context>,
//     resolution: UVec2,
//     vertex_shader: gl::Shader,
//     fragment_shader: gl::Shader,
//     program: gl::Program,
//     texture_location: gl::UniformLocation,
//     resource_swapchain: Vec<Resources>,
// }

// impl PassthroughPass {
//     pub fn new(
//         gl_context: Rc<gl::Context>,
//         output_swapchain: &[&wgpu::Texture],
//         resolution: UVec2,
//         is_passthrough: bool,
//     ) -> Result<Self> {
//         unsafe {
//             let gl = gl_context;

//             let program = to_any(gl.create_program())?;
//             let vertex_shader = create_shader(
//                 &gl,
//                 gl::VERTEX_SHADER,
//                 include_str!("../shaders/client/srgb_vert.glsl"),
//             )?;
//             gl.attach_shader(program, vertex_shader);
//             let fragment_shader = create_shader(
//                 &gl,
//                 gl::FRAGMENT_SHADER,
//                 include_str!("../shaders/client/srgb_frag.glsl"),
//             )?;
//             gl.attach_shader(program, fragment_shader);
//             gl.link_program(program);
//             if !gl.get_program_link_status(program) {
//                 let log = gl.get_program_info_log(program);
//                 bail!("Program linking failed: {}", log);
//             }
//             let texture_location = gl.get_uniform_location(program, "texture").to_any()?;

//             let resource_swapchain = output_swapchain
//                 .iter()
//                 .map(|output| {
//                     let input = gl.create_texture().unwrap();
//                     // todo: check if needed
//                     // gl.tex_parameter_i32(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
//                     // gl.tex_parameter_i32(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
//                     // gl.tex_parameter_i32(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
//                     // gl.tex_parameter_i32(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);

//                     let mut output_texture = None;
//                     let mut output_target = None;

//                     output.as_hal::<hal::api::Gles, _>(|raw_tex| {
//                         let hal::gles::TextureInner::Texture { raw, target } =
//                             raw_tex.unwrap().inner
//                         else {
//                             panic!("Unexpected texture type");
//                         };

//                         output_texture = Some(raw);
//                         output_target = Some(target);
//                     });

//                     let framebuffer = gl.create_framebuffer().unwrap();
//                     gl.bind_framebuffer(gl::DRAW_FRAMEBUFFER, Some(framebuffer));
//                     gl.framebuffer_texture_2d(
//                         gl::DRAW_FRAMEBUFFER,
//                         gl::COLOR_ATTACHMENT0,
//                         output_target.unwrap_or_default(),
//                         output_texture,
//                         0,
//                     );

//                     let _output = output.create_view(&wgpu::TextureViewDescriptor::default());

//                     Resources {
//                         input,
//                         framebuffer,
//                         _output,
//                     }
//                 })
//                 .collect();

//             Ok(Self {
//                 gl,
//                 resolution,
//                 vertex_shader,
//                 fragment_shader,
//                 program,
//                 texture_location,
//                 resource_swapchain,
//             })
//         }
//     }

//     pub fn get_input_texture(&self, swapchain_index: usize) -> gl::Texture {
//         self.resource_swapchain[swapchain_index].input
//     }

//     pub fn render(&self, swapchain_index: usize) {
//         unsafe {
//             let resources = &self.resource_swapchain[swapchain_index];

//             self.gl.use_program(Some(self.program));
//             self.gl
//                 .bind_framebuffer(gl::DRAW_FRAMEBUFFER, Some(resources.framebuffer));

//             self.gl.disable(gl::CULL_FACE);
//             self.gl.disable(gl::BLEND);
//             self.gl.disable(gl::SCISSOR_TEST);
//             self.gl.disable(gl::DEPTH_TEST);
//             self.gl
//                 .viewport(0, 0, self.resolution.x as i32, self.resolution.y as i32);

//             self.gl.active_texture(gl::TEXTURE0);
//             self.gl
//                 .bind_texture(GL_TEXTURE_EXTERNAL_OES, Some(resources.input));
//             self.gl.uniform_1_i32(Some(&self.texture_location), 0);

//             self.gl.draw_arrays(gl::TRIANGLE_STRIP, 0, 4);
//         }
//     }
// }

// impl Drop for PassthroughPass {
//     fn drop(&mut self) {
//         unsafe {
//             self.gl.delete_program(self.program);
//             self.gl.delete_shader(self.vertex_shader);
//             self.gl.delete_shader(self.fragment_shader);

//             for resources in self.resource_swapchain.drain(..) {
//                 // NB: destruction order
//                 self.gl.delete_texture(resources.input);
//                 self.gl.delete_framebuffer(resources.framebuffer);
//                 drop(resources._output);
//             }
//         };
//     }
// }
