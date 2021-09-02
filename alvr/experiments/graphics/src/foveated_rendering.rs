use alvr_common::Fov;
use alvr_session::FoveatedRenderingDesc;
use wgpu::{CommandEncoder, TextureView};

pub enum FrDirection {
    Encoding,
    Decoding,
}

// Implements Axis-Aligned Distorted Transfer algorithm
pub struct FoveatedRenderingPass {}

impl FoveatedRenderingPass {
    // There is no way of selecting the best output size. The returned size is calculated using the
    // reference_fov which might not be what is actually used.
    // todo: reparametrize FoveatedRenderingDesc with focus area width and height in degrees
    pub fn new(
        direction: FrDirection,
        original_size: (u32, u32),
        desc: &FoveatedRenderingDesc,
        reference_fov: Fov, // initial fov used to choose the encoded frame size
    ) -> (Self, (u32, u32)) {
        todo!()
    }

    pub fn input(&self) -> &TextureView {
        todo!()
    }

    // note: depending on the eye tracking implemetation, moving the focus area could be achieved
    // just by changing the fov argument
    pub fn draw(&self, encoder: &mut CommandEncoder, fov: Fov) {
        todo!()
    }
}
