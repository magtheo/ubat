extends Control

# Path to game scene
const GAME_WORLD = "res://project/scenes/GameWorld.tscn"

# References to bridges via BridgeManager
@onready var config_bridge = BridgeManager.config_bridge
@onready var game_bridge = BridgeManager.game_bridge
@onready var network_bridge = BridgeManager.network_bridge
@onready var game_init_helper = BridgeManager.game_init_helper

# References to panels
@onready var main_panel = $MainPanel
@onready var standalone_options = $StandaloneOptions
@onready var host_options = $HostOptions
@onready var client_options = $ClientOptions
@onready var loading_overlay = $LoadingOverlay
@onready var error_dialog = $ErrorDialog

# Called when the node enters the scene tree
# In main_menu.gd - look for the _ready() function
func _ready():
	# Verify that all bridges were successfully created
	if !config_bridge or !game_bridge or !network_bridge:
		push_error("Bridge components not available - check BridgeManager initialization")
		_show_error("Failed to initialize game components")
		return
	
	# Connect bridge signals
	config_bridge.connect("config_updated", _on_config_updated)
	config_bridge.connect("config_loaded", _on_config_loaded)
	config_bridge.connect("config_saved", _on_config_saved)
	game_bridge.connect("game_state_changed", _on_game_state_changed)
	game_bridge.connect("game_error", _on_game_error)
	
	# Create our init helper if not provided by BridgeManager
	if not game_init_helper:
		game_init_helper = GameInitHelper.new()
		add_child(game_init_helper) # Add it to the scene tree
		game_init_helper.debug_mode = true
	
	# IMPORTANT: Always set the bridges on the helper, even if it comes from BridgeManager
	game_init_helper.set_bridges(config_bridge, game_bridge)
	
	# The rest of your _ready() function...
	
	# Initialize configuration
	if not config_bridge.create_default_config():
		_show_error("Failed to create default configuration")
		return

	# Set random default seed if needed
	if config_bridge.world_seed == 0:
		config_bridge.world_seed = randi()
		config_bridge.apply_world_seed()
	
	# Initialize UI with current configuration values
	_setup_ui()
	
	# Make sure only main panel is visible at start
	_show_panel(main_panel)
	
	debug_log("Main menu initialized")

# Set up UI controls with current configuration values
func _setup_ui():
	debug_log("Setting up UI with configuration values")
	
	# First, check if nodes exist and print their paths for debugging
	var seed_input = find_node_recursive(standalone_options, "SeedInput")
	if seed_input:
		print("Found SeedInput at: ", (seed_input.get_path()))
		seed_input.value = config_bridge.world_seed
	else:
		debug_error("SeedInput not found in scene hierarchy")
	
	# Continue with other UI elements using the same pattern...
	var world_width_input = find_node_recursive(standalone_options, "WorldWidthInput")
	if world_width_input:
		world_width_input.value = config_bridge.world_width
	else:
		debug_error("WorldWidthInput not found")
		
	var world_height_input = find_node_recursive(standalone_options, "WorldHeightInput")
	if world_height_input:
		world_height_input.value = config_bridge.world_height
	else:
		debug_error("WorldHeightInput not found")
	
	# Host options
	var host_seed_input = find_node_recursive(host_options, "SeedInput") 
	if host_seed_input:
		host_seed_input.value = config_bridge.world_seed
	else:
		debug_error("SeedInput not found in HostOptions")
	
	var server_name_input = find_node_recursive(host_options, "ServerName")
	if server_name_input:
		server_name_input = config_bridge.get_custom_value("server_name") or "My Game Server"
	else:
		debug_error("ServerName not found")
	
	var port_input = find_node_recursive(host_options, "PortInput")
	if port_input:
		port_input.value = config_bridge.server_port
	else:
		debug_error("PortInput not found")
	
	var max_players_input = find_node_recursive(host_options, "MaxPlayersInput")
	if max_players_input:
		max_players_input.value = config_bridge.max_players
	else:
		debug_error("MaxPlayersInput not found")
	
	# Client options
	var server_address = find_node_recursive(client_options, "ServerAddress")
	if server_address:
		server_address = config_bridge.server_address
	else:
		debug_error("ServerAddress not found")
	
	var player_name = find_node_recursive(client_options, "PlayerName")
	if player_name:
		player_name = config_bridge.get_custom_value("player_name") or "Player"
	else:
		debug_error("PlayerName not found")
		
# Helper function to find nodes recursively by name
func find_node_recursive(parent, node_name):
	# First check direct children
	for child in parent.get_children():
		if child.name == node_name:
			return child
		
		# Check if this child has children
		if child.get_child_count() > 0:
			var found = find_node_recursive(child, node_name)
			if found:
				return found
	
	# Not found
	return null
	
