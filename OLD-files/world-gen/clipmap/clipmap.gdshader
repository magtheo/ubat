shader_type spatial;

//global uniform sampler2D normalmap;
global uniform float amplitude;
uniform sampler2D heightmap;
uniform sampler2D normalmap;

varying vec2 texture_position; 

void vertex() {
	// Called for every vertex the material is visible on.
	vec3 world_vertex = VERTEX + MODEL_MATRIX[3].xyz;
	texture_position = ( world_vertex.xz* 0.5 )/float(textureSize(heightmap, 0).x);
	VERTEX.y = texture(heightmap, texture_position).r * amplitude;
}

void fragment() {
	// Called for every pixel the material is visible on.
	NORMAL_MAP = texture(normalmap, texture_position).rgb;
}

//void light() {
	// Called for every pixel for every light affecting the material.
	// Uncomment to replace the default light processing function with this one.
//}
