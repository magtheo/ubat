shader_type spatial;

// Input textures from C++ code
uniform sampler2D biome_blend_map : source_color;
uniform sampler2D height_map : hint_default_white;

// Biome textures - add your own textures here
uniform sampler2D corral_texture : source_color, filter_linear_mipmap, repeat_enable;
uniform sampler2D sand_texture : source_color, filter_linear_mipmap, repeat_enable;
uniform sampler2D rock_texture : source_color, filter_linear_mipmap, repeat_enable;
uniform sampler2D kelp_texture : source_color, filter_linear_mipmap, repeat_enable;
uniform sampler2D lavarock_texture : source_color, filter_linear_mipmap, repeat_enable;

// Scaling for the textures
uniform float texture_scale = 0.1;

// Height parameters
uniform float height_scale = 1.0;
uniform float height_offset = 0.0;

// Blend parameters
uniform float blend_sharpness = 5.0;

// Debug mode - set to true if textures aren't loaded
uniform bool debug_mode = false;

void vertex() {
	// No height displacement needed in vertex shader since we're already building
	// the mesh with height displacement in C++

	// Pass UV to fragment shader
	UV = UV * texture_scale;
}

void fragment() {
	// Sample the biome blend map
	vec4 biome_color = texture(biome_blend_map, UV / texture_scale);

	// DEBUG MODE - just show the biome blend map colors
	if (debug_mode) {
		ALBEDO = biome_color.rgb;
		ROUGHNESS = 0.7;
		METALLIC = 0.0;
		return;
	}

	// Sample all textures at the current UV
	vec4 corral_color = texture(corral_texture, UV);
	vec4 sand_color = texture(sand_texture, UV);
	vec4 rock_color = texture(rock_texture, UV);
	vec4 kelp_color = texture(kelp_texture, UV);
	vec4 lavarock_color = texture(lavarock_texture, UV);

	// Initialize final color
	vec4 final_color = vec4(0.0);

	// Color matching for biomes using the red, green, blue channels
	// These colors should match your BiomeManager's color definitions
	float corral_weight = smoothstep(0.0, 1.0/blend_sharpness, 1.0 - distance(biome_color.rgb, vec3(0.9, 0.6, 0.6)));
	float sand_weight = smoothstep(0.0, 1.0/blend_sharpness, 1.0 - distance(biome_color.rgb, vec3(0.9, 0.9, 0.6)));
	float rock_weight = smoothstep(0.0, 1.0/blend_sharpness, 1.0 - distance(biome_color.rgb, vec3(0.6, 0.6, 0.6)));
	float kelp_weight = smoothstep(0.0, 1.0/blend_sharpness, 1.0 - distance(biome_color.rgb, vec3(0.3, 0.7, 0.3)));
	float lavarock_weight = smoothstep(0.0, 1.0/blend_sharpness, 1.0 - distance(biome_color.rgb, vec3(0.7, 0.3, 0.3)));

	// Calculate sum of weights
	float total_weight = corral_weight + sand_weight + rock_weight + kelp_weight + lavarock_weight;

	// If no biome matches (to avoid division by zero), use a fallback color
	if (total_weight < 0.001) {
		final_color = vec4(0.5, 0.5, 0.5, 1.0);
	} else {
		// Normalize weights
		corral_weight /= total_weight;
		sand_weight /= total_weight;
		rock_weight /= total_weight;
		kelp_weight /= total_weight;
		lavarock_weight /= total_weight;

		// Blend textures based on weights
		final_color =
			corral_color * corral_weight +
			sand_color * sand_weight +
			rock_color * rock_weight +
			kelp_color * kelp_weight +
			lavarock_color * lavarock_weight;
	}

	// Apply final color to the material
	ALBEDO = final_color.rgb;

	// Adjust roughness based on biome
	// Rock and lava rock are less smooth than sand and kelp
	ROUGHNESS = mix(0.9, 0.3, sand_weight + kelp_weight);

	// Adjust metallic property - lava rock can be slightly metallic
	METALLIC = lavarock_weight * 0.3;
}