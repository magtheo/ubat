# TerrainSystem.gd
extends Node3D

@onready var biome_manager = $BiomeManager
@onready var chunk_manager = $ChunkManager
@onready var chunk_controller = $ChunkController

var player = null
var world_integration = null

func _ready():
	print("GODOT:TerrainSystem: Initializing...")
	
	# Connect to game bridges
	var game_bridge = BridgeManager.game_bridge
	if game_bridge:
		game_bridge.connect("game_world_initialized", _on_game_world_initialized)
		print("GODOT:TerrainSystem: Connected to game_bridge signals")
	
	# Find player
	await get_tree().process_frame
	player = get_tree().get_first_node_in_group("player")
	if player:
		print("GODOT:TerrainSystem: Found player at", player.global_position)
	
	# Initialize world integration
	if biome_manager and chunk_manager:
		var event_bridge = BridgeManager.event_bridge
		if event_bridge:
			# Here we would connect to events, but we're now using signals instead
			print("GODOT:TerrainSystem: Using direct signal connections instead of event bus")
	
	print("GODOT:TerrainSystem: Initialization complete")

func _process(delta):
	# Update terrain with player position if available
	if player and chunk_controller:
		chunk_controller.update_player_position(player.global_position)
	
	# Process pending events from world integration if available
	# We don't need this anymore since we're using signals

func _on_game_world_initialized():
	print("GODOT:TerrainSystem: Game world initialized, setting up terrain...")
	
	# Get configuration from config bridge
	var config_bridge = BridgeManager.config_bridge
	if config_bridge:
		var seed_value = config_bridge.world_seed
		var width = config_bridge.world_width
		var height = config_bridge.world_height
		
		print("GODOT:TerrainSystem: Using configuration - Seed:", seed_value, ", Size:", width, "x", height)
		
		# Set up biome manager
		if biome_manager:
			biome_manager.set_seed(seed_value)
			biome_manager.set_world_dimensions(width, height)
			print("GODOT:TerrainSystem: Updated BiomeManager with seed", seed_value)
		
		# Force chunk controller update
		if chunk_controller:
			chunk_controller.force_update()
			print("GODOT:TerrainSystem: Forced chunk update")
	else:
		push_error("TerrainSystem: Config bridge not available")
