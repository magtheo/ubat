shader_type spatial;

uniform vec4 color_ground : source_color = vec4(0.3, 0.7, 0.3, 1.0);
uniform vec4 color_water : source_color = vec4(0.1, 0.3, 0.6, 0.8);
uniform vec4 color_mountain : source_color = vec4(0.6, 0.5, 0.4, 1.0);
uniform float height_scale = 1.0;

varying float height;

void vertex() {
    // Store the vertex height for use in the fragment shader
    height = VERTEX.y * height_scale;
}

void fragment() {
    // Basic color based on height
    vec4 base_color;
    
    if (height < 0.0) {
        // Water color for below sea level
        base_color = color_water;
    } else if (height > 5.0) {
        // Transition to mountain color for high elevations
        float t = smoothstep(5.0, 10.0, height);
        base_color = mix(color_ground, color_mountain, t);
    } else {
        // Ground color
        base_color = color_ground;
    }
    
    // Add noise for texture
    float noise = fract(sin(dot(FRAGCOORD.xy, vec2(12.9898, 78.233))) * 43758.5453);
    base_color.rgb += (noise * 0.1 - 0.05); // Subtle noise variation
    
    // Lighting
    float ndotl = dot(NORMAL, normalize(vec3(1.0, 2.0, 1.0)));
    float light_intensity = max(0.2, ndotl);
    
    // Apply final color
    ALBEDO = base_color.rgb * light_intensity;
    ROUGHNESS = 0.9;
    SPECULAR = 0.1;
}