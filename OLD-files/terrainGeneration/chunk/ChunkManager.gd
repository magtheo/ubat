extends Node3D

const CHUNK_SIZE = 32  
const RENDER_DISTANCE = 5  

const chunk_scene = preload("res://terrainGeneration/chunk/Chunk.tscn")
var active_chunks = {}
var chunk_pool = []

# Load C++ terrain generator
var terrain_generator = preload("res://terrain_generator.gdextension").new()

func add_chunk(coord: Vector3, player_position: Vector3):
	print("Loading chunk", coord)
	var chunk = get_chunk_from_pool()
	if not chunk:
		chunk = chunk_scene.instantiate()
		add_child(chunk)

	chunk.visible = true
	chunk.initialize(coord, CHUNK_SIZE)

	# Request terrain data from C++
	var terrain_data = terrain_generator.generate_chunk_data(coord.x, coord.z)
	var mesh = terrain_generator.generate_chunk_mesh(terrain_data)

	chunk.mesh = mesh
	active_chunks[coord] = chunk
