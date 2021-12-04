#version 450

layout(push_constant) uniform PushConstants {
    ivec2 input_size;    // unaligned input slice size
    int input_columns;   // number of slices per row
    ivec2 combined_size; // size of the image if slices are combined
    ivec2 output_offset; // top left corner of the output slice in the combined image
    ivec2 target_size;   // aligned output slice size == render target size
}
pc;

// slices aranged row major
layout(set = 0, binding = 0, rgba8) uniform readonly image2DArray input_slices;

layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 color;
void main() {
    ivec2 coord = ivec2(uv * vec2(pc.target_size));

    ivec2 combined_coord = pc.output_offset + coord;

    // Note: for the right-most and bottom-most slices, sampling might be done out of bounds. Since
    // no sampler is used to define the address mode, the behavior is undefined. Random GPU memory
    // might be sampled, which might be noisy and require more bandwidth when encoded at the same
    // quality.
    // This line fixes the problem:
    ivec2 clamped_coord = min(combined_coord, pc.combined_size); // todo: check if needed

    ivec2 slice_coord = clamped_coord / pc.input_size;
    int slice_index = slice_coord.y * pc.input_columns + slice_coord.x;

    ivec2 input_coord = clamped_coord % pc.input_size;

    color = imageLoad(input_slices, ivec3(input_coord, slice_index));
}