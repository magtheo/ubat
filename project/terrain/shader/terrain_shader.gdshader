// res://materials/biome_terrain.gdshader
shader_type spatial;
// render_mode world_vertex_coords; // Optional: useful for triplanar mapping later
render_mode blend_mix, depth_draw_opaque, cull_back; // Match render mode from previous code

// Uniform for the texture array resource you created
uniform sampler2DArray biome_textures : source_color, hint_default_white;
// Optional: Add other uniforms for tiling, normal maps, etc.
uniform vec2 uv_scale = vec2(1.0, 1.0);

// --- ADD DEBUG UNIFORM ---
// This will be set to true/false by the Rust ChunkController code
uniform bool u_debug_mode = false;

// Receive vertex color from the vertex shader
varying vec4 vert_color;
// Pass UV and Normal too
varying vec2 v_uv;
varying vec3 v_normal;


void vertex() {
    // Pass the vertex color straight through to the fragment shader
    vert_color = COLOR;

    // Apply UV scaling if needed
    v_uv = UV * uv_scale; // Store scaled UV in varying

    // Pass normal
    v_normal = NORMAL;
}

void fragment() {
    // Default to magenta in case something goes wrong
    ALBEDO = vec3(1.0, 0.0, 1.0);
    ALPHA = 1.0; // Default to opaque

    if (u_debug_mode) {
        // --- Debug Mode ---
        // Output the vertex color directly.
        // The Rust code already calculated the debug color based on the
        // selected mode (Height, Biome ID, etc.)
        ALBEDO = vert_color.rgb;
        // Optional: Use alpha from debug color if it has meaning, otherwise force opaque
        ALPHA = 1.0; // Or set to 1.0 if debug colors should always be opaque

    } else {
        // --- Normal Rendering Mode ---
        // Decode the biome ID from the alpha channel passed in vert_color
        // (Assuming the Rust code set vert_color.a correctly in normal mode)
        float biome_id_f = vert_color.a * 255.0;
        // Round to nearest integer to get the layer index and avoid precision errors
        int layer_idx = int(round(biome_id_f));

        // Ensure layer_idx is within valid range if possible (requires texture array size uniform)
        // Example: uniform int u_texture_layers = 6;
        // layer_idx = clamp(layer_idx, 0, u_texture_layers - 1);

        // Sample the correct layer of the texture array using the scaled UV
        vec4 tex_color = texture(biome_textures, vec3(v_uv, float(layer_idx)));

        // Apply texture color to ALBEDO
        ALBEDO = tex_color.rgb;
        ALPHA = tex_color.a; // Use texture's alpha

        // Basic lighting properties (adjust as needed)
        ROUGHNESS = 0.8;
        METALLIC = 0.1;
    }

    // Set the normal map in both modes (using the varying)
    // If you have actual normal maps later, you'll need to sample them here too.
    NORMAL_MAP = v_normal;
    // Or if not using normal maps and just relying on vertex normals:
    // NORMAL = v_normal; // (Don't use both NORMAL and NORMAL_MAP)

}