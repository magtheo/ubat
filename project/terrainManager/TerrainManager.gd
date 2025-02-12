# TerrainManager.gd
extends Node
class_name TerrainManager

signal chunk_loaded(chunk_coord)
signal chunk_unloaded(chunk_coord)
signal chunk_generation_failed(chunk_coord, error_message)

# Configuration options (in world units or chunk counts)
@export var chunk_size: int = 64
@export var loading_radius: int = 3   # in chunks
@export var unloading_radius: int = 4
@export var max_active_chunks: int = 50

# Preloaded noise resources (created as files in your project)
@export var global_noise: FastNoiseLite
@export var corral_noise: FastNoiseLite
@export var sand_noise: FastNoiseLite
@export var rock_noise: FastNoiseLite
@export var kelp_noise: FastNoiseLite
@export var blending_noise: FastNoiseLite
@export var lavaRock_noise: FastNoiseLite


# Data structures
var noise_seeds: Dictionary = {}  # Will hold seeds for various biomes
var loaded_chunks: Dictionary = {}    # Maps Vector2 chunk coordinates to chunk nodes.

func _ready() -> void:
	if Engine.has_singleton("NetworkManager"):
		var net_mgr = Engine.get_singleton("NetworkManager")
		net_mgr.connect("noise_seeds_received", Callable(self, "set_noise_seeds"))
	else:
		print("TerrainManager: No NetworkManager found. Using default seeds for testing.")
		set_noise_seeds({
			"global": 12345,
			"corral": 23456,
			"sand": 34567,
			"rock": 45678,
			"kelp": 56789,
			"blending": 67890
		})
	set_process(true)


# This function is called when the server (or the test bypass) sends a set of seeds.
func set_noise_seeds(seeds: Dictionary) -> void:
	noise_seeds = seeds
	print("TerrainManager: Noise seeds set to", noise_seeds)
	
	# Update each preloaded noise resource with the corresponding seed.
	if global_noise:
		global_noise.seed = seeds.get("global", 0)
	if corral_noise:
		corral_noise.seed = seeds.get("corral", 0)
	if sand_noise:
		sand_noise.seed = seeds.get("sand", 0)
	if rock_noise:
		rock_noise.seed = seeds.get("rock", 0)
	if kelp_noise:
		kelp_noise.seed = seeds.get("kelp", 0)
	if blending_noise:
		blending_noise.seed = seeds.get("blending", 0)
	
	# Now that the noise resources are updated, trigger a terrain update.
	update_chunks(get_viewer_position())

# Retrieves the player's position from the scene (using a group for simplicity)
func get_viewer_position() -> Vector3:
	var players = get_tree().get_nodes_in_group("player")
	if players.size() > 0:
		return players[0].global_transform.origin
	return Vector3.ZERO

func _process(delta: float) -> void:
	if noise_seeds.size() > 0:
		update_chunks(get_viewer_position())

# Converts a world position to chunk coordinates.
func world_to_chunk(world_pos: Vector3) -> Vector2:
	return Vector2(int(floor(world_pos.x / chunk_size)), int(floor(world_pos.z / chunk_size)))

# Determine which chunks need to be loaded/unloaded based on the player's position.
func update_chunks(player_pos: Vector3) -> void:
	var current_chunk_coord: Vector2 = world_to_chunk(player_pos)
	var chunks_to_load: Array = []
	
	# Find missing chunks within the loading radius.
	for x in range(current_chunk_coord.x - loading_radius, current_chunk_coord.x + loading_radius + 1):
		for z in range(current_chunk_coord.y - loading_radius, current_chunk_coord.y + loading_radius + 1):
			var coord: Vector2 = Vector2(x, z)
			if not loaded_chunks.has(coord):
				chunks_to_load.append(coord)
	
	# Request new chunks.
	for coord in chunks_to_load:
		if loaded_chunks.size() < max_active_chunks:
			load_chunk(coord)
		else:
			print("TerrainManager: Max active chunks reached.")
	
	# Unload chunks outside the unloading radius.
	var to_unload: Array = []
	for coord in loaded_chunks.keys():
		if abs(coord.x - current_chunk_coord.x) > unloading_radius or abs(coord.y - current_chunk_coord.y) > unloading_radius:
			to_unload.append(coord)
	
	for coord in to_unload:
		unload_chunk(coord)# Simulate asynchronous chunk generation (in production, this would call your C++ module).
func load_chunk(chunk_coord: Vector2) -> void:
	print("TerrainManager: Requesting generation for chunk", chunk_coord)
	var timer: Timer = Timer.new()
	timer.wait_time = 0.1
	timer.one_shot = true
	timer.connect("timeout", Callable(self, "_on_chunk_generated").bind(chunk_coord))
	add_child(timer)
	timer.start()

# Unload a chunk that is no longer needed.
func unload_chunk(chunk_coord: Vector2) -> void:
	if loaded_chunks.has(chunk_coord):
		var chunk_node: Node = loaded_chunks[chunk_coord]
		chunk_node.queue_free()
		loaded_chunks.erase(chunk_coord)
		emit_signal("chunk_unloaded", chunk_coord)
		print("TerrainManager: Chunk unloaded at", chunk_coord)

# Callback when a chunk's generation is simulated as complete.
func _on_chunk_generated(chunk_coord: Vector2) -> void:
	var chunk_node: MeshInstance3D = MeshInstance3D.new()
	var box_mesh: BoxMesh = BoxMesh.new()
	box_mesh.size = Vector3(chunk_size, 1, chunk_size)
	chunk_node.mesh = box_mesh
	chunk_node.translate(Vector3(chunk_coord.x * chunk_size, 0, chunk_coord.y * chunk_size))
	add_child(chunk_node)
	
	loaded_chunks[chunk_coord] = chunk_node
	emit_signal("chunk_loaded", chunk_coord)
	print("TerrainManager: Chunk loaded at", chunk_coord)
