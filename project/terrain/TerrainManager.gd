extends Node

var chunk_generator  # reference to our C++ class

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
const SEED       = 12345

# We'll track which chunks are loaded to avoid duplicates
var loaded_chunks = {}

func _ready():
	# Create the GD extension class
	chunk_generator = ChunkGenerator.new()

	# Initialize with .tres paths + chunk size + seed
	chunk_generator.initialize(
		PATH_CORRAL,
		PATH_SAND,
		PATH_ROCK,
		PATH_KELP,
		PATH_LAVAROCK,
		PATH_SECTION,
		PATH_BLEND,
		CHUNK_SIZE,
		SEED
	)

	# Load initial chunks around (0,0)
	var player_pos = Vector2(0, 0)
	load_chunks_around_player(player_pos)

func load_chunks_around_player(player_pos: Vector2):
	# example: load a 3x3 area around the chunk containing player
	var chunk_x = int(player_pos.x) / CHUNK_SIZE
	var chunk_y = int(player_pos.y) / CHUNK_SIZE

	for dy in range(-1, 2):
		for dx in range(-1, 2):
			var cx = chunk_x + dx
			var cy = chunk_y + dy
			request_chunk(cx, cy)

func request_chunk(cx: int, cy: int):
	if Vector2i(cx, cy) in loaded_chunks:
		return # already loaded

	# We'll generate in a thread
	var thread = Thread.new()
	thread.run(self, "_thread_generate_chunk", [cx, cy])

func _thread_generate_chunk(params: Array):
	var cx = params[0]
	var cy = params[1]

	var chunk_data = chunk_generator.generate_chunk(cx, cy)
	call_deferred("on_chunk_generated", cx, cy, chunk_data)

func on_chunk_generated(cx: int, cy: int, chunk_data: PackedFloat32Array):
	loaded_chunks[Vector2i(cx, cy)] = true
	# Build a terrain mesh or tilemap from chunk_data
	# For demonstration, we just log:
	print("Chunk (", cx, ",", cy, ") generated. First cell:", chunk_data[0])

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

func _process(delta: float):
	# If your player moves, call load_chunks_around_player(new_position)
	# to load new chunks.
	var player = get_node("Player")
	if player:
		load_chunks_around_player(player.position)
