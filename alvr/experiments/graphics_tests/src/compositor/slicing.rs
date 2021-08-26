use wgpu::Texture;

fn get_slice_size(original_size: (u32, u32), slice_count: usize) -> (u32, u32) {
    todo!()
}

// Merge k slices then split the result into n slices
// Slices are assumed to be packed and unpacked by this same pipeline, following a particular layout
// determined by the number of slices and the shape of the reconstructed frame.
pub struct SlicingPass {
    input: Vec<Texture>,
}

impl SlicingPass {
    pub fn new(
        original_size: (u32, u32),
        input_slices_count: usize,
        output_slices_count: usize,
    ) -> (Self, (u32, u32)) {
        todo!()
    }

    pub fn input(&self) -> &[Texture] {
        &self.input
    }
}
