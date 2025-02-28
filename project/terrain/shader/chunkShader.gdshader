shader_type spatial;

// Biome Textures
uniform sampler2D corral_texture;
uniform sampler2D sand_texture;
uniform sampler2D rock_texture;
uniform sampler2D kelp_texture;
uniform sampler2D lavaRock_texture;

// Biome Blend Map (passed from C++)
uniform sampler2D biome_blend_map; // Stores biome weights in R, G, B, A

// Heightmap (passed from C++)
uniform sampler2D height_map;
uniform float height_scale = 10.0;

void vertex() {
    vec2 uv = UV;
    
    // Sample height from heightmap (R channel)
    float height = texture(height_map, uv).r * height_scale;
    VERTEX.y += height; // Apply height displacement
}

void fragment() {
    vec2 uv = UV;

    // Sample biome blend map
    vec4 biome_weights = texture(biome_blend_map, uv);

    // Section 1: Corral & Sand (Red Channel)
    vec4 corral_color = texture(corral_texture, uv);
    vec4 sand_color = texture(sand_texture, uv);
    vec4 section1 = mix(corral_color, sand_color, biome_weights.r);

    // Section 2: Rock & Kelp (Green Channel)
    vec4 rock_color = texture(rock_texture, uv);
    vec4 kelp_color = texture(kelp_texture, uv);
    vec4 section2 = mix(rock_color, kelp_color, biome_weights.g);

    // Section 3: Rock & Lavarock (Blue Channel)
    vec4 lavaRock_color = texture(lavaRock_texture, uv);
    vec4 section3 = mix(rock_color, lavaRock_color, biome_weights.b);

    // Boss Area (White channel - Optional)
    vec4 boss_area = vec4(1.0, 0.0, 0.0, 1.0); // Red visual cue for debug
    vec4 final_color = mix(mix(section1, section2, biome_weights.g), section3, biome_weights.b);

    // If Boss Area is detected, override
    if (biome_weights.a > 0.8) {
        final_color = boss_area;
    }

    ALBEDO = final_color.rgb;
}
