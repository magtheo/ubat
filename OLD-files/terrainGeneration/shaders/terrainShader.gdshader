shader_type spatial;

uniform sampler2D heightmap;
uniform sampler2D coral_texture;
uniform sampler2D sand_texture;
uniform sampler2D rock_texture;
uniform sampler2D kelp_texture;
uniform sampler2D lavarock_texture;

// Noise for each biome
uniform sampler2D coral_noise;
uniform sampler2D sand_noise;
uniform sampler2D rock_noise;
uniform sampler2D kelp_noise;
uniform sampler2D volcanic_noise;

uniform int lod_level;

varying vec3 world_pos;

void vertex() {
    world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
    float detail_factor = pow(2.0, float(lod_level));
    VERTEX.y = texture(heightmap, UV).r * (50.0 / detail_factor);
}

void fragment() {
    // Sample noise for each biome
    vec2 noise_uv = world_pos.xz * 0.1;
    float coral_n = texture(coral_noise, noise_uv).r;
    float sand_n = texture(sand_noise, noise_uv).r;
    float rock_n = texture(rock_noise, noise_uv).r;
    float kelp_n = texture(kelp_noise, noise_uv).r;
    float volcanic_n = texture(volcanic_noise, noise_uv).r;

    // Sample textures
    vec2 tex_uv = world_pos.xz * 0.1;
    vec3 coral = texture(coral_texture, tex_uv).rgb;
    vec3 sand = texture(sand_texture, tex_uv).rgb;
    vec3 rock = texture(rock_texture, tex_uv).rgb;
    vec3 kelp = texture(kelp_texture, tex_uv).rgb;
    vec3 lavarock = texture(lavarock_texture, tex_uv).rgb;

    // Get height for blending
    float height = texture(heightmap, UV).r;

    // Blend textures based on height and noise
    vec3 color = mix(sand, coral, smoothstep(0.0, 0.3, height + coral_n * 0.2));
    color = mix(color, rock, smoothstep(0.3, 0.6, height + rock_n * 0.2));
    color = mix(color, kelp, smoothstep(0.4, 0.7, height + kelp_n * 0.2) * (1.0 - smoothstep(0.6, 0.9, height)));
    color = mix(color, lavarock, smoothstep(0.7, 1.0, height + volcanic_n * 0.2));

    ALBEDO = color;
}