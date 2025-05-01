// Shader for terrain with 3-way texture blending based on vertex attributes
shader_type spatial;
// Standard render modes
render_mode depth_draw_always, cull_back;

// Uniforms
uniform sampler2DArray biome_textures : source_color, filter_linear_mipmap, repeat_enable;
uniform vec2 uv_scale = vec2(0.05, 0.05);
uniform bool u_debug_mode = false; // Add this uniform for switching debug mode

// Varyings to pass data to fragment shader
varying vec3 v_biome_ids;
varying vec3 v_biome_weights;
varying vec2 v_uv;

void vertex() {
    // Pass UV, scaling it
    v_uv = UV * uv_scale;
    
    // Access custom attributes directly
    v_biome_ids = vec3(float(CUSTOM0.r), float(CUSTOM0.g), float(CUSTOM0.b));
    v_biome_weights = CUSTOM1.rgb;
}

void fragment() {
    // If in debug mode, show colors directly and skip texture sampling
    if (u_debug_mode) {
        ALBEDO = COLOR.rgb; // Use vertex color for debug
        ROUGHNESS = 0.8;
        METALLIC = 0.0;
    } else {
        // Extract IDs
        int id1 = max(0, int(v_biome_ids.x));
        int id2 = max(0, int(v_biome_ids.y));
        int id3 = max(0, int(v_biome_ids.z));
        
        // Extract weights
        float w1 = v_biome_weights.x;
        float w2 = v_biome_weights.y;
        float w3 = v_biome_weights.z;
        
        // Normalize weights
        float total_weight = max(0.0001, w1 + w2 + w3);
        w1 /= total_weight;
        w2 /= total_weight;
        w3 /= total_weight;
        
        // Output debugging info for gray mesh
        if (id1 <= 0 && id2 <= 0 && id3 <= 0) {
            // All IDs are zero or negative, indicating possible data issue
            ALBEDO = vec3(0.7, 0.0, 0.7); // Bright purple to indicate problem
        } else {
            // Sample textures
            vec4 tex_color1 = texture(biome_textures, vec3(v_uv, float(id1)));
            vec4 tex_color2 = texture(biome_textures, vec3(v_uv, float(id2)));
            vec4 tex_color3 = texture(biome_textures, vec3(v_uv, float(id3)));
            
            // Blend Albedo
            ALBEDO = tex_color1.rgb * w1 + tex_color2.rgb * w2 + tex_color3.rgb * w3;
        }
        ROUGHNESS = 0.8;
        METALLIC = 0.0;
    }
}