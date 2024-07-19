// This shader type is used for materials that affect geometry in 3D space.
shader_type spatial;

// Uniform variable to control the amplitude of the height scaling factor. It's a hint that this value should range between 0.1 and 8.0.
uniform float amplitude;  

uniform sampler2D biomeNoise;


// Uniform sampler2D variables for texture maps: one is for height values (hinted as albedo) and another for normal vectors.
uniform sampler2D heightmapSand;  
uniform sampler2D normalmapSand; 

uniform sampler2D heightmapCorral;
uniform sampler2D normalmapCorral;

// Varying variable to hold the UV coordinates of each vertex in texture space.
varying vec2 texture_position; 

void vertex() {
    // This function is called for every vertex the material is visible on.
    
    // Calculate the world position of the current vertex by adding it with model matrix's translation part.
    vec3 world_vertex = VERTEX + MODEL_MATRIX[3].xyz; 
    
    // Convert the vertex coordinates to texture space and normalize them, so they range from -1 to 1 in x and z directions.
    // Then multiply by 0.5 for a better visibility of height map on the surface.
    texture_position = (world_vertex.xz * 0.5) / float(textureSize(biomeNoise, 0).x);  
    
	vec4 color = texture(biomeNoise, texture_position);
    if (color == vec4(1.0)) { // white pixels will be mapped to heightmap2
        VERTEX.y = texture(heightmapSand, texture_position).r * amplitude; 
    } else {                 // black pixels will be mapped to heightmap1
        VERTEX.y = texture(heightmapCorral, texture_position).r * amplitude; 
    }
}

void fragment() {
    // This function is called for every pixel on the surface of the material.
    vec4 color = texture(biomeNoise, texture_position);
    if (color == vec4(1.0)) { // white pixels will be mapped to heightmap2
		NORMAL_MAP = texture(normalmapSand, texture_position).rgb; 
    } else {                    // black pixels will be mapped to heightmap1
		NORMAL_MAP = texture(normalmapCorral, texture_position).rgb; 
    }
    // Sample the normalmap texture at the current UV coordinates and use its RGB values as the normal vector of the vertex.
}