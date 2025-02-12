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

# Data structures
var loaded_chunks: Dictionary = {}    # Maps Vector2 chunk coordinates to chunk nodes.
var noise_seed: int = -1     # Will be set by the server; -1 means not set.

func _ready() -> void:
	# Connect to the NetworkManager to receive the noise seed.
	if Engine.has_singleton("NetworkManager"):
		var net_mgr = Engine.get_singleton("NetworkManager")
		net_mgr.connect("noise_seed_received", Callable(self, "set_noise_seed"))
	else:
		print("TerrainManager: No NetworkManager found.")
		set_noise_seed(12345)  # Default noise seed for testing.
	set_process(true)

func set_noise_seed(seed: int) -> void:
	noise_seed = seed
	print("TerrainManager: Noise seed set to ", noise_seed)
	# With the seed set, you can (re)generate terrain as needed.
	update_chunks(get_viewer_position())

func get_viewer_position() -> Vector3:
	# Look for nodes in the group "player"
	var players := get_tree().get_nodes_in_group("CameraController")
	if players.size() > 0:
		print("player found")
		# Return the global position of the first player found.
		return players[0].global_transform.origin
	else:
		print("no player found")
		# Fallback: if no player is found, return the origin.
		return Vector3.ZERO


func _process(delta: float) -> void:
	if noise_seed != -1:
		update_chunks(get_viewer_position())

# Convert a world position to chunk coordinates.
func world_to_chunk(world_pos: Vector3) -> Vector2:
	return Vector2(int(floor(world_pos.x / chunk_size)), int(floor(world_pos.z / chunk_size)))

# Determine which chunks need to be loaded/unloaded.
func update_chunks(player_pos: Vector3) -> void:
	var current_chunk_coord: Vector2 = world_to_chunk(player_pos)
	var chunks_to_load: Array = []
	
	# Determine which chunks within the loading radius are missing.
	for x in range(current_chunk_coord.x - loading_radius, current_chunk_coord.x + loading_radius + 1):
		for z in range(current_chunk_coord.y - loading_radius, current_chunk_coord.y + loading_radius + 1):
			var coord: Vector2 = Vector2(x, z)
			if not loaded_chunks.has(coord):
				chunks_to_load.append(coord)
	
	# Request new chunks (if we haven't exceeded the maximum)
	for coord in chunks_to_load:
		if loaded_chunks.size() < max_active_chunks:
			load_chunk(coord)
		else:
			print("TerrainManager: Max active chunks reached.")
	
	# Unload chunks that fall outside the unloading radius.
	var to_unload: Array = []
	for coord in loaded_chunks.keys():
		if abs(coord.x - current_chunk_coord.x) > unloading_radius or abs(coord.y - current_chunk_coord.y) > unloading_radius:
			to_unload.append(coord)
	
	for coord in to_unload:
		unload_chunk(coord)

# Asynchronously request a chunk be generated.
func load_chunk(chunk_coord: Vector2) -> void:
	print("TerrainManager: Requesting generation for chunk ", chunk_coord)
	# In production code, you would call your C++ module to generate the chunk
	# asynchronously, passing in 'chunk_coord' and 'noise_seed'.
	# Here we simulate asynchronous generation with a Timer.
	var timer: Timer = Timer.new()
	timer.wait_time = 0.1  # Simulated generation delay.
	timer.one_shot = true
	# Bind the chunk_coord as an extra argument to the callback.
	timer.connect("timeout", Callable(self, "_on_chunk_generated").bind(chunk_coord))
	add_child(timer)
	timer.start()

# Remove a chunk that is no longer needed.
func unload_chunk(chunk_coord: Vector2) -> void:
	if loaded_chunks.has(chunk_coord):
		var chunk_node: Node = loaded_chunks[chunk_coord]
		chunk_node.queue_free()
		loaded_chunks.erase(chunk_coord)
		emit_signal("chunk_unloaded", chunk_coord)
		print("TerrainManager: Chunk unloaded at ", chunk_coord)

# Callback for when a chunk has been generated.
func _on_chunk_generated(chunk_coord: Vector2) -> void:
	# In real code, you’d receive data from the C++ module (e.g. an ArrayMesh).
	# Here we simulate it by creating a simple BoxMesh.
	var chunk_node: MeshInstance3D = MeshInstance3D.new()
	var box_mesh: BoxMesh = BoxMesh.new()
	box_mesh.size = Vector3(chunk_size, 1, chunk_size)
	chunk_node.mesh = box_mesh
	chunk_node.translation = Vector3(chunk_coord.x * chunk_size, 0, chunk_coord.y * chunk_size)
	add_child(chunk_node)
	
	loaded_chunks[chunk_coord] = chunk_node
	emit_signal("chunk_loaded", chunk_coord)
	print("TerrainManager: Chunk loaded at ", chunk_coord)
