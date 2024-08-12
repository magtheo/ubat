shader_type spatial;

uniform sampler2D heightmap;
uniform sampler2D coral_texture;
uniform sampler2D sand_texture;
uniform sampler2D rock_texture;
uniform sampler2D kelp_texture;
uniform sampler2D lavarock_texture;

varying vec3 world_pos;

void vertex() {
    world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
    VERTEX.y = texture(heightmap, UV).r * 50.0;  // Adjust the multiplier to control overall terrain height
}

void fragment() {
    // Sample textures
    vec3 coral = texture(coral_texture, world_pos.xz * 0.1).rgb;
    vec3 sand = texture(sand_texture, world_pos.xz * 0.1).rgb;
    vec3 rock = texture(rock_texture, world_pos.xz * 0.1).rgb;
    vec3 kelp = texture(kelp_texture, world_pos.xz * 0.1).rgb;
    vec3 lavarock = texture(lavarock_texture, world_pos.xz * 0.1).rgb;
    
    // Implement biome blending based on height and position
    float height = texture(heightmap, UV).r;
    
    // Example blending (you'll need to adjust this based on your specific requirements)
    vec3 color = mix(sand, coral, smoothstep(0.0, 0.3, height));
    color = mix(color, rock, smoothstep(0.3, 0.6, height));
    color = mix(color, kelp, smoothstep(0.4, 0.7, height) * (1.0 - smoothstep(0.6, 0.9, height)));
    color = mix(color, lavarock, smoothstep(0.7, 1.0, height));
    
    ALBEDO = color;
}
