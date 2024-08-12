# chunk.gd
extends MeshInstance3D

var coord: Vector2
var size: int
var biome_manager: BiomeManager
var heightmap: Image
var sections: Dictionary

func initialize(chunk_coord: Vector2, chunk_size: int, bm: BiomeManager):
	coord = chunk_coord
	size = chunk_size
	biome_manager = bm
	position = Vector3(coord.x * size, 0, coord.y * size)

func generate_terrain():
	heightmap = Image.create(size, size, false, Image.FORMAT_RF)
	sections = biome_manager.get_sections_for_chunk(coord, size)

	for x in range(size):
		for z in range(size):
			var world_x = coord.x * size + x
			var world_z = coord.y * size + z
			var height = calculate_height(world_x, world_z)
			heightmap.set_pixel(x, z, Color(height, 0, 0))
	
	create_mesh()

func calculate_height(x: float, z: float) -> float:
	var total_height = 0.0
	var total_weight = 0.0

	for section in sections:
		var section_weight = section.get_weight(x, z)
		var section_height = section.get_height(x, z)
		total_height += section_height * section_weight
		total_weight += section_weight
	
	return total_height / total_weight if total_weight > 0 else 0.0

func create_mesh():
	var surface_tool = SurfaceTool.new()
	surface_tool.begin(Mesh.PRIMITIVE_TRIANGLES)

	for x in range(size - 1):
		for z in range(size - 1):
			var v1 = Vector3(x, heightmap.get_pixel(x, z).r, z)
			var v2 = Vector3(x + 1, heightmap.get_pixel(x + 1, z).r, z)
			var v3 = Vector3(x, heightmap.get_pixel(x, z + 1).r, z + 1)
			var v4 = Vector3(x + 1, heightmap.get_pixel(x + 1, z + 1).r, z + 1)

			surface_tool.add_vertex(v1)
			surface_tool.add_vertex(v2)
			surface_tool.add_vertex(v3)

			surface_tool.add_vertex(v3)
			surface_tool.add_vertex(v2)
			surface_tool.add_vertex(v4)

	surface_tool.generate_normals()
	surface_tool.generate_tangents()

	mesh = surface_tool.commit()
	create_trimesh_collision()
	
	# Apply the terrain shader
	var material = ShaderMaterial.new()
	material.shader = load("res://terrain_shader.gdshader")
	material.set_shader_parameter("heightmap", heightmap)
	set_surface_override_material(0, material)
