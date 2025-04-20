// res://materials/biome_terrain.gdshader
shader_type spatial;
// render_mode world_vertex_coords; // Optional: useful for triplanar mapping later

// Uniform for the texture array resource you created
uniform sampler2DArray biome_textures : source_color, hint_default_white;
// Optional: Add other uniforms for tiling, normal maps, etc.
uniform vec2 uv_scale = vec2(1.0, 1.0);

// Receive vertex color from the vertex shader
varying vec4 vert_color;

void vertex() {
	// Pass the vertex color straight through to the fragment shader
	vert_color = COLOR;

	// Apply UV scaling if needed
	UV = UV * uv_scale;
}

void fragment() {
	// Decode the biome ID from the alpha channel
	// Multiply by 255.0 because we normalized it to 0.0-1.0 earlier
	float biome_id_f = vert_color.a * 255.0;
	// Round to nearest integer to get the layer index and avoid precision errors
	int layer_idx = int(round(biome_id_f));

	// Sample the correct layer of the texture array
	vec4 tex_color = texture(biome_textures, vec3(UV, float(layer_idx)));

	// Apply texture color to ALBEDO
	ALBEDO = tex_color.rgb;

	// Basic lighting (optional, Godot does standard lighting by default)
	// ROUGHNESS = 0.8;
	// METALLIC = 0.0;

    // --- DEBUGGING: Visualize Biome ID directly ---
    // Uncomment one of these lines to see the raw biome ID as color,
    // useful for verifying the ID is being passed correctly.
    // ALBEDO = vert_color.rgb * vert_color.a; // Tint white by normalized ID
    // ALBEDO = vec3(biome_id_f / 5.0); // Visualize ID directly (adjust divisor based on max biome ID)
    // ------------------------------------------
}