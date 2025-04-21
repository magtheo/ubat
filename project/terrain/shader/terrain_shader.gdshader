// res://project/terrain/shader/terrain_shader.gdshader
shader_type spatial;

render_mode depth_draw_always, cull_back; // Standard opaque

uniform sampler2DArray biome_textures : source_color, hint_default_white;
uniform vec2 uv_scale = vec2(1.0, 1.0);
uniform bool u_debug_mode = false;

varying vec4 vert_color; // Still need for biome ID / debug color
varying vec2 v_uv;       // Still need for texture sampling
// varying vec3 v_normal; // REMOVED

void vertex() {
	vert_color = COLOR;   // Pass vertex color
	v_uv = UV * uv_scale; // Pass scaled UV
	// v_normal = NORMAL;    // REMOVED - Let engine handle normal transformation
}

void fragment() {
	if (u_debug_mode) {
		ALBEDO = vert_color.rgb;
	} else {
		// Normal rendering using biome ID from vertex alpha
		float biome_id_f = vert_color.a * 255.0;
		int layer_idx = int(round(biome_id_f));
		// TODO: Consider clamping layer_idx

		vec4 tex_color = texture(biome_textures, vec3(v_uv, float(layer_idx)));
		ALBEDO = tex_color.rgb;
	}

	// NORMAL = normalize(v_normal); // REMOVED - Let engine handle lighting normals

	// Set other PBR properties if needed
	ROUGHNESS = 0.8;
	METALLIC = 0.0;
}