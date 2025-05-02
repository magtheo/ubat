extends Node

signal bridges_initialized

# Bridge components
var game_init_helper = null
var game_bridge = null
# var config_bridge = null
var network_bridge = null
var event_bridge = null
var terrain_bridge = null

var all_bridges_initialized = false

# Called when the node enters the scene tree for the first time
# In your bridge_manager.gd
func _ready():
	print("BridgeManager: Initializing...")
	
	# Create the game init helper
	game_init_helper = GameInitHelper.new()
	add_child(game_init_helper)

	# --- Deferred Initialization ---
	# Don't call _initialize_bridges immediately. Wait until the game
	# signals that core systems (including terrain) are likely ready.
	# You might need a signal from your Rust setup or use a timer.
	# For simplicity, let's use call_deferred. This isn't robust timing,
	# but demonstrates the principle. A signal is better.
	call_deferred("_attempt_bridge_initialization")
	print("BridgeManager: _ready() complete, deferred _attempt_bridge_initialization.")
	# Alternative: Connect to a signal emitted by GameInitHelper/SystemInitializer

func _attempt_bridge_initialization():
	# Check if the underlying Rust system is ready (if possible)
	if game_init_helper.is_system_ready():
		print("BridgeManager: System is ready, proceeding with bridge initialization.")
		_initialize_bridges()
	else:
		# System not ready yet, retry after a short delay
		print("BridgeManager: System not ready yet, retrying bridge init shortly...")
		await get_tree().create_timer(0.5).timeout # Wait 0.5 seconds
		_attempt_bridge_initialization() # Try again


func _initialize_bridges():
	print("BridgeManager: Running _initialize_bridges...") # Log entry point

	# Fetch standard bridges via GameInitHelper first
	_fetch_standard_bridge_instances() # Gets game/network/event bridges

	# --- Find and ASSIGN TerrainBridge directly ---
	var terrain_system_node = get_tree().root.get_node_or_null("TerrainSystem")
	if is_instance_valid(terrain_system_node):
		print("BridgeManager: Found TerrainSystem node.") # Log success
		# Find the child and ASSIGN it to the class variable 'terrain_bridge'
		self.terrain_bridge = terrain_system_node.get_node_or_null("TerrainBridge")
		if is_instance_valid(self.terrain_bridge):
			print("BridgeManager: Successfully found and assigned self.terrain_bridge.") # Log success
		else:
			# Push error if TerrainBridge child isn't found under TerrainSystem
			push_error("BridgeManager: Found TerrainSystem but FAILED to find/assign TerrainBridge child!")
	else:
		# Push error if TerrainSystem itself isn't found
		push_error("BridgeManager: FAILED to find TerrainSystem node! Cannot get TerrainBridge.")

	# Verify all bridges, NOW including the potentially assigned terrain_bridge
	if _verify_bridges(): # _verify_bridges checks the class variable self.terrain_bridge
		all_bridges_initialized = true
		print("BridgeManager: All bridges verified successfully.")
		emit_signal("bridges_initialized")
		print("BridgeManager: 'bridges_initialized' signal emitted.")
	else:
		push_error("BridgeManager: One or more bridges failed verification.")


func _fetch_standard_bridge_instances():
	# Get bridges directly from the game_init_helper
	game_bridge = game_init_helper.get_game_bridge()
	network_bridge = game_init_helper.get_network_bridge()
	event_bridge = game_init_helper.get_event_bridge()


# Verify that all bridges are properly initialized
func _verify_bridges() -> bool:
	var all_ok = true
	if not is_instance_valid(game_bridge):
		push_error("BridgeManager: Game bridge not found/initialized")
		all_ok = false
	if not is_instance_valid(network_bridge):
		push_error("BridgeManager: Network bridge not found/initialized")
		all_ok = false
	if not is_instance_valid(event_bridge):
		push_error("BridgeManager: Event bridge not found/initialized")
		all_ok = false
	# --- Verify Terrain Bridge ---
	if not is_instance_valid(terrain_bridge):
		# This might be expected if init is still ongoing, adjust severity?
		push_error("BridgeManager: Terrain bridge not found/initialized")
		all_ok = false
	return all_ok

# --- Add Getter for Terrain Bridge ---
func get_terrain_bridge() -> Node:
	# No need to check all_bridges_initialized flag strictly if we just return the var
	# but it might be null if initialization failed or hasn't completed.
	if not is_instance_valid(terrain_bridge):
		push_warning("BridgeManager: get_terrain_bridge() called but bridge is not valid/ready.")
	return terrain_bridge

# Check if bridges are initialized
func are_bridges_initialized() -> bool:
	# Maybe refine this to check specifically if terrain_bridge is valid too
	return all_bridges_initialized and is_instance_valid(terrain_bridge)

		
