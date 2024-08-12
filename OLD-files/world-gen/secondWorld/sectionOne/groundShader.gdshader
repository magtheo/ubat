shader_type spatial;

uniform sampler2D primaryNoise;  // Noise map for primary biome regions
uniform sampler2D biomeNoise1;   // Noise map for blending biomes in region 1
uniform sampler2D biomeNoise2;   // Noise map for blending biomes in region 2
uniform sampler2D biomeNoise3;   // Noise map for blending biomes in region 3

// Biome texture maps
uniform sampler2D biome1TextureA;
uniform sampler2D biome1NormalA;
uniform sampler2D biome1TextureB;
uniform sampler2D biome1NormalB;

uniform sampler2D biome2TextureA;
uniform sampler2D biome2NormalA;
uniform sampler2D biome2TextureB;
uniform sampler2D biome2NormalB;

uniform sampler2D biome3TextureA;
uniform sampler2D biome3NormalA;
uniform sampler2D biome3TextureB;
uniform sampler2D biome3NormalB;

uniform vec2 uv_offset;

varying vec2 texture_position;

float smooth_step(float edge0, float edge1, float x) {
    float t = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

void vertex() {
    vec3 world_vertex = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
    texture_position = (world_vertex.xz * 0.5 + uv_offset) / float(textureSize(primaryNoise, 0).x);
}

void fragment() {
    vec2 uv = texture_position;

    // Sample the primary noise map to determine the biome region
    float primaryValue = texture(primaryNoise, uv).r;

    // Define cutoff points for the three regions
    float region1 = smooth_step(0.0, 0.33, primaryValue);
    float region2 = smooth_step(0.33, 0.66, primaryValue);
    float region3 = smooth_step(0.66, 1.0, primaryValue);

    vec4 finalColor;
    vec3 finalNormal;

    // Blend biomes within region 1
    if (region1 > 0.5) {
        float blendFactor = texture(biomeNoise1, uv).r;
        vec4 biome1A = texture(biome1TextureA, uv);
        vec4 biome1B = texture(biome1TextureB, uv);
        vec3 normal1A = texture(biome1NormalA, uv).rgb * 2.0 - 1.0;
        vec3 normal1B = texture(biome1NormalB, uv).rgb * 2.0 - 1.0;

        finalColor = mix(biome1A, biome1B, blendFactor);
        finalNormal = normalize(mix(normal1A, normal1B, blendFactor));
    }
    // Blend biomes within region 2
    else if (region2 > 0.5) {
        float blendFactor = texture(biomeNoise2, uv).r;
        vec4 biome2A = texture(biome2TextureA, uv);
        vec4 biome2B = texture(biome2TextureB, uv);
        vec3 normal2A = texture(biome2NormalA, uv).rgb * 2.0 - 1.0;
        vec3 normal2B = texture(biome2NormalB, uv).rgb * 2.0 - 1.0;

        finalColor = mix(biome2A, biome2B, blendFactor);
        finalNormal = normalize(mix(normal2A, normal2B, blendFactor));
    }
    // Blend biomes within region 3
    else if (region3 > 0.5) {
        float blendFactor = texture(biomeNoise3, uv).r;
        vec4 biome3A = texture(biome3TextureA, uv);
        vec4 biome3B = texture(biome3TextureB, uv);
        vec3 normal3A = texture(biome3NormalA, uv).rgb * 2.0 - 1.0;
        vec3 normal3B = texture(biome3NormalB, uv).rgb * 2.0 - 1.0;

        finalColor = mix(biome3A, biome3B, blendFactor);
        finalNormal = normalize(mix(normal3A, normal3B, blendFactor));
    }

    ALBEDO = finalColor.rgb;
    NORMAL = finalNormal;
}
