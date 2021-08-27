[[block]]
struct PushConstants {
    input_size: vec2<i32>;    // unaligned input slice size
    input_columns: i32;       // number of slices per row
    combined_size: vec2<i32>; // size of the image if slices are combined
    output_offset: vec2<i32>; // top left corner of the output slice in the combined image
    target_size: vec2<i32>;   // aligned output slice size == render target size
};
var<push_constant> pc: PushConstants;

[[group(0), binding(0)]]
var input_slices: texture_2d_array<f32>; // slices aranged row major

[[stage(fragment)]]
fn main([[location(0)]] uv: vec2<f32>) -> [[location(0)]] vec4<f32> {
    let coord = vec2<i32>(uv * vec2<f32>(pc.target_size));

    let combined_coord = pc.output_offset + coord;

    // Note: for the right-most and bottom-most slices, sampling might be done out of bounds. Since
    // no sampler is used to define the address mode, the behavior is undefined. Random GPU memory
    // might be sampled, which might be noisy and require more bandwidth when encoded at the same
    // quality. In theory this should be fixed once the final webgpu spec is released and wgpu
    // becomes perfectly compliant.
    // This line fixes the problem:
    let clamped_coord = min(combined_coord, pc.combined_size);

    let slice_coord = clamped_coord / pc.input_size;
    let slice_index = slice_coord.y * pc.input_columns + slice_coord.x;

    let input_coord = clamped_coord % pc.input_size;
    
    return textureLoad(input_slices, input_coord, slice_index, 0);
}
