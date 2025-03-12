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
uniform float blend_min = 0.4;  // Below this value, use biome 1 only
uniform float blend_max = 0.6;  // Above this value, use biome 2 only

// Debug mode selector (0-5)
// 0 = normal rendering
// 1 = show biome blend map RGB
// 2 = show blend factor (alpha channel)
// 3 = show section identification
// 4 = show biome weights
// 5 = show only kelp for debugging
uniform int debug_mode = 0;

void vertex() {
	// No height displacement needed in vertex shader since we're already building
	// the mesh with height displacement in C++
}

void fragment() {
	// Sample the biome blend map
	vec4 biome_blend = texture(biome_blend_map, UV);
	
	// Debug visualizations
	if (debug_mode == 1) {
		// Show the biome blend map RGB values (section identification)
		ALBEDO = biome_blend.rgb;
		ROUGHNESS = 0.7;
		METALLIC = 0.0;
	} else if (debug_mode == 2) {
		// Show the blend factor (alpha channel)
		float blend_factor = biome_blend.a;
		ALBEDO = vec3(blend_factor, blend_factor, blend_factor);
		ROUGHNESS = 0.7;
		METALLIC = 0.0;
	} else if (debug_mode == 3) {
		// Visualize section identification
		vec3 section_color;
		
		// Section 1: Corral + Sand (show as yellow)
		if (length(biome_blend.rgb - vec3(0.8, 0.8, 0.8)) < 0.2) {
			section_color = vec3(1.0, 1.0, 0.0);
		}
		// Section 2: Rock + Kelp (show as green)
		else if (biome_blend.r > 0.7) {
			section_color = vec3(0.0, 1.0, 0.0);
		}
		// Section 3: Rock + Lavarock (show as red)
		else {
			section_color = vec3(1.0, 0.0, 0.0);
		}
		
		ALBEDO = section_color;
		ROUGHNESS = 0.7;
		METALLIC = 0.0;
	} else if (debug_mode == 4) {
		// Visualize blend weight
		float blend_factor = biome_blend.a;
		float weight;
		if (blend_factor < blend_min) {
			weight = 0.0;
		} else if (blend_factor > blend_max) {
			weight = 1.0;
		} else {
			weight = (blend_factor - blend_min) / (blend_max - blend_min);
		}
		
		// Red = 100% biome 1, Green = 100% biome 2, Yellow = blended
		ALBEDO = vec3(1.0 - weight, weight, 0.0);
		ROUGHNESS = 0.7;
		METALLIC = 0.0;
	} else if (debug_mode == 5) {
		// Only show kelp for debugging
		ALBEDO = texture(kelp_texture, UV).rgb;
		ROUGHNESS = 0.5;
		METALLIC = 0.0;
	} else {
		// Normal rendering with biome blending
		// Sample all textures at the current UV
		vec4 corral_color = texture(corral_texture, UV);
		vec4 sand_color = texture(sand_texture, UV);
		vec4 rock_color = texture(rock_texture, UV);
		vec4 kelp_color = texture(kelp_texture, UV);
		vec4 lavarock_color = texture(lavarock_texture, UV);

		// Initialize final color
		vec4 final_color = vec4(0.0);

		// Get blend factor from alpha channel
		float blend_factor = biome_blend.a;
		
		// Apply the step blending logic
		float weight;
		if (blend_factor < blend_min) {
			// Below blend_min: 100% biome 1
			weight = 0.0;
		} else if (blend_factor > blend_max) {
			// Above blend_max: 100% biome 2
			weight = 1.0;
		} else {
			// In between: smooth blend (normalized to 0-1 range)
			weight = (blend_factor - blend_min) / (blend_max - blend_min);
		}

		// SECTION 1: Corral + Sand
		if (length(biome_blend.rgb - vec3(0.8, 0.8, 0.8)) < 0.2) {
			// Apply the blend between corral and sand
			final_color = mix(corral_color, sand_color, weight);
			
			// Adjust roughness and metallic properties based on blend
			ROUGHNESS = mix(0.8, 0.3, weight); // Corral rougher, sand smoother
			METALLIC = 0.0;
		}
		// SECTION 2: Rock + Kelp
		else if (biome_blend.r > 0.7) {
			// Apply the blend between rock and kelp
			final_color = mix(rock_color, kelp_color, weight);
			
			// Adjust roughness and metallic properties
			ROUGHNESS = mix(0.9, 0.5, weight); // Rock rougher, kelp smoother
			METALLIC = 0.0;
		}
		// SECTION 3: Rock + Lavarock
		else {
			// Apply the blend between rock and lavarock
			final_color = mix(rock_color, lavarock_color, weight);
			
			// Adjust roughness and metallic properties
			ROUGHNESS = mix(0.9, 0.7, weight);
			METALLIC = mix(0.0, 0.3, weight); // Lavarock slightly metallic
		}

		// Apply final color to the material
		ALBEDO = final_color.rgb;
	}
}