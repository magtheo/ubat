// Shader for terrain with 3-way texture blending based on vertex attributes
shader_type spatial;

render_mode depth_draw_always, cull_back; // Standard opaque rendering

// Uniforms
uniform sampler2DArray biome_textures : source_color, filter_linear_mipmap, repeat_enable;
uniform vec2 uv_scale = vec2(0.05, 0.05); // Example scale, adjust as needed

// Vertex Attributes (Make sure these match what you set in mesh generation!)
// CUSTOM0: Stores biome IDs as floats (x=ID1, y=ID2, z=ID3)
// CUSTOM1: Stores biome weights (x=W1, y=W2, z=W3) -> Must sum to 1.0!
varying vec3 custom0; // Biome IDs (passed as vec3 from mesh)
varying vec3 custom1; // Biome Weights (passed as vec3 from mesh)

// Varyings to pass data to fragment shader
varying vec3 v_biome_ids;
varying vec3 v_biome_weights;
varying vec2 v_uv;

void vertex() {
    // Pass UV, scaling it
    v_uv = UV * uv_scale;

    // Pass biome IDs and weights directly
    v_biome_ids = custom0;
    v_biome_weights = custom1;

    // Godot handles POSITION and NORMAL transformation automatically
}

void fragment() {
    // Extract IDs (convert float to int)
    // Add small offset before int() to counteract potential float precision issues if needed
    // e.g., int(v_biome_ids.x + 0.01)
    int id1 = int(v_biome_ids.x);
    int id2 = int(v_biome_ids.y);
    int id3 = int(v_biome_ids.z);

    // Extract weights (already sum to 1.0 from Rust code)
    float w1 = v_biome_weights.x;
    float w2 = v_biome_weights.y;
    float w3 = v_biome_weights.z; // Or: max(0.0, 1.0 - w1 - w2); if only sending 2

    // Sample textures using IDs as array layers
    vec4 tex_color1 = texture(biome_textures, vec3(v_uv, float(id1)));
    vec4 tex_color2 = texture(biome_textures, vec3(v_uv, float(id2)));
    vec4 tex_color3 = texture(biome_textures, vec3(v_uv, float(id3)));

    // Blend Albedo colors using weights
    // Ensure weights used are non-negative
    w1 = max(0.0, w1);
    w2 = max(0.0, w2);
    w3 = max(0.0, w3);
    // Optional: Re-normalize weights here just in case sums aren't exactly 1 due to float issues
    // float total_w = w1 + w2 + w3; if (total_w > 0.001) { w1/=total_w; w2/=total_w; w3/=total_w;}

    vec3 blended_albedo = tex_color1.rgb * w1 + tex_color2.rgb * w2 + tex_color3.rgb * w3;

    ALBEDO = blended_albedo;

    // --- Optional: Blend other PBR properties (Roughness, Metallic, Normals) ---
    // Example: Blend roughness based on texture alpha or separate maps
    // float rough1 = texture(biome_roughness_maps, vec3(v_uv, float(id1))).r;
    // float rough2 = texture(biome_roughness_maps, vec3(v_uv, float(id2))).r;
    // float rough3 = texture(biome_roughness_maps, vec3(v_uv, float(id3))).r;
    // ROUGHNESS = rough1 * w1 + rough2 * w2 + rough3 * w3;
    ROUGHNESS = 0.8; // Default for now

    // Example: Blend normals (more complex, requires tangent space calculation)
    // You'd sample normal maps and blend them using techniques like Reoriented Normal Mapping (RNM)
    // or simple linear blend (less accurate). For now, rely on mesh normals.

    METALLIC = 0.0; // Default for now
}