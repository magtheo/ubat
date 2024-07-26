extends Node3D

@onready var mesh_instance = $mainMesh2

@export var mesh_instance1: MeshInstance3D
@export var areas: Array[Area3D]

# Dictionary to store the color associated with each area
var area_colors = {}

func _ready():
	# Assign colors to each area
	for area in areas:
		area_colors[area] = get_area_color(area)
		area.body_entered.connect(_on_area_body_entered.bind(area))
		area.body_exited.connect(_on_area_body_exited.bind(area))

func _on_area_body_entered(body, area):
	if body == mesh_instance:
		update_mesh_vertex_colors(area, true)

func _on_area_body_exited(body, area):
	if body == mesh_instance:
		update_mesh_vertex_colors(area, false)

func update_mesh_vertex_colors(area, entering):
	var color = area_colors[area]
	if not entering:
		color = Color(1, 1, 1)  # Default color when area exits, change as needed

	var mesh = mesh_instance.mesh
	if not mesh:
		return

	var surface_tool = SurfaceTool.new()
	surface_tool.begin(Mesh.PRIMITIVE_TRIANGLES)

	for surface in range(mesh.get_surface_count()):
		var array = mesh.surface_get_arrays(surface)
		var vertices = array[Mesh.ARRAY_VERTEX]
		var colors = array[Mesh.ARRAY_COLOR]

		if colors.empty():
			colors.resize(vertices.size())

		var mesh_transform = mesh_instance.global_transform

		for i in range(vertices.size()):
			var local_vertex = vertices[i]
			var global_vertex = mesh_transform.origin + mesh_transform.basis * local_vertex
			if area.shape.intersects_point(global_vertex):
				colors[i] = color

		array[Mesh.ARRAY_COLOR] = colors
		surface_tool.add_color_array(colors)
		surface_tool.add_vertex_array(vertices)
		surface_tool.index()
		surface_tool.commit(mesh_instance.mesh)

func get_area_color(area):
	# Define your own logic to get the color for the specific area
	# Example: Assign different colors based on area names or any other property
	match area.name:
		"Section1":
			print("COLOR: Section1 found")
			return Color(1, 0, 0) # RED
		"Section2":
			print("COLOR: Section2 found")
			return Color(0, 1, 0) # GREEN
		"Section3":
			print("COLOR: Section3 found")
			return Color(0, 0, 1) # BLUE
		_:
			return Color(1, 1, 1)  # Default white color for undefined areas
