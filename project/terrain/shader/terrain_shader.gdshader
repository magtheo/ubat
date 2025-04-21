// res://project/terrain/shader/terrain_shader.gdshader
shader_type spatial;

//render_mode blend_mix, depth_draw_opaque, cull_back; // OLD - Problematic
render_mode depth_draw_always, cull_back; // NEW - Standard for opaque geometry

uniform sampler2DArray biome_textures : source_color, hint_default_white;
uniform vec2 uv_scale = vec2(1.0, 1.0);
uniform bool u_debug_mode = false;

varying vec4 vert_color; // COLOR input from mesh, includes biome ID in alpha
varying vec2 v_uv;       // UV passed from vertex shader
varying vec3 v_normal;   // Vertex normal passed from vertex shader

void vertex() {
	vert_color = COLOR;
	v_uv = UV * uv_scale;
	// Ensure the normal passed is suitable for the fragment shader
	// Depending on transformations, you might need:
	// v_normal = (INV_VIEW_MATRIX * vec4(NORMAL, 0.0)).xyz; // If NORMAL is view-space
	// Or often just passing NORMAL is fine if the engine handles space correctly.
	v_normal = NORMAL;
}

void fragment() {
	if (u_debug_mode) {
		// Use the debug color calculated in Rust
		ALBEDO = vert_color.rgb;
		// ALPHA = 1.0; // Not needed without blend_mix
	} else {
		// Normal rendering using biome ID from vertex alpha
		float biome_id_f = vert_color.a * 255.0;
		int layer_idx = int(round(biome_id_f));
		// TODO: Consider clamping layer_idx to valid range of texture array layers

		vec4 tex_color = texture(biome_textures, vec3(v_uv, float(layer_idx)));
		ALBEDO = tex_color.rgb;
		// ALPHA = tex_color.a; // Not needed without blend_mix unless base texture has alpha
	}

	// --- FIX ---
	// Assign the (normalized) vertex normal to the NORMAL built-in.
	// Do NOT assign to NORMAL_MAP unless you are sampling a normal map texture.
	NORMAL = normalize(v_normal); // Use the varying normal passed from vertex()

	// Set other PBR properties as needed
	ROUGHNESS = 0.8;
	METALLIC = 0.0;
}