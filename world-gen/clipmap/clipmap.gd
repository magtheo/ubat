extends Node3D

@onready var mesh_instance = $mainMesh
@onready var shader_material = mesh_instance.get_active_material(0) # error line
@export var areas: Array[Area3D]


# Function to get mesh vertices
func get_mesh_vertices(mesh_instance: MeshInstance3D) -> Array:
	var vertices = []
	var mesh = mesh_instance.mesh
	if mesh:
		var arrays = mesh.surface_get_arrays(0)
		var vertex_array = arrays[Mesh.ARRAY_VERTEX]
		for vertex in vertex_array:
			vertices.append(vertex)
	return vertices

# Function to check if a point is inside an Area3D
func is_point_in_area(point: Vector3, area: Area3D) -> bool:
	var collision_shape = area.shape_owner_get_shape(0, 0)

	if collision_shape is BoxShape3D:
		var local_point = point - area.global_transform.origin
		if abs(local_point.x) <= collision_shape.extents.x and abs(local_point.y) <= collision_shape.extents.y and abs(local_point.z) <= collision_shape.extents.z:
			return true
		else:
			return false
	else:
		print("section box error")
		return false

	
# Function to find colliding vertices
func find_colliding_vertices():
	var vertices = get_mesh_vertices(mesh_instance)
	var result = {}
	
	for area in areas:
		result[area] = []
		for vertex in vertices:
			var global_vertex = vertex - mesh_instance.global_transform.origin
			if is_point_in_area(global_vertex, area):
				result[area].append(global_vertex)

	return result	


func _ready():
	print("Mesh instance: ", mesh_instance)
	if mesh_instance:
		print("Mesh instance valid.")
	else:
		print("Mesh instance is null.")
	if shader_material:
		print("shader_material valid:", shader_material)
	else:
		print("shader_material not valid:", shader_material)
	
	# Get colliding vertices
	var colliding_vertices = find_colliding_vertices()
	
	# Prepare data for shader
	var vertex_positions = []
	var vertex_areas = []
	var area_id = 1
	
	for area in areas:
		if area in colliding_vertices:
			for vertex in colliding_vertices[area]:
				vertex_positions.append(vertex)
				vertex_areas.append(area_id)
		area_id += 1
	
	# Send data to shader
	shader_material.set_shader_parameter("vertex_positions", vertex_positions)
	shader_material.set_shader_parameter("areas", areas)