# Shows only the specified panel, hides others
func _show_panel(panel):
	main_panel.visible = (panel == main_panel)
	standalone_options.visible = (panel == standalone_options)
	host_options.visible = (panel == host_options)
	client_options.visible = (panel == client_options)
	loading_overlay.visible = (panel == loading_overlay)

# MAIN PANEL BUTTON HANDLERS
func _on_StandaloneButton_pressed():
	debug_button_press("Standalone")
	_show_panel(standalone_options)

func _on_HostButton_pressed():
	debug_button_press("Host")
	_show_panel(host_options)

func _on_ClientButton_pressed():
	debug_button_press("Client")
	_show_panel(client_options)

func _on_QuitButton_pressed():
	debug_button_press("Quit")
	get_tree().quit()


func _on_StartStandaloneButton_pressed():
	debug_button_press("Start Standalone")
	
	# Show loading
	_show_panel(loading_overlay)
	
	# Find input elements using recursive search
	var seed_input = find_node_recursive(standalone_options, "SeedInput")
	var width_input = find_node_recursive(standalone_options, "WorldWidthInput")
	var height_input = find_node_recursive(standalone_options, "WorldHeightInput")
	
	# Check if all required inputs were found
	if not seed_input or not width_input or not height_input:
		debug_error("Required input elements not found in scene")
		_show_error("UI configuration error - please check scene setup")
		return
		
	# Get values from the input elements
	var seed_value = int(seed_input.value)  # Explicitly convert to int
	var width_value = int(width_input.value)  # Explicitly convert to int
	var height_value = int(height_input.value)  # Explicitly convert to int
	
	debug_log("Found inputs - Seed: " + str(seed_value) + ", Width: " + str(width_value) + ", Height: " + str(height_value))
	
	# Create options dictionary
	var options = {
		"world_seed": seed_value if seed_value > 0 else int(randi()),  # Ensure it's an int
		"world_width": width_value,
		"world_height": height_value
	}
	
	debug_log("Starting standalone game with options: " + str(options))
	
	# Check if game_init_helper is available
	if not game_init_helper:
		debug_error("GameInitHelper is not available")
		_show_error("Game initialization helper not available")
		return
	
	debug_log("GameInitHelper available, config_path: " + str(config_bridge.config_path))
	
	# Use the GameInitHelper to handle the initialization
	var init_result = game_init_helper.init_standalone(config_bridge.config_path, options)
	if init_result:
		debug_log("Game initialized successfully, waiting before scene transition")
		# Wait a bit for any backend initialization to complete
		await get_tree().create_timer(0.5).timeout
		
		# Change to game scene
		debug_log("Changing to game scene: " + GAME_WORLD)
		var error = get_tree().change_scene_to_file(GAME_WORLD)
		if error != OK:
			_show_error("Failed to load game scene (error " + str(error) + ")")
	else:
		debug_error("Failed to initialize standalone game")
		_show_error("Failed to initialize game in standalone mode")
		
func _on_RandomSeedStandaloneButton_pressed():
	print("Random Seed pressed")
	
	# Try to find the SeedInput node directly
	var seed_input = find_node_recursive(standalone_options, "SeedInput")
	if seed_input:
		print("Found SeedInput at: ", seed_input.get_path())
		seed_input.value = randi()  # Try with .value property
	else:
		print("SeedInput not found in scene hierarchy!")

# HOST MODE HANDLERS
func _on_StartServerButton_pressed():
	debug_button_press("Start Server")
	
	# Show loading
	_show_panel(loading_overlay)

	# Get input values
	var server_name = $HostOptions/ConfigContainer/ServerName.text
	var port = $HostOptions/ConfigContainer/PortContainer/PortInput.value
	var max_players = $HostOptions/ConfigContainer/MaxPlayersContainer/MaxPlayersInput.value
	var seed_value = $HostOptions/ConfigContainer/seed/SeedInput.value
	
	debug_log("Starting host game with: Server=" + server_name + ", Port=" + str(port) + ", Players=" + str(max_players) + ", Seed=" + str(seed_value))
	
	# Create options dictionary
	var options = {
		"world_seed": seed_value if seed_value > 0 else randi(),
		"server_port": port,
		"max_players": max_players,
		"server_name": server_name
	}
	
	# Apply all settings at once
	if not config_bridge.apply_multiple_settings(options, true):
		_show_error("Failed to apply server configuration")
		return
	
	# Set game mode to host
	if not config_bridge.set_game_mode(1, true):
		_show_error("Failed to set host mode")
		return
	
	# Initialize the network
	if not network_bridge.initialize_network(1, port, ""):
		_show_error("Failed to initialize network")
		return
	
	# Initialize the game
	if not game_bridge.initialize(config_bridge.config_path):
		_show_error("Failed to initialize game")
		return
	
	# Start the game
	if not game_bridge.start_game():
		_show_error("Failed to start game")
		return
	
	# Wait a bit for any backend initialization to complete
	await get_tree().create_timer(0.5).timeout
	
	# Change to game scene
	debug_log("Changing to game scene: " + GAME_WORLD)
	var error = get_tree().change_scene_to_file(GAME_WORLD)
	if error != OK:
		_show_error("Failed to load game scene (error " + str(error) + ")")

