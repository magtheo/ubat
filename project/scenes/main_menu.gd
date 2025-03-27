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
		game_init_helper.set_bridges(config_bridge, game_bridge)
		game_init_helper.debug_mode = true
	
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
	# Standalone options
	$StandaloneOptions/ConfigConteiner/seed/SeedInput.value = config_bridge.world_seed
	$StandaloneOptions/ConfigConteiner/WorldHeightContener/WorldHeightInput.value = config_bridge.world_width
	$StandaloneOptions/ConfigConteiner/WorldWidthContener/WorldWidthInput.value = config_bridge.world_height
	
	# Host options
	$HostOptions/ConfigConteiner/seed/SeedInput.value = config_bridge.world_seed
	$HostOptions/ConfigConteiner/ServerName.text = config_bridge.get_custom_value("server_name") or "My Game Server"
	$HostOptions/ConfigConteiner/PortContainer/PortInput.value = config_bridge.server_port
	$HostOptions/ConfigConteiner/MaxPlayersContainer/MaxPlayersInput.value = config_bridge.max_players
	
	# Client options
	$ClientOptions/ServerAddressInput.text = config_bridge.server_address
	$ClientOptions/PlayerNameInput.text = config_bridge.get_custom_value("player_name") or "Player"

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

# STANDALONE MODE HANDLERS

func _on_StartStandaloneButton_pressed():
	debug_button_press("Start Standalone")
	
	# Show loading
	_show_panel(loading_overlay)
	
	# Get world seed value
	var seed_value = $StandaloneOptions/SeedInput.value
	
	# Create options dictionary
	var options = {
		"world_seed": seed_value if seed_value > 0 else randi(),
		"world_width": $StandaloneOptions/WorldWidthInput.value,
		"world_height": $StandaloneOptions/WorldHeightInput.value
	}
	
	debug_log("Starting standalone game with options: " + str(options))
	
	# Apply all settings at once
	if not config_bridge.apply_multiple_settings(options, true):
		_show_error("Failed to apply game configuration")
		return
	
	# Set game mode to standalone
	if not config_bridge.set_game_mode(0, true):
		_show_error("Failed to set standalone mode")
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

func _on_RandomSeedStandaloneButton_pressed():
	$StandaloneOptions/SeedInput.value = randi()

# HOST MODE HANDLERS

func _on_StartServerButton_pressed():
	debug_button_press("Start Server")
	
	# Show loading
	_show_panel(loading_overlay)

	# Get input values
	var server_name = $HostOptions/ServerNameInput.text
	var port = $HostOptions/PortInput.value
	var max_players = $HostOptions/MaxPlayersInput.value
	var seed_value = $HostOptions/SeedInput.value
	
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
	$HostOptions/SeedInput.value = randi()

# CLIENT MODE HANDLERS

func _on_ConnectButton_pressed():
	debug_button_press("Connect to Server")
	
	# Show loading
	_show_panel(loading_overlay)

	# Get input values
	var server_address = $ClientOptions/ServerAddressInput.text
	var player_name = $ClientOptions/PlayerNameInput.text
	
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
