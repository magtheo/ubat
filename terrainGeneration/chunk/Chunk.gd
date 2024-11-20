# chunk.gd
extends MeshInstance3D

var coord: Vector3
var size: int
var heightmap: Image
var sections: Dictionary
var biome_manager
var noise_textures: Dictionary

var lod_level = 0
var mesh_detail_levels = {}
const MAX_LOD_LEVELS = 4

var terrain_shader: Shader
var biome_textures: Dictionary

# Cache for height calculations
var height_cache = {}
const CACHE_SIZE = 1000

func initialize(chunk_coord: Vector3, chunk_size: int, biome_mgr, shader: Shader, biome_tex: Dictionary, noise_tex: Dictionary):
	coord = chunk_coord
	size = chunk_size
	biome_manager = biome_mgr
	terrain_shader = shader
	biome_textures = biome_tex
	noise_textures = noise_tex
	position = Vector3(coord.x * size, 0, coord.z * size)
	
	# Initialize heightmap with specific format for better performance
	heightmap = Image.create(size, size, false, Image.FORMAT_RF)
	
	# Pre-allocate mesh detail levels
	for i in range(MAX_LOD_LEVELS):
		mesh_detail_levels[i] = null

func generate_terrain():
	# Generate heightmap using biome data
	generate_heightmap()
	
	# Generate LOD levels asynchronously if possible
	call_deferred("generate_lod_levels")

func generate_heightmap():
	sections = biome_manager.get_sections_for_chunk(coord, size)
	
	# Create temporary arrays for faster access
	var height_data = []
	height_data.resize(size * size)
	
	# Calculate all heights in a single pass
	for z in range(size):
		for x in range(size):
			var world_x = coord.x * size + x
			var world_z = coord.z * size + z
			var height = calculate_height(world_x, world_z)
			var index = z * size + x
			height_data[index] = height
			heightmap.set_pixel(x, z, Color(height, 0, 0))

func calculate_height(x: float, z: float) -> float:
	# Check cache first
	var cache_key = str(Vector3(x, 0, z))
	if height_cache.has(cache_key):
		return height_cache[cache_key]
	
	var total_height = 0.0
	var total_weight = 0.0
	
	# Calculate combined height from all biomes
	for biome_type in biome_manager.biomes:
		var biome_data = biome_manager.biomes[biome_type]
		
		 # Scale the coordinates to control the size of terrain features
		var scaled_x = x / biome_manager.BIOME_SCALE
		var scaled_z = z / biome_manager.BIOME_SCALE
		
		var noise_val = biome_data.noise.get_noise_2d(scaled_x, scaled_z)
		var weight = biome_data.weight_noise.get_noise_2d(scaled_x, scaled_z)
		
		# Normalize noise values
		noise_val = (noise_val + 1.0) * 0.5
		weight = (weight + 1.0) * 0.5
		
		# Apply biome-specific height multiplier
		var height = noise_val * biome_data.height_multiplier
		
		total_height += height * weight
		total_weight += weight
	
	var final_height = total_height / max(total_weight, 0.001)
	
	# Cache management
	if height_cache.size() > CACHE_SIZE:
		height_cache.clear() # Clear cache if too large
	height_cache[cache_key] = final_height
	
	return final_height

func generate_lod_levels():
	# Generate each LOD level
	for lod in range(MAX_LOD_LEVELS):
		if mesh_detail_levels[lod] == null:
			generate_lod_mesh(lod)
	
	# Set initial LOD level
	set_lod(0)

func generate_lod_mesh(lod: int):
	var surface_tool = SurfaceTool.new()
	surface_tool.begin(Mesh.PRIMITIVE_TRIANGLES)
	
	var vertex_skip = pow(2, lod)
	var uv_scale = 1.0 / float(size)
	
	# Pre-calculate vertices for better performance
	var vertices = []
	var uvs = []
	vertices.resize((size / vertex_skip) * (size / vertex_skip) * 6)
	uvs.resize((size / vertex_skip) * (size / vertex_skip) * 6)
	
	var vertex_index = 0
	
	for x in range(0, size - vertex_skip, vertex_skip):
		for z in range(0, size - vertex_skip, vertex_skip):
			# Calculate vertices
			var v1 = Vector3(x, heightmap.get_pixel(x, z).r, z)
			var v2 = Vector3(x + vertex_skip, heightmap.get_pixel(min(x + vertex_skip, size - 1), z).r, z)
			var v3 = Vector3(x, heightmap.get_pixel(x, min(z + vertex_skip, size - 1)).r, z + vertex_skip)
			var v4 = Vector3(x + vertex_skip, heightmap.get_pixel(min(x + vertex_skip, size - 1), min(z + vertex_skip, size - 1)).r, z + vertex_skip)
			
			# Calculate UVs
			var uv1 = Vector2(x * uv_scale, z * uv_scale)
			var uv2 = Vector2((x + vertex_skip) * uv_scale, z * uv_scale)
			var uv3 = Vector2(x * uv_scale, (z + vertex_skip) * uv_scale)
			var uv4 = Vector2((x + vertex_skip) * uv_scale, (z + vertex_skip) * uv_scale)
			
			# First triangle
			vertices[vertex_index] = v1
			vertices[vertex_index + 1] = v2
			vertices[vertex_index + 2] = v3
			uvs[vertex_index] = uv1
			uvs[vertex_index + 1] = uv2
			uvs[vertex_index + 2] = uv3
			
			# Second triangle
			vertices[vertex_index + 3] = v3
			vertices[vertex_index + 4] = v2
			vertices[vertex_index + 5] = v4
			uvs[vertex_index + 3] = uv3
			uvs[vertex_index + 4] = uv2
			uvs[vertex_index + 5] = uv4
			
			vertex_index += 6
	
	# Add all vertices and UVs at once
	for i in range(vertex_index):
		surface_tool.set_uv(uvs[i])
		surface_tool.add_vertex(vertices[i])
	
	surface_tool.generate_normals()
	surface_tool.generate_tangents()
	
	mesh_detail_levels[lod] = surface_tool.commit()
	
	if lod == 0:
		setup_collision_and_material()

func setup_collision_and_material():
	create_trimesh_collision()
	
	var material = ShaderMaterial.new()
	material.shader = terrain_shader
	
	# Set shader parameters
	material.set_shader_parameter("heightmap", heightmap)
	material.set_shader_parameter("lod_level", lod_level)
	
	# Set biome textures
	for biome_name in biome_textures:
		material.set_shader_parameter(biome_name + "_texture", biome_textures[biome_name])
	
	# Set noise textures
	for noise_name in noise_textures:
		material.set_shader_parameter(noise_name + "_noise", noise_textures[noise_name])
	
	set_surface_override_material(0, material)

func set_lod(level: int):
	lod_level = clamp(level, 0, MAX_LOD_LEVELS - 1)
	
	if mesh_detail_levels[lod_level] == null:
		generate_lod_mesh(lod_level)
	
	mesh = mesh_detail_levels[lod_level]
	
	var material = get_surface_override_material(0)
	if material:
		material.set_shader_parameter("lod_level", lod_level)
