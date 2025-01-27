# ChunkManager.gd
extends Node3D

const CHUNK_SIZE = 32  # Adjust as needed
const RENDER_DISTANCE = 5  # Number of chunks in each direction
const POOL_SIZE = 121  # (2 * RENDER_DISTANCE + 1)²

var THREAD_COUNT = max(2, OS.get_processor_count() - 2)

const chunk_scene = preload("res://terrainGeneration/chunk/Chunk.tscn")
var active_chunks = {}
var chunk_pool = []
var thread_pool = []
var biome_manager
var terrain_shader: Shader
var biome_textures: Dictionary
var noise_textures: Dictionary

# Update constructor to include noise_textures
func _init(biome_mgr, shader: Shader, textures: Dictionary, noise_tex: Dictionary):
	biome_manager = biome_mgr
	terrain_shader = shader
	biome_textures = textures
	noise_textures = noise_tex

func _ready():
	# Initialize thread pool - matches CPU cores
	for i in range(THREAD_COUNT):
		var thread = Thread.new()
		thread_pool.append(thread)
	
	# Initialize chunk pool - matches maximum possible visible chunks
	for i in range(POOL_SIZE):
		var chunk = chunk_scene.instantiate()
		chunk.visible = false
		chunk_pool.append(chunk)
		add_child(chunk)

func update_chunks(player_position: Vector3):
	var chunk_coords = world_to_chunk_coordinates(player_position)
	
	# Update LOD levels for existing chunks
	for coord in active_chunks.keys():
		var chunk = active_chunks[coord]
		var chunk_center = Vector3(coord.x * CHUNK_SIZE + CHUNK_SIZE/2, 0, coord.z * CHUNK_SIZE + CHUNK_SIZE/2)
		var distance = (chunk_center - player_position).length()
		chunk.set_lod(calculate_lod_level(distance))

	# Determine which chunks should be active
	var should_be_active = {}
	for x in range(chunk_coords.x - RENDER_DISTANCE, chunk_coords.x + RENDER_DISTANCE + 1):
		for z in range(chunk_coords.z - RENDER_DISTANCE, chunk_coords.z + RENDER_DISTANCE + 1):
			should_be_active[Vector3(x,0, z)] = true

	# Remove chunks that are no longer needed
	var coords_to_remove = []
	for coord in active_chunks:
		if not should_be_active.has(coord):
			coords_to_remove.append(coord)
	
	for coord in coords_to_remove:
		remove_chunk(coord)

	# Add new chunks
	for coord in should_be_active:
		if not active_chunks.has(coord):
			add_chunk(coord, player_position)

func get_available_thread() -> Thread:
	for thread in thread_pool:
		if not thread.is_alive():
			return thread
	return null

func get_chunk_from_pool() -> MeshInstance3D:
	for chunk in chunk_pool:
		if not chunk.is_visible():
			return chunk
	return null

func add_chunk(coord: Vector3, player_position: Vector3):
	print("Loading chunk", coord)
	var chunk = get_chunk_from_pool()
	if not chunk:
		chunk = chunk_scene.instantiate()
		add_child(chunk)
	
	chunk.visible = true
	# Pass all required parameters including noise_textures
	chunk.initialize(coord, CHUNK_SIZE, biome_manager, terrain_shader, biome_textures, noise_textures)
	active_chunks[coord] = chunk
	
	# Calculate initial LOD level based on distance
	var chunk_center = Vector3(coord.x * CHUNK_SIZE + CHUNK_SIZE/2, 0, coord.z * CHUNK_SIZE + CHUNK_SIZE/2)
	var distance = (chunk_center - player_position).length()
	chunk.set_lod(calculate_lod_level(distance))
	
	var available_thread = get_available_thread()
	if available_thread:
		available_thread.start(Callable(chunk, "generate_terrain"))

func remove_chunk(coord: Vector3):
	print("Removing chunk", coord)
	var chunk = active_chunks[coord]
	chunk.visible = false
	active_chunks.erase(coord)
	chunk_pool.append(chunk)

func calculate_lod_level(distance: float) -> int:
	var LOD_DISTANCES = [100.0, 200.0, 300.0, 400.0]
	for i in range(LOD_DISTANCES.size()):
		if distance < LOD_DISTANCES[i]:
			return i
	return LOD_DISTANCES.size() - 1

func world_to_chunk_coordinates(position: Vector3) -> Vector3:
	return Vector3(floor(position.x / CHUNK_SIZE), 0, floor(position.z / CHUNK_SIZE))

func cleanup():
	print("cleaning chunks")
	# Clean up threads
	for thread in thread_pool:
		if thread.is_alive():
			thread.wait_to_finish()
	
	# Clean up chunks
	for chunk in active_chunks.values():
		chunk.queue_free()
	for chunk in chunk_pool:
		chunk.queue_free()
