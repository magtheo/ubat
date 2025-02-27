extends Node

var libchunk_generator  # reference to our C++ class
@onready var player: CharacterBody3D = $"../../CameraController"

# Adjust to your actual resource paths:
const PATH_CORRAL   = "res://Noise/CorralNoise.tres"
const PATH_SAND     = "res://Noise/SandNoise.tres"
const PATH_ROCK     = "res://Noise/RockNoise.tres"
const PATH_KELP     = "res://Noise/KelpNoise.tres"
const PATH_LAVAROCK = "res://Noise/LavaRockNoise.tres"

const PATH_SECTION  = "res://Noise/SectionNoise.tres"
const PATH_BLEND    = "res://Noise/BlendNoise.tres"

# Basic settings
const CHUNK_SIZE = 64
#const SEED       = 12345

# We'll track which chunks are loaded to avoid duplicates
var loaded_chunks = {}

# Store the player's previous chunk coordinates
var prev_chunk_x = null
var prev_chunk_y = null


func _ready():
	# Create the GD extension class
	libchunk_generator = ChunkGenerator.new()
	print("libchunk_generator: ", libchunk_generator)

	var seed_node = get_node("SeedNode")
	seed_node.connect("noises_randomized", _on_noises_randomized)

	# Initialize with .tres paths + chunk size + seed
	libchunk_generator.initialize(
		PATH_CORRAL,
		PATH_SAND,
		PATH_ROCK,
		PATH_KELP,
		PATH_LAVAROCK,
		PATH_SECTION,
		PATH_BLEND,
		CHUNK_SIZE,
		seed_node
	)
	#print("ChunkGenerator initialized with:", PATH_CORRAL, PATH_SAND, PATH_ROCK, PATH_KELP, PATH_LAVAROCK, PATH_SECTION, PATH_BLEND, CHUNK_SIZE, seed_node)

	# Load initial chunks around (0,0)
	var player_pos = Vector2(0, 0)
	load_chunks_around_player(player_pos)

func _on_noises_randomized():
	print("Noises randomized, refreshing chunks")
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

func request_chunk(cx: int, cy: int):
	if Vector2i(cx, cy) in loaded_chunks:
		return # Already loaded

	var thread = Thread.new()
	var result = thread.start(_thread_generate_chunk.bind(cx, cy))

	if result != OK:
		print("⚠️ Failed to start chunk generation thread.")
		return

	# Store the thread reference so we can clean it up later
	loaded_chunks[Vector2i(cx, cy)] = thread


func _thread_generate_chunk(cx: int, cy: int):
	print("Generating chunk at: ", cx, cy)
	var chunk_data = libchunk_generator.generate_chunk(cx, cy)

	# Ensure the thread is cleaned up after use
	call_deferred("on_chunk_generated", cx, cy, chunk_data)

	# Clean up thread (ensure it does not get garbage-collected improperly)
	var key = Vector2i(cx, cy)
	if key in loaded_chunks:
		var thread = loaded_chunks[key]
		thread.wait_to_finish()
		loaded_chunks.erase(key)


func on_chunk_generated(cx: int, cy: int, chunk_data):
	if chunk_data is Dictionary:
		if "vertices" in chunk_data:
			chunk_data = chunk_data["vertices"] as PackedFloat32Array
		else:
			push_error("❌ Error: Received Dictionary, but missing 'vertices' key!")
			return

	if not chunk_data is PackedFloat32Array:
		push_error("❌ Error: Invalid chunk data format!")
		return

	loaded_chunks[Vector2i(cx, cy)] = true
	print("✅ Chunk (", cx, ",", cy, ") generated. First cell:", chunk_data[0])

	# Optionally spawn objects
	#spawn_biome_objects(cx, cy, chunk_data)

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
	if player:
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
