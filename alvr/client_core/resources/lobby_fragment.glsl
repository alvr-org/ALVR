#version 300 es

uniform lowp int object_type;
uniform sampler2D hud_texture;

in vec2 uv;
in vec3 position;

out vec4 out_color;

void main() {
    if(object_type == 0) { // Ground
        lowp vec3 groundCenter = vec3(0.0f, 0.0f, 0.0f);
        lowp vec3 groundHorizon = vec3(0.0f, 0.0f, 0.015f);

        lowp vec3 gridClose = vec3(0.114f, 0.545f, 0.804f);
        lowp vec3 gridFar = vec3(0.259f, 0.863f, 0.886f);

        lowp float lineFadeStart = 10.0f;
        lowp float lineFadeEnd = 50.0f;
        lowp float lineFadeDist = lineFadeEnd - lineFadeStart;

        lowp float lineBloom = 10.0f;

        lowp float distance = length(position.xz);

        // Pick a coordinate to visualize in a grid
        lowp vec2 coord = position.xz / 2.0f;

        // Compute anti-aliased world-space grid lines
        lowp vec2 grid = abs(fract(coord - 0.5f) - 0.5f) / fwidth(coord);

        // Create mask for grid lines and fade over distance
        lowp float line = clamp(1.0f - min(grid.x, grid.y), 0.0f, 1.0f);
        line *= clamp((lineFadeStart - distance) / lineFadeDist, 0.0f, 1.0f);

        // Fill in normal ground colour
        out_color.rgb = groundCenter * (1.0f - line);

        // Add cheap and simple "bloom" to the grid lines
        line *= 1.0f + lineBloom;

        // Fill in grid line colour
        out_color.rgb += line * mix(gridFar, gridClose, clamp((lineFadeEnd - distance) / lineFadeEnd, 0.0f, 1.0f));

        // Fade to the horizon colour over distance
        if(distance > 10.0f) {
            lowp float coef = 1.0f - 10.0f / distance;
            out_color.rgb = (1.0f - coef) * out_color.rgb + coef * groundHorizon;
        }

        out_color.a = 1.0f;
    } else if(object_type == 1) { // HUD
        lowp vec3 textColor = vec3(1.0f, 1.0f, 1.0f);

        out_color.rgb = textColor;
        out_color.a = texture(hud_texture, uv).a;
    } else if(object_type == 2) { // Hands
        out_color = vec4(1.0f, 1.0f, 1.0f, 1.0f);
    }
}