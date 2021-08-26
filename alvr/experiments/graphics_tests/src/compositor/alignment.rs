pub fn align_to_32(size: (u32, u32)) -> (u32, u32) {
    (
        (size.0 as f32 / 32_f32).ceil() as u32 * 32,
        (size.1 as f32 / 32_f32).ceil() as u32 * 32,
    )
}

// Some encoders require the size to be a multiple of 32. To avoid color bleeding, the empty space
// is filled with the nearest pixel color
pub struct AlignmentPass {}

impl AlignmentPass {
    pub fn new(input_size: (u32, u32), output_size: (u32, u32)) -> Self {
        todo!()
    }
}
