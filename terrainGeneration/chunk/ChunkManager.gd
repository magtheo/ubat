extends Node3D

const CHUNK_SIZE = 32  # Adjust as needed
const RENDER_DISTANCE = 5  # Number of chunks in each direction

var chunk_scene = preload("res://terrainGeneration/chunk/chunk.tscn")
var active_chunks = {}
var chunk_thread_pool = []
var biome_manager: BiomeManager

func _init(bm: BiomeManager):
	biome_manager = bm

func _ready():
	# Initialize thread pool
	for i in range(4):  # Adjust number of threads as needed
		var thread = Thread.new()
		chunk_thread_pool.append(thread)

func update_chunks(player_position: Vector3):
	var chunk_coords = world_to_chunk_coordinates(player_position)

	# Determine which chunks should be active
	var should_be_active = {}
	for x in range(chunk_coords.x - RENDER_DISTANCE, chunk_coords.x + RENDER_DISTANCE + 1):
		for z in range(chunk_coords.z - RENDER_DISTANCE, chunk_coords.z + RENDER_DISTANCE + 1):
			should_be_active[Vector2(x, z)] = true

	# Remove chunks that are no longer needed
	for coord in active_chunks.keys():
		if not should_be_active.has(coord):
			remove_chunk(coord)

	# Add new chunks
	for coord in should_be_active.keys():
		if not active_chunks.has(coord):
			add_chunk(coord)

func add_chunk(coord: Vector2):
	var chunk = chunk_scene.instantiate()
	chunk.initialize(coord, CHUNK_SIZE, biome_manager)
	active_chunks[coord] = chunk
	add_child(chunk)
	
	# Start chunk generation in a separate thread
	var available_thread = get_available_thread()
	if available_thread:
		available_thread.start(Callable(chunk, "generate_terrain"))

func remove_chunk(coord: Vector2):
	var chunk = active_chunks[coord]
	active_chunks.erase(coord)
	chunk.queue_free()

func world_to_chunk_coordinates(position: Vector3) -> Vector2:
	return Vector2(floor(position.x / CHUNK_SIZE), floor(position.z / CHUNK_SIZE))

func get_available_thread() -> Thread:
	for thread in chunk_thread_pool:
		if not thread.is_alive():
			return thread
	return null
