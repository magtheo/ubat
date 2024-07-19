extends Node3D

var section1_shader = load("res://world-gen/secondWorld/sectionOne/Shader/Section1biomeShader3.gdshader")
var section2_shader = load("res://world-gen/secondWorld/sectionOne/Shader/Section2biomeShader.gdshader")
var section3_shader = load("res://world-gen/secondWorld/sectionOne/Shader/Section3.gdshader")

@onready var mesh = $mainMesh

func _physics_process(delta):
	if not mesh:
		return
		
	for body in mesh.get_overlapping_bodies():
		var collision_shape = body.get_collision_shape()
		if collision_shape and collision_shape.has_method("get_biome"):  # Assuming each biome shape has a function to get its biome
			var shader: ShaderMaterial
			
			match collision_shape.call("get_biome"):  # Get the biome from the collision shape
				"section1":
					shader = section1_shader
				"section2":
					shader = section2_shader
				"section3":
					shader = section3_shader
			mesh.surface_set_material(0, shader)  # Replace with the correct surface index if needed
