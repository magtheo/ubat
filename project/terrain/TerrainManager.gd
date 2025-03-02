extends Node

var libchunk_generator  # reference to our C++ class
@onready var player: CharacterBody3D = $"../../CameraController"

# Adjust to your actual resource paths:
'const PATH_CORRAL   = "res://Noise/CorralNoise.tres"
const PATH_SAND     = "res://Noise/SandNoise.tres"
const PATH_ROCK     = "res://Noise/RockNoise.tres"
const PATH_KELP     = "res://Noise/KelpNoise.tres"
const PATH_LAVAROCK = "res://Noise/LavaRockNoise.tres"

const PATH_SECTION  = "res://Noise/SectionNoise.tres"
const PATH_BLEND    = "res://Noise/BlendNoise.tres"'

# Basic settings
const CHUNK_SIZE = 64
var seedsRandomized = false

# We'll track which chunks are loaded to avoid duplicates
var loaded_chunks = {}
var chunk_mutex := Mutex.new()

# Store the player's previous chunk coordinates
var prev_chunk_x = null
var prev_chunk_y = null

# TODO: add logic for removing chunks
# TODO: LOD (Level of Detail): Add a LOD system that renders distant chunks with simpler meshes and less detail, gradually increasing detail as the player approaches.

func _ready():
	# Create the GD extension class
	libchunk_generator = ChunkGenerator.new()
	if libchunk_generator:
		print("ChunkGenerator created successfully: ", libchunk_generator)
	else:
		push_error("Failed to create ChunkGenerator!")

	var seed_node = load("res://project/terrain/SeedNode.gd").new()

	seed_node.noises_randomized.connect(_on_noises_randomized)
	seed_node.randomize_noises()

	# Initialize with .tres paths + chunk size + seed
	libchunk_generator.initialize(
		CHUNK_SIZE,
	)

	# Load initial chunks around (0,0)
	var player_pos = Vector2(0, 0)
	# load_chunks_around_player(player_pos)

func _on_noises_randomized():
	print("TerrainManager.gd: Noises randomized, refreshing chunks")
	seedsRandomized = true
	# Clear loaded chunks
	for chunk_pos in loaded_chunks:
		# Remove chunk from scene
		var chunk = get_node(str(chunk_pos))
		if chunk:
			chunk.queue_free()

	loaded_chunks.clear()

	# Load initial chunks around (0,0)
	var player_pos = Vector2(0, 0)
	load_chunks_around_player(player_pos)

func load_chunks_around_player(player_pos: Vector2):
	# TODO: should only load chunks when the player moves to new chunks
	# example: load a 3x3 area around the chunk containing player
	var chunk_x = float(player_pos.x) / float(CHUNK_SIZE)
	var chunk_y = float(player_pos.y) / float(CHUNK_SIZE)

	for dy in range(-1, 2):
		for dx in range(-1, 2):
			var cx = chunk_x + dx
			var cy = chunk_y + dy
			request_chunk(cx, cy)

# Pre-generate biome data on the main thread
func generate_biome_data(cx: int, cy: int, chunk_size: int) -> Dictionary:
	var biome_data = {}
	for y in range(chunk_size):
		for x in range(chunk_size):
			var world_x = cx * chunk_size + x
			var world_y = cy * chunk_size + y
			var key = Vector2i(x, y)
			biome_data[key] = BiomeMask.get_biome_color(world_x, world_y)
	return biome_data

func request_chunk(cx: int, cy: int):
	if Vector2i(cx, cy) in loaded_chunks:
		return # Already loaded
	
	# Use the C++ implementation to generate biome data
	var biome_data = libchunk_generator.generate_biome_data(cx, cy, CHUNK_SIZE)
	
	var thread = Thread.new()
	var result = thread.start(_thread_generate_chunk.bind(cx, cy, thread, biome_data))

	if result != OK:
		print("⚠️ Failed to start chunk generation thread.")
		return

	# Store the thread reference so we can clean it up later
	loaded_chunks[Vector2i(cx, cy)] = thread


func _thread_generate_chunk(cx: int, cy: int, thread: Thread, biome_data: Dictionary):
	print("TerrainManager: Generate chunk at: ", cx, cy)
	
	# Pass the pre-generated biome data to the chunk generator
	var chunk = libchunk_generator.generate_chunk_with_biome_data(cx, cy, biome_data)
	call_deferred("_on_chunk_thread_completed", cx, cy, chunk, thread)


func _on_chunk_thread_completed(cx: int, cy: int, chunk, thread: Thread):
	add_child(chunk)
	
	# Wait for thread to finish and clean up
	thread.wait_to_finish()
	
	# Update the loaded chunks dictionary
	if Vector2i(cx, cy) in loaded_chunks:
		loaded_chunks.erase(Vector2i(cx, cy))
	
	# Mark as loaded
	loaded_chunks[Vector2i(cx, cy)] = true
	print("TerrainManager: ✅ Chunk (", cx, ",", cy, ") generated.")


# This function is no longer needed as we handle everything in _on_chunk_thread_completed

'func spawn_biome_objects(cx: int, cy: int, chunk_data: PackedFloat32Array):
	# If you need to know which biome is dominant at each cell,
	# you can either store that info in an additional array
	# or re-run the same logic (section + blend) to figure it out
	# for each cell. For now, we do a simple "if height > 0.7 => place X object."

	for i in range(5):
		var lx = randi() % CHUNK_SIZE
		var ly = randi() % CHUNK_SIZE
		var idx = ly * CHUNK_SIZE + lx
		if chunk_data[idx] > 0.7:
			# Example: spawn a "LargeCoral" scene
			var coral_scene = preload("res://Objects/LargeCoral.tscn")
			var coral_inst = coral_scene.instantiate()
			add_child(coral_inst)
			coral_inst.position = Vector2(
				(cx * CHUNK_SIZE) + lx,
				(cy * CHUNK_SIZE) + ly
			)'

func _process(_delta: float):
	# If your player moves, call load_chunks_around_player(new_position)
	# to load new chunks.	
	#var player = get_node("Player")
	#print("player in TerrainManager: ", player)
	if player and seedsRandomized: # and seedRandomized
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
