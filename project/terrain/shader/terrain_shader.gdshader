// res://project/terrain/shader/terrain_material.tres (or .glsl)
shader_type spatial;
render_mode depth_draw_always, cull_back;

uniform sampler2DArray biome_textures : source_color, filter_linear_mipmap, repeat_enable;
// uniform sampler2DArray biome_detail : source_color, filter_linear_mipmap, repeat_enable; // Optional
uniform vec2 uv_scale = vec2(0.05, 0.05);
uniform bool u_debug_mode = false;

uniform float texture_blend_sharpness = 1.0; // Default to 1.0, can make sharper/softer
uniform float shader_noise_influence = 0.1; // Reduced, as primary blend is data-driven
// uniform float detail_influence = 0.3; // If using detail textures
// uniform float edge_darkness = 0.2;

varying vec3 v_biome_ids;       // Raw IDs [0-255]
varying vec3 v_biome_weights;   // Smoothed, normalized weights for top 3 biomes
varying vec2 v_uv;
varying vec3 v_world_pos; // World position for noise sampling

// Simple noise function (can be more complex if needed)
float random(vec2 st) {
    return fract(sin(dot(st.xy, vec2(12.9898,78.233))) * 43758.5453123);
}

void vertex() {
    v_world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
    
    // UVs are passed directly now, randomization can be done in mesh gen if desired
    v_uv = UV * uv_scale; 
    
    // Biome IDs (CUSTOM0.rgb are already 0-1 from u8, multiply by 255 to get original ID)
    v_biome_ids = CUSTOM0.rgb * 255.0; 
    
    // Biome weights (CUSTOM1.rgb are already 0-1, normalized, and smoothed)
    v_biome_weights = CUSTOM1.rgb; 
}

void fragment() {
    if (u_debug_mode) {
        ALBEDO = COLOR.rgb; // COLOR is set in ChunkController for debug modes
        ROUGHNESS = 0.8;
        METALLIC = 0.0;
    } else {
        float id1 = floor(v_biome_ids.r + 0.5); // Ensure integer ID
        float id2 = floor(v_biome_ids.g + 0.5);
        float id3 = floor(v_biome_ids.b + 0.5);
        
        // v_biome_weights are already the final blend weights for the top 3
        // They should sum to 1.0.
        float w1 = v_biome_weights.x;
        float w2 = v_biome_weights.y;
        float w3 = v_biome_weights.z;

        // Optional: Apply shader-side noise for fine-grained texture break-up at transitions
        // This noise should be subtle as the main blending is data-driven.
        if (shader_noise_influence > 0.0) {
            float noise = (random(v_world_pos.xz * 0.1) - 0.5) * 2.0 * shader_noise_influence; // Noise between -influence and +influence
            // Distribute noise influence carefully to maintain sum ~1.0
            // This is a simple distribution, can be made more sophisticated
            float total_orig_w = w1 + w2 + w3; // Should be ~1.0
            if (total_orig_w > 0.01) { // Avoid division by zero
                 float W1_contrib = w1 / total_orig_w;
                 float W2_contrib = w2 / total_orig_w;
                 // float W3_contrib = w3 / total_orig_w; // Not needed for 2-way modulation

                 w1 = clamp(w1 + noise * W2_contrib, 0.0, 1.0); // If w1 gains, w2 loses proportionally
                 w2 = clamp(w2 - noise * W1_contrib, 0.0, 1.0);
                 // w3 could also be modulated, e.g., w3 = clamp(w3 - noise * (some_factor), 0.0, 1.0);
                 
                 // Re-normalize after shader noise perturbation
                 float current_sum = max(0.001, w1 + w2 + w3);
                 w1 /= current_sum;
                 w2 /= current_sum;
                 w3 /= current_sum;
            }
        }

        // Apply texture_blend_sharpness using pow()
        // This makes transitions sharper (exponent > 1) or softer (exponent < 1)
        vec3 final_blend_weights = vec3(
            pow(w1, texture_blend_sharpness),
            pow(w2, texture_blend_sharpness),
            pow(w3, texture_blend_sharpness)
        );
        
        // Re-normalize after sharpness adjustment
        float total_sharp_weight = max(0.001, final_blend_weights.x + final_blend_weights.y + final_blend_weights.z);
        final_blend_weights /= total_sharp_weight;
        
        vec3 blended_color = vec3(0.0);
        if (final_blend_weights.x > 0.001 && id1 > 0.0) { // Check ID > 0 to avoid sampling layer 0 if it's a placeholder
            blended_color += texture(biome_textures, vec3(v_uv, id1)).rgb * final_blend_weights.x;
        }
        if (final_blend_weights.y > 0.001 && id2 > 0.0) {
            blended_color += texture(biome_textures, vec3(v_uv, id2)).rgb * final_blend_weights.y;
        }
        if (final_blend_weights.z > 0.001 && id3 > 0.0) {
            blended_color += texture(biome_textures, vec3(v_uv, id3)).rgb * final_blend_weights.z;
        }
        
        // Fallback if all weights became zero (e.g. if all ids were 0)
        if (total_sharp_weight < 0.001) {
             if (id1 > 0.0) { // Try to use the first ID if valid
                 blended_color = texture(biome_textures, vec3(v_uv, id1)).rgb;
             } else {
                 blended_color = vec3(0.5, 0.2, 0.8); // Purple error color
             }
        }

        ALBEDO = blended_color;
        
        // Slope-based effects can remain similar
        float slope = 1.0 - abs(dot(NORMAL, vec3(0.0, 1.0, 0.0)));
        ROUGHNESS = mix(0.7, 0.9, slope); 
        METALLIC = 0.0;
        // ALBEDO = mix(ALBEDO, ALBEDO * (1.0 - edge_darkness), slope); // Optional edge darkening
    }
}