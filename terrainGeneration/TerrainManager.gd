# TerrainManager.gd
extends Node3D

# Shader resources
@export var terrain_shader: Shader

# Texture resources
@export_group("Biome Textures")
@export var coral_texture: Texture2D
@export var sand_texture: Texture2D
@export var rock_texture: Texture2D
@export var kelp_texture: Texture2D
@export var lavarock_texture: Texture2D

# Noise resources
@export_group("Biome Noises")
@export var coral_noise: FastNoiseLite
@export var sand_noise: FastNoiseLite
@export var rock_noise: FastNoiseLite
@export var kelp_noise: FastNoiseLite
@export var volcanic_noise: FastNoiseLite

@export_group("Weight Noise")
@export var coral_weight: FastNoiseLite
@export var sand_weight: FastNoiseLite
@export var rock_weight: FastNoiseLite
@export var kelp_weight: FastNoiseLite
@export var volcanic_weight: FastNoiseLite

# Player and update settings
@export_group("Player Settings")
@export var player_path: NodePath
@export var update_interval: float = 0.1
@export var min_distance_for_update: float = 1.0

var biome_textures = {}
var noise_textures = {}
var biome_manager
var chunk_manager

var player: Node3D
var last_player_position: Vector3
var update_timer: float = 0.0

func _ready():
	print("TerrainManager: Starting initialization...")
	await setup_resources()
	print("TerrainManager: Resources setup complete")
	await setup_managers()
	print("TerrainManager: Managers setup complete")
	setup_player()
	print("TerrainManager: Player setup complete")
	
	if player:
		last_player_position = get_player_position()
		print("TerrainManager: Initial player position: ", last_player_position)
		chunk_manager.update_chunks(last_player_position)
		print("TerrainManager: Initial chunks updated")
	else:
		push_error("TerrainManager: No player node found! Please set player_path or add player to 'player' group")


func setup_resources():
	# Load shader if not set
	if !terrain_shader:
		terrain_shader = load("res://terrain_shader.gdshader")
	
	# Initialize biome textures
	biome_textures = {
		"coral": coral_texture if coral_texture else load("res://textures/coral.png"),
		"sand": sand_texture if sand_texture else load("res://textures/sand.png"),
		"rock": rock_texture if rock_texture else load("res://textures/rock.png"),
		"kelp": kelp_texture if kelp_texture else load("res://textures/kelp.png"),
		"lavarock": lavarock_texture if lavarock_texture else load("res://textures/lavarock.png")
	}
	print("biome textures", biome_textures)

func setup_managers():
	print("TerrainManager: Starting BiomeManager initialization...")
	var BiomeManagerScript = load("res://terrainGeneration/BiomeManager.gd")
	print("biome_manager script: ", BiomeManagerScript)
	if !BiomeManagerScript:
		push_error("TerrainManager: Failed to load BiomeManager script!")
		return
	
	print("TerrainManager: BiomeManager script loaded")
	
	biome_manager = BiomeManagerScript.new()
	if !biome_manager:
		push_error("TerrainManager: Failed to instantiate BiomeManager!")
		return
		
	print("TerrainManager: BiomeManager instantiated")
	add_child(biome_manager)
	print("TerrainManager: BiomeManager added as child")
	print("Biome manager object: ", biome_manager)
	
	
	print("pass noise to biome manager")
	if coral_noise and sand_noise and rock_noise and kelp_noise and volcanic_noise and coral_weight and sand_weight and rock_weight and kelp_weight and volcanic_weight:
		print("All noise configurations are present")
		biome_manager.initialize_biomes_with_noise(
			coral_noise, coral_weight,
			sand_noise, sand_weight,
			rock_noise, rock_weight,
			kelp_noise, kelp_weight,
			volcanic_noise, volcanic_weight
		)
	else:
		push_error("TerrainManager: Missing noise configurations!")
		print("Noise status:")
		print("- Coral:", coral_noise != null, "Weight:", coral_weight != null)
		print("- Sand:", sand_noise != null, "Weight:", sand_weight != null)
		print("- Rock:", rock_noise != null, "Weight:", rock_weight != null)
		print("- Kelp:", kelp_noise != null, "Weight:", kelp_weight != null)
		print("- Volcanic:", volcanic_noise != null, "Weight:", volcanic_weight != null)
		return
		
	# Wait for biome_manager to be ready
	#await biome_manager.ready
	print("TerrainManager: BiomeManager ready")
	
	# Verify biomes are initialized
	if !biome_manager.biomes or biome_manager.biomes.is_empty():
		push_error("TerrainManager: BiomeManager biomes not initialized!")
		return
	
	print("TerrainManager: BiomeManager biomes count: ", biome_manager.biomes.size())
	
	# Print noise information for debugging
	print("TerrainManager: Checking noise setup...")
	if coral_noise and sand_noise and rock_noise and kelp_noise and volcanic_noise:
		print("TerrainManager: Using custom noise resources")
		noise_textures = {
			"coral": coral_noise,
			"sand": sand_noise,
			"rock": rock_noise,
			"kelp": kelp_noise,
			"volcanic": volcanic_noise
		}
		print("noise: ", noise_textures)

	else:
		print("TerrainManager: Attempting to use default biome manager noise")
		# Verify each biome type exists before accessing
		var has_all_biomes = true
		for biome_type in [
			biome_manager.BiomeType.CORAL_REEF,
			biome_manager.BiomeType.SANDY_BOTTOM,
			biome_manager.BiomeType.ROCKY_OUTCROP,
			biome_manager.BiomeType.KELP_FOREST,
			biome_manager.BiomeType.VOLCANIC_VENT
		]:
			if !biome_manager.biomes.has(biome_type):
				push_error("TerrainManager: Missing biome type: " + str(biome_type))
				has_all_biomes = false
				break
				
		if has_all_biomes:
			noise_textures = {
				"coral": biome_manager.biomes[biome_manager.BiomeType.CORAL_REEF].noise,
				"sand": biome_manager.biomes[biome_manager.BiomeType.SANDY_BOTTOM].noise,
				"rock": biome_manager.biomes[biome_manager.BiomeType.ROCKY_OUTCROP].noise,
				"kelp": biome_manager.biomes[biome_manager.BiomeType.KELP_FOREST].noise,
				"volcanic": biome_manager.biomes[biome_manager.BiomeType.VOLCANIC_VENT].noise
			}
			print("TerrainManager: Default noise textures set up")
		else:
			push_error("TerrainManager: Failed to set up noise textures - missing biomes")
			return
	
	print("TerrainManager: Starting ChunkManager initialization...")
	var ChunkManagerScript = load("res://terrainGeneration/chunk/ChunkManager.gd")
	if !ChunkManagerScript:
		push_error("TerrainManager: Failed to load ChunkManager script!")
		return
		
	chunk_manager = ChunkManagerScript.new(biome_manager, terrain_shader, biome_textures, noise_textures)
	if !chunk_manager:
		push_error("TerrainManager: Failed to instantiate ChunkManager!")
		return
		
	add_child(chunk_manager)
	print("TerrainManager: ChunkManager initialized")



func setup_player():
	if player_path:
		player = get_node(player_path)
	else:
		var players = get_tree().get_nodes_in_group("player")
		if players.size() > 0:
			player = players[0]

func _process(delta: float):
	update_timer += delta
	
	if update_timer >= update_interval:
		update_timer = 0.0
		var current_position = get_player_position()
		
		if current_position.distance_to(last_player_position) > min_distance_for_update:
			chunk_manager.update_chunks(current_position)
			last_player_position = current_position

func get_player_position() -> Vector3:
	if !player:
		return Vector3.ZERO
	return player.global_position

func force_update_chunks():
	if player:
		chunk_manager.update_chunks(get_player_position())

func set_render_distance(distance: int):
	chunk_manager.RENDER_DISTANCE = distance
	force_update_chunks()

func _exit_tree():
	if chunk_manager:
		chunk_manager.cleanup()
