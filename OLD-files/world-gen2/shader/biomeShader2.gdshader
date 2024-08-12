shader_type spatial;

uniform float generalAmplitude;
uniform float coralAmplitude;
uniform float sandAmplitude;
uniform float rockAmplitude;
uniform float kelpAmplitude;
uniform float lavaStoneAmplitude;
uniform float biomeStrengthAmplifyer;
uniform float height_difference_amp;

uniform sampler2D biomeNoise;
uniform sampler2D heightChanger;

uniform sampler2D heightmapSand;
uniform sampler2D normalmapSand;
uniform sampler2D textureSand;

uniform sampler2D heightmapCoral;
uniform sampler2D normalmapCoral;
uniform sampler2D textureCoral;

uniform sampler2D heightmapRock;
uniform sampler2D normalmapRock;
uniform sampler2D textureRock;

uniform sampler2D heightmapKelp;
uniform sampler2D normalmapKelp;
uniform sampler2D textureKelp;

uniform sampler2D heightmapLavaStone;
uniform sampler2D normalmapLavaStone;
uniform sampler2D textureLavaStone;

varying vec2 texture_position;

uniform float coralCutoff;
uniform float sandCutoff;
uniform float rockCutoff;
uniform float kelpCutoff;
uniform float lavaStoneCutoff;

float smooth_step(float edge0, float edge1, float x) {
    float t = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

void vertex() {
    vec3 world_vertex = VERTEX + MODEL_MATRIX[3].xyz;
    texture_position = (world_vertex.xz * 0.5) / float(textureSize(biomeNoise, 0).x);

    float biomeStrength = texture(biomeNoise, texture_position).r * biomeStrengthAmplifyer;
    float height_amplifier = texture(heightChanger, texture_position).r;

    float sandWeight = smooth_step(sandCutoff - 0.1, sandCutoff + 0.1, biomeStrength);
    float coralWeight = smooth_step(coralCutoff - 0.1, coralCutoff + 0.1, biomeStrength);
    float rockWeight = smooth_step(rockCutoff - 0.1, rockCutoff + 0.1, biomeStrength);
    float kelpWeight = smooth_step(kelpCutoff - 0.1, kelpCutoff + 0.1, biomeStrength);
    float lavaStoneWeight = smooth_step(lavaStoneCutoff - 0.1, lavaStoneCutoff + 0.1, biomeStrength);

    float transitionWeight = 1.0 - sandWeight - coralWeight - rockWeight - kelpWeight - lavaStoneWeight;

    float sandHeight = texture(heightmapSand, texture_position).r * sandAmplitude;
    float coralHeight = texture(heightmapCoral, texture_position).r * coralAmplitude;
    float rockHeight = texture(heightmapRock, texture_position).r * rockAmplitude;
    float kelpHeight = texture(heightmapKelp, texture_position).r * kelpAmplitude;
    float lavaStoneHeight = texture(heightmapLavaStone, texture_position).r * lavaStoneAmplitude;

    VERTEX.y = (
        sandHeight * sandWeight +
        coralHeight * coralWeight +
        rockHeight * rockWeight +
        kelpHeight * kelpWeight +
        lavaStoneHeight * lavaStoneWeight +
        sandHeight * transitionWeight
    ) * generalAmplitude * (height_amplifier * height_difference_amp);
}

void fragment() {
    float biomeStrength = texture(biomeNoise, texture_position).r * biomeStrengthAmplifyer;

    float sandWeight = smooth_step(sandCutoff - 0.1, sandCutoff + 0.1, biomeStrength);
    float coralWeight = smooth_step(coralCutoff - 0.1, coralCutoff + 0.1, biomeStrength);
    float rockWeight = smooth_step(rockCutoff - 0.1, rockCutoff + 0.1, biomeStrength);
    float kelpWeight = smooth_step(kelpCutoff - 0.1, kelpCutoff + 0.1, biomeStrength);
    float lavaStoneWeight = smooth_step(lavaStoneCutoff - 0.1, lavaStoneCutoff + 0.1, biomeStrength);

    float transitionWeight = 1.0 - sandWeight - coralWeight - rockWeight - kelpWeight - lavaStoneWeight;

    vec3 sandNormal = texture(normalmapSand, texture_position).rgb * sandAmplitude;
    vec3 coralNormal = texture(normalmapCoral, texture_position).rgb * coralAmplitude;
    vec3 rockNormal = texture(normalmapRock, texture_position).rgb * rockAmplitude;
    vec3 kelpNormal = texture(normalmapKelp, texture_position).rgb * kelpAmplitude;
    vec3 lavaStoneNormal = texture(normalmapLavaStone, texture_position).rgb * lavaStoneAmplitude;

    NORMAL_MAP = normalize(
        sandNormal * sandWeight +
        coralNormal * coralWeight +
        rockNormal * rockWeight +
        kelpNormal * kelpWeight +
        lavaStoneNormal * lavaStoneWeight +
        sandNormal * transitionWeight
    );

    vec4 sandTexture = texture(textureSand, texture_position);
    vec4 coralTexture = texture(textureCoral, texture_position);
    vec4 rockTexture = texture(textureRock, texture_position);
    vec4 kelpTexture = texture(textureKelp, texture_position);
    vec4 lavaStoneTexture = texture(textureLavaStone, texture_position);

    vec4 blendedTexture = (
        sandTexture * sandWeight +
        coralTexture * coralWeight +
        rockTexture * rockWeight +
        kelpTexture * kelpWeight +
        lavaStoneTexture * lavaStoneWeight +
        sandTexture * transitionWeight
    );

    ALBEDO = blendedTexture.rgb;
}