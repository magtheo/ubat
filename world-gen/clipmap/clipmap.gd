extends Node3D

@onready var section_1 = $"Section 1"
@onready var section_2 = $"Section 2"
@onready var section_3 = $"Section 3"

@onready var meshInstance = $mainMesh
@onready var mesh = meshInstance.mesh

func _physics_process(delta):
	if not mesh:
		return
		
	for body in mesh.get_overlapping_bodies():
		var collision_shape = body.get_collision_shape()
		if collision_shape:  # Assuming each biome shape has a function to get its biome
			var coordinates = []  # Initialize an array to store the coordinates
			
			# Get the vertices of the collision shape and calculate their world positions
			for vertex in collision_shape.get_vertices():
				var world_vertex = mesh.to_local(vertex)  # Convert local vertex to world space
				var x = world_vertex.x
				var y = world_vertex.y
				var z = world_vertex.z
				coordinates.append([x, y, z])  # Store the coordinates in the array
			
			# Now you can use these coordinates in your shader to determine which noise to use for terrain generation
			# For example:
			#var noise_index = get_noise_index(coordinates)  # Replace with your own implementation
			# ...