func _on_RandomSeedHostButton_pressed():
	$HostOptions/ConfigContainer/seed/SeedInput.value = randi()

# CLIENT MODE HANDLERS
func _on_ConnectButton_pressed():
	debug_button_press("Connect to Server")
	
	# Show loading
	_show_panel(loading_overlay)

	# You need to add these inputs to your ClientOptions node
	# or adjust these paths to match your actual node structure
	var server_address = ""
	var player_name = ""
	
	if $ClientOptions.has_node("ServerAddressInput"):
		server_address = $ClientOptions/ServerAddressInput.text
	else:
		_show_error("Server address input not found in UI")
		return
		
	if $ClientOptions.has_node("PlayerNameInput"):
		player_name = $ClientOptions/PlayerNameInput.text
	else:
		player_name = "Player" # Default name
	
	if server_address.strip_edges().empty():
		_show_error("Please enter a server address")
		return
	
	if player_name.strip_edges().empty():
		_show_error("Please enter a player name")
		return
	
	debug_log("Connecting to server: " + server_address + " as " + player_name)
	
	# Create options dictionary
	var options = {
		"server_address": server_address,
		"player_name": player_name
	}
	
	# Apply all settings at once
	if not config_bridge.apply_multiple_settings(options, true):
		_show_error("Failed to apply client configuration")
		return
	
	# Set game mode to client
	if not config_bridge.set_game_mode(2, true):
		_show_error("Failed to set client mode")
		return
	
	# Validate the configuration for client mode
	if not config_bridge.validate_for_mode(2):
		_show_error("Invalid client configuration")
		return
	
	# Initialize the network
	if not network_bridge.initialize_network(2, 0, server_address):
		_show_error("Failed to initialize network")
		return
	
	# Initialize the game
	if not game_bridge.initialize(config_bridge.config_path):
		_show_error("Failed to initialize game")
		return
	
	# Start the game
	if not game_bridge.start_game():
		_show_error("Failed to start game")
		return
	
	# Wait a bit for any backend initialization to complete
	await get_tree().create_timer(0.5).timeout
	
	# Change to game scene
	debug_log("Changing to game scene: " + GAME_WORLD)
	var error = get_tree().change_scene_to_file(GAME_WORLD)
	if error != OK:
		_show_error("Failed to load game scene (error " + str(error) + ")")

# BACK BUTTON HANDLERS
func _on_StandaloneOptions_BackButton_pressed():
	_show_panel(main_panel)

func _on_HostOptions_BackButton_pressed():
	_show_panel(main_panel)

func _on_ClientOptions_BackButton_pressed():
	_show_panel(main_panel)

# ERROR HANDLING
func _show_error(message):
	# Create error dialog if it doesn't exist
	if not has_node("ErrorDialog"):
		var dialog = AcceptDialog.new()
		dialog.name = "ErrorDialog"
		dialog.title = "Error"
		add_child(dialog)
		error_dialog = dialog
	
	# Return to main panel
	_show_panel(main_panel)
	
	# Show error dialog
	error_dialog.dialog_text = message
	error_dialog.popup_centered()
	
	debug_error(message)

# SIGNAL HANDLERS
func _on_game_error(error_message):
	_show_error(error_message)

func _on_game_state_changed(old_state, new_state, state_name):
	debug_log("Game state changed: " + state_name)

func _on_config_updated(key, value):
	debug_log("Config updated: " + str(key) + " = " + str(value))

func _on_config_loaded(success):
	debug_log("Config loaded: " + ("Success" if success else "Failed"))
	if success:
		_setup_ui()

func _on_config_saved(success):
	debug_log("Config saved: " + ("Success" if success else "Failed"))

# Debug utility functions
func debug_log(message):
	print("[GODOT DEBUG] " + message)
	
func debug_error(message):
	push_error("[GODOT ERROR] " + message)
	
func debug_button_press(button_name):
	debug_log("Button pressed: " + button_name)
	
func debug_bridge_state():
	debug_log("Godot Bridge Status:")
	debug_log("- config_bridge: " + str(config_bridge))
	debug_log("- game_bridge: " + str(game_bridge))
	debug_log("- network_bridge: " + str(network_bridge))
	debug_log("- game_init_helper: " + str(game_init_helper))
	
	if config_bridge:
		debug_log("  - config_path: " + str(config_bridge.config_path))
		debug_log("  - network_mode: " + str(config_bridge.network_mode))
		debug_log("  - world_seed: " + str(config_bridge.world_seed))
