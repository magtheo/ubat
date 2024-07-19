shader_type spatial;
uniform float amplitude;

// Uniform sampler2D variables for texture maps: one is for height values (hinted as albedo) and another for normal vectors.
uniform sampler2D biomeNoise;
uniform sampler2D heightmapSand;
uniform sampler2D normalmapSand;

uniform sampler2D heightmapCorral;
uniform sampler2D normalmapCorral;

// Varying variable to hold the UV coordinates of each vertex in texture space.
varying vec2 texture_position;

uniform float corralCutof;
uniform float sandCutof;

// Hermite interpolation function for smooth transitions
float smooth_step(float edge0, float edge1, float x) {
    float t = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

void vertex() {
    // Calculate the world position of the current vertex by adding it with model matrix's translation part.
    vec3 world_vertex = VERTEX + MODEL_MATRIX[3].xyz;

    // Adjust texture coordinates to range [0, 1]
    texture_position = (world_vertex.xz * 0.5) / float(textureSize(biomeNoise, 0).x);

    // Sample biome noise
    float biomeStrength = texture(biomeNoise, texture_position).r;

    // Determine the height based on biome strength with smooth transitions
    float sandHeight = texture(heightmapSand, texture_position).r;
    float corralHeight = texture(heightmapCorral, texture_position).r;
    
    if (biomeStrength >= corralCutof) { // Corral
        VERTEX.y = corralHeight * amplitude;
    } else if (biomeStrength > sandCutof && biomeStrength < corralCutof) { // Transition zone
        float blend = smooth_step(sandCutof, corralCutof, biomeStrength);
        VERTEX.y = mix(sandHeight, corralHeight, blend) * amplitude;
    } else if (biomeStrength >= sandCutof) { // Sand
        VERTEX.y = sandHeight * amplitude;
    }
}

void fragment() {
    // Sample biome noise
    float biomeStrength = texture(biomeNoise, texture_position).r;

    // Determine the normal map based on biome strength with smooth transitions
    vec3 sandNormal = texture(normalmapSand, texture_position).rgb;
    vec3 corralNormal = texture(normalmapCorral, texture_position).rgb;
    
    if (biomeStrength >= corralCutof) { // Corral
        NORMAL_MAP = corralNormal;
    } else if (biomeStrength > sandCutof && biomeStrength < corralCutof) { // Transition zone
        float blend = smooth_step(sandCutof, corralCutof, biomeStrength);
        NORMAL_MAP = mix(sandNormal, corralNormal, blend);
    } else if (biomeStrength >= sandCutof) { // Sand
        NORMAL_MAP = sandNormal;
    }
}
