extends Node

var libchunk_generator  # reference to our C++ class
@onready var player: CharacterBody3D = $"../../CameraController"
@onready var thread_pool = $ThreadPool  # The node we wrote above

# Basic settings
const CHUNK_SIZE = 64
var seedsRandomized = false

# We'll track which chunks are loaded to avoid duplicates
var loaded_chunks = {}
var chunk_mutex := Mutex.new()

# Store the player's previous chunk coordinates
var prev_chunk_x = null
var prev_chunk_y = null

# Track which chunks should be removed - chunks beyond this distance will be unloaded
var cleanup_distance = 5

func _ready():
	# Create the GD extension class
	libchunk_generator = ChunkGenerator.new()
	if libchunk_generator:
		print("ChunkGenerator created successfully: ", libchunk_generator)
	else:
		push_error("Failed to create ChunkGenerator!")
		
	if !thread_pool:
		push_error("Threadpool not found")

	# Initialize with .tres paths + chunk size + seed
	libchunk_generator.initialize(CHUNK_SIZE)
	
	# Assign it to the thread pool so the pool can call it
	thread_pool.libchunk_generator = libchunk_generator
	
	var seed_node = load("res://project/terrain/SeedNode.gd").new()
	seed_node.noises_randomized.connect(_on_noises_randomized)
	seed_node.randomize_noises()

func _on_noises_randomized():
	print("TerrainManager.gd: Noises randomized, refreshing chunks")
	seedsRandomized = true
	
	# Clear loaded chunks
	for chunk_pos in loaded_chunks:
		# Remove chunk from scene
		var chunk_name = "Chunk_%d_%d" % [chunk_pos.x, chunk_pos.y]
		var chunk = get_node_or_null(chunk_name)
		if chunk:
			chunk.queue_free()

	loaded_chunks.clear()

	# Load initial chunks around (0,0)
	var player_pos = Vector2(0, 0)
	load_chunks_around_player(player_pos)

func load_chunks_around_player(player_pos: Vector2):
	var chunk_x = int(player_pos.x / CHUNK_SIZE)
	var chunk_y = int(player_pos.y / CHUNK_SIZE)
	
	var view_distance = 3
	var chunks_to_load = []
	
	# Calculate distances and prepare prioritized loading
	for dy in range(-view_distance, view_distance + 1):
		for dx in range(-view_distance, view_distance + 1):
			var cx = chunk_x + dx
			var cy = chunk_y + dy
			
			# Skip if already loaded
			var pos = Vector2i(cx, cy)
			if pos in loaded_chunks:
				continue
				
			# Calculate distance for priority
			var distance = sqrt(dx*dx + dy*dy)
			if distance <= view_distance:
				chunks_to_load.append({
					"cx": cx,
					"cy": cy,
					"distance": distance
				})
	
	# Sort by distance (closest first)
	chunks_to_load.sort_custom(func(a, b): return a.distance < b.distance)
	
	# Queue chunks for loading
	for chunk_data in chunks_to_load:
		request_chunk(chunk_data.cx, chunk_data.cy)
	
	# Clean up distant chunks
	cleanup_distant_chunks(chunk_x, chunk_y, cleanup_distance)

func request_chunk(cx: int, cy: int):
	var pos = Vector2i(cx, cy)
	if pos in loaded_chunks:
		return # Already loaded or requested
	
	# Mark in loaded_chunks so we don't request it again
	# false means it's requested but not yet loaded
	loaded_chunks[pos] = false
	
	# Enqueue the chunk task in the pool
	thread_pool.enqueue_chunk(cx, cy, CHUNK_SIZE)

func cleanup_distant_chunks(center_x: int, center_y: int, max_distance: int):
	var chunks_to_remove = []
	
	# Find chunks that are too far away
	for chunk_pos in loaded_chunks:
		var dx = chunk_pos.x - center_x
		var dy = chunk_pos.y - center_y
		var distance = sqrt(dx*dx + dy*dy)
		
		if distance > max_distance:
			chunks_to_remove.append(chunk_pos)
	
	# Remove the distant chunks
	for pos in chunks_to_remove:
		var chunk_name = "Chunk_%d_%d" % [pos.x, pos.y]
		var chunk = get_node_or_null(chunk_name)
		if chunk:
			chunk.queue_free()
		loaded_chunks.erase(pos)
	
	# Clean up cached textures in the chunk generator
	if chunks_to_remove.size() > 0:
		var min_chunk = Vector2i(center_x - max_distance, center_y - max_distance)
		var max_chunk = Vector2i(center_x + max_distance, center_y + max_distance)
		libchunk_generator.cleanup_chunk_caches(min_chunk, max_chunk)

func _process(_delta: float):
	if player and seedsRandomized:
		var player_pos_2d = Vector2(player.position.x, player.position.z)
		
		# Calculate the player's current chunk coordinates
		var current_chunk_x = int(player_pos_2d.x / CHUNK_SIZE)
		var current_chunk_y = int(player_pos_2d.y / CHUNK_SIZE)

		# Check if the player has moved to a new chunk
		if prev_chunk_x == null or prev_chunk_y == null or current_chunk_x != prev_chunk_x or current_chunk_y != prev_chunk_y:
			load_chunks_around_player(player_pos_2d)

			# Update the previous chunk coordinates
			prev_chunk_x = current_chunk_x
			prev_chunk_y = current_chunk_y
