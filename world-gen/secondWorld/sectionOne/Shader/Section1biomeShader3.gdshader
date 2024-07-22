shader_type spatial;

uniform float geraralAmplitude;
uniform float corralAmplitude;
uniform float sandAmplitude;
uniform float biomeStrengthAmplifyer;
uniform float height_difference_amp;

// Uniform sampler2D variables for texture maps: one is for height values (hinted as albedo) and another for normal vectors.
uniform sampler2D biomeNoise;
uniform sampler2D heightChanger;

uniform sampler2D heightmapSand;
uniform sampler2D normalmapSand;
uniform sampler2D textureSand;

uniform sampler2D heightmapCorral;
uniform sampler2D normalmapCorral;
uniform sampler2D textureCorral;

uniform vec3 vertex_positions[1000]; // verteces pr area, Adjust size as needed
uniform int areas[3]; // Adjust size as needed

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
    float biomeStrength = texture(biomeNoise, texture_position).r * biomeStrengthAmplifyer;
	float height_amplefier = texture(heightChanger, texture_position).r;

    // Calculate weights for blending
    float sandWeight = smooth_step(sandCutof - 0.1, sandCutof + 0.1, biomeStrength);
    float corralWeight = smooth_step(corralCutof - 0.1, corralCutof + 0.1, biomeStrength);
    float transitionWeight = 1.0 - sandWeight - corralWeight;

    // Sample heights
    float sandHeight = texture(heightmapSand, texture_position).r * sandAmplitude;
    float corralHeight = texture(heightmapCorral, texture_position).r * corralAmplitude;
	
    // Blend heights
    VERTEX.y = (sandHeight * sandWeight + corralHeight * corralWeight + sandHeight * transitionWeight) * geraralAmplitude * (height_amplefier * height_difference_amp);
}

void fragment() {
    // Sample biome noise
    float biomeStrength = texture(biomeNoise, texture_position).r;

    // Calculate weights for blending
    float sandWeight = smooth_step(sandCutof - 0.1, sandCutof + 0.1, biomeStrength);
    float corralWeight = smooth_step(corralCutof - 0.1, corralCutof + 0.1, biomeStrength);
    float transitionWeight = 1.0 - sandWeight - corralWeight;

    // Sample normals
    vec3 sandNormal = texture(normalmapSand, texture_position).rgb * sandAmplitude;
    vec3 corralNormal = texture(normalmapCorral, texture_position).rgb * corralAmplitude;

    // Blend normals
    NORMAL_MAP = normalize(sandNormal * sandWeight + corralNormal * corralWeight + sandNormal * transitionWeight);

    // Sample textures
    vec4 sandTexture = texture(textureSand, texture_position);
    vec4 corralTexture = texture(textureCorral, texture_position);

    // Blend textures
    vec4 blendedTexture = sandTexture * sandWeight + corralTexture * corralWeight + sandTexture * transitionWeight;
    
    // Set the final texture color
    ALBEDO = blendedTexture.rgb;

    // Output biome data (for example: red channel for sand, green for corral)
    vec3 biomeColor = vec3(sandWeight, corralWeight, 0.0);
    //ALBEDO = biomeColor;
}
