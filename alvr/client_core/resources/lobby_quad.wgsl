struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
}

@group(0) @binding(0) var hud_texture: texture_2d<f32>;
@group(0) @binding(1) var hud_sampler: sampler;

struct PushConstant {
    transform: mat4x4f,
    object_type: u32,
    floor_side: f32,
}
var<push_constant> pc: PushConstant;

@vertex
fn vertex_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var result: VertexOutput;

    let norm_vert_a = f32(vertex_index & 1);
    let norm_vert_b = f32(vertex_index >> 1);

    result.uv = vec2f(norm_vert_a, norm_vert_b);
    result.position = pc.transform * vec4f(result.uv.x - 0.5, 0.5 - result.uv.y, 0.0, 1.0);

    return result;
}

@fragment
fn fragment_main(@location(0) uv: vec2f) -> @location(0) vec4f {
    if pc.object_type == 0 { // Ground
        let world_xz = (uv - 0.5) * pc.floor_side;

        let ground_center = vec3f(0.0, 0.0, 0.0);
        let ground_horizon = vec3f(0.0, 0.0, 0.015);

        let grid_close = vec3f(0.114, 0.545, 0.804);
        let grid_far = vec3f(0.259, 0.863, 0.886);

        let line_fade_start = 10.0;
        let line_fade_end = 50.0;
        let line_fade_dist = line_fade_end - line_fade_start;

        let line_bloom = 10.0;

        let distance = length(world_xz);

        // Pick a coordinate to visualize in a grid
        let cell_size = 2.0;
        let coord = world_xz / cell_size;

        // Compute anti-aliased world-space grid lines
        let screen_space_line_width = 1.0 * fwidth(coord); // todo: make resolution agnostic?
        let grid = abs(fract(coord - 0.5) - 0.5) / screen_space_line_width;

        // Create mask for grid lines and fade over distance
        var line = clamp(1.0 - min(grid.x, grid.y), 0.0, 1.0);
        line *= clamp((line_fade_start - distance) / line_fade_dist, 0.0, 1.0);
    
        // Fill in normal ground colour
        var out_color = ground_center * (1.0 - line);

        // Add cheap and simple "bloom" to the grid lines
        line *= 1.0 + line_bloom;

        // Fill in grid line colour
        out_color += line * mix(grid_far, grid_close, clamp((line_fade_end - distance) / line_fade_end, 0.0, 1.0));

        // Fade to the horizon colour over distance
        if distance > 10.0 {
            let coef = 1.0 - 10.0 / distance;
            out_color = (1.0 - coef) * out_color + coef * ground_horizon;
        }

        return vec4f(out_color, 1.0);
    } else { // HUD
        return textureSample(hud_texture, hud_sampler, uv);
    }
}
