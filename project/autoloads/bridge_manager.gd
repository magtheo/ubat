extends Node

signal bridges_initialized

# Export paths to any resources needed by the bridges
@export var default_config_path: String = "user://config.json"

# Bridge components
var game_init_helper = null
var game_bridge = null
var config_bridge = null
var network_bridge = null
var event_bridge = null

var all_bridges_initialized = false

# Called when the node enters the scene tree for the first time
func _ready():
	print("BridgeManager: Initializing...")
	
	# Create the game init helper
	game_init_helper = GameInitHelper.new()
	add_child(game_init_helper)
	game_init_helper.debug_mode = true
	
	# Wait a frame to ensure everything is set up
	await get_tree().process_frame
	
	# Initialize all bridge components
	_initialize_bridges()
	
# Initialize bridge components
func _initialize_bridges():
	# Get access to bridges through SystemInitializer (via GameInitHelper)
	if game_init_helper.is_system_ready():
		# Access the singleton SystemInitializer to get the bridges
		_fetch_bridge_instances()
		
		# Make sure bridges are properly configured
		if _verify_bridges():
			all_bridges_initialized = true
			emit_signal("bridges_initialized")
			print("BridgeManager: All bridges initialized successfully")
		else:
			push_error("BridgeManager: One or more bridges failed to initialize")
	else:
		push_error("BridgeManager: System initializer is not ready")

# Fetch bridge instances from the SystemInitializer
func _fetch_bridge_instances():
	# These calls go through the GameInitHelper which accesses the SystemInitializer
	var helper_script = load("res://addons/game_init_helper.gdns")
	if helper_script:
		var helper = helper_script.new()
		game_bridge = helper.get_game_bridge()
		config_bridge = helper.get_config_bridge()
		network_bridge = helper.get_network_bridge()
		event_bridge = helper.get_event_bridge()

# Verify that all bridges are properly initialized
func _verify_bridges() -> bool:
	if not game_bridge:
		push_error("BridgeManager: Game bridge not initialized")
		return false
	
	if not config_bridge:
		push_error("BridgeManager: Config bridge not initialized")
		return false
	
	if not network_bridge:
		push_error("BridgeManager: Network bridge not initialized")
		return false
	
	if not event_bridge:
		push_error("BridgeManager: Event bridge not initialized")
		return false
	
	return true
		
# Check if bridges are initialized
func are_bridges_initialized() -> bool:
	return all_bridges_initialized
