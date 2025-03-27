extends Control

# Path to game scene
const GAME_WORLD = "res://project/scenes/GameWorld.tscn"

# References to bridges via BridgeManager
@onready var config_bridge = BridgeManager.config_bridge
@onready var game_bridge = BridgeManager.game_bridge
@onready var network_bridge = BridgeManager.network_bridge

# References to panels
@onready var main_panel = $MainPanel
@onready var standalone_options = $StandaloneOptions
@onready var host_options = $HostOptions
@onready var client_options = $ClientOptions
@onready var loading_overlay = $LoadingOverlay


# Called when the node enters the scene tree
func _ready():
	
	# Verify that all bridges were successfully created
	if !config_bridge or !game_bridge or !network_bridge:
		push_error("Bridge components not available - check BridgeManager initialization")
		return
	
	# Connect bridge signals
	config_bridge.connect("config_updated", _on_config_updated)
	game_bridge.connect("game_state_changed", _on_game_state_changed)
	game_bridge.connect("game_error", _on_game_error)
	
	debug_check_signals()


	# Initialize configuration
	if not config_bridge.create_default_config():
		_show_error("Failed to create default configuration")

	# Make sure only main panel is visible
	_show_panel(main_panel)

# Shows only the specified panel, hides others
func _show_panel(panel):
	main_panel.visible = (panel == main_panel)
	standalone_options.visible = (panel == standalone_options)
	host_options.visible = (panel == host_options)
	client_options.visible = (panel == client_options)
	loading_overlay.visible = (panel == loading_overlay)

# MAIN PANEL BUTTON HANDLERS

func _on_StandaloneButton_pressed():
	# For standalone mode, we can start right away
	debug_button_press("Standalone")
	debug_bridge_state()
	_start_standalone_game()

func _on_HostButton_pressed():
	debug_button_press("Host")
	debug_bridge_state()
	_show_panel(host_options)

func _on_ClientButton_pressed():
	debug_button_press("Client")
	debug_bridge_state()
	_show_panel(client_options)

func _on_QuitButton_pressed():
	debug_button_press("Client")
	get_tree().quit()

# STANDALONE MODE LOGIC

func _start_standalone_game():
	debug_log("Starting standalone game...")

	# Show loading
	_show_panel(loading_overlay)
	debug_log("Showing loading overlay")

	# Configure standalone mode
	debug_log("Setting network mode to Standalone (0)")
	config_bridge.network_mode = 0
	config_bridge.apply_network_mode()

	# Generate random seed if needed
	if config_bridge.world_seed == 0:
		config_bridge.world_seed = randi()
		debug_log("Generated random seed: " + str(config_bridge.world_seed))
		config_bridge.apply_world_seed()

	# Save configuration
	debug_log("Saving configuration...")
	if not config_bridge.save_config():
		debug_error("Failed to save configuration")
		_show_error("Failed to save configuration")
		return

	# Initialize game systems
	debug_log("Initializing game systems...")
	if not game_bridge.initialize(config_bridge.config_path):
		debug_error("Failed to initialize game")
		_show_error("Failed to initialize game")
		return

	# Start the game
	debug_log("Starting game...")
	if not game_bridge.start_game():
		debug_error("Failed to start game")
		_show_error("Failed to start game")
		return

	# Wait a bit for any backend initialization to complete
	debug_log("Waiting for initialization to complete...")
	await get_tree().create_timer(0.5).timeout

	# Change to game scene
	debug_log("Changing to game scene: " + GAME_WORLD)
	get_tree().change_scene_to_file(GAME_WORLD)

# HOST MODE LOGIC

func _on_StartServerButton_pressed():
	# Show loading
	_show_panel(loading_overlay)

	# Get input values
	var server_name = $HostOptions/ServerNameInput.text
	var port = $HostOptions/PortInput.value
	var max_players = $HostOptions/MaxPlayersInput.value

	# Configure host mode
	config_bridge.network_mode = 1  # 1 = Host
	config_bridge.apply_network_mode()

	config_bridge.server_port = port
	config_bridge.apply_server_port()

	config_bridge.max_players = max_players
	config_bridge.apply_max_players()

	# Set server name as a custom value
	config_bridge.set_custom_value("server_name", server_name)

	# Save configuration
	if not config_bridge.save_config():
		_show_error("Failed to save configuration")
		return

	# Initialize network
	if not network_bridge.initialize_network(1, port, ""):
		_show_error("Failed to initialize network")
		return

	# Initialize game systems
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
	get_tree().change_scene_to_file(GAME_WORLD)

# CLIENT MODE LOGIC

func _on_ConnectButton_pressed():
	# Show loading
	_show_panel(loading_overlay)

	# Get input values
	var server_address = $ClientOptions/ServerAddressInput.text
	var player_name = $ClientOptions/PlayerNameInput.text

	# Configure client mode
	config_bridge.network_mode = 2  # 2 = Client
	config_bridge.apply_network_mode()

	config_bridge.server_address = server_address
	config_bridge.apply_server_address()

	# Set player name as a custom value
	config_bridge.set_custom_value("player_name", player_name)

	# Save configuration
	if not config_bridge.save_config():
		_show_error("Failed to save configuration")
		return

	# Initialize network
	if not network_bridge.initialize_network(2, 0, server_address):
		_show_error("Failed to initialize network")
		return

	# Initialize game systems
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
	get_tree().change_scene_to_file(GAME_WORLD)

# BACK BUTTON HANDLERS

func _on_HostOptions_BackButton_pressed():
	_show_panel(main_panel)

func _on_ClientOptions_BackButton_pressed():
	_show_panel(main_panel)

# ERROR HANDLING

func _show_error(message):
	# Return to main panel
	_show_panel(main_panel)

	# Show error dialog
	var dialog = AcceptDialog.new()
	dialog.window_title = "Error"
	dialog.dialog_text = message
	add_child(dialog)
	dialog.popup_centered()

# SIGNAL HANDLERS

func _on_game_error(error_message):
	_show_error(error_message)

func _on_game_state_changed(old_state, new_state, state_name):
	print("Game state changed: ", state_name)

func _on_config_updated(key, value):
	print("Config updated: ", key, " = ", value)


# Debug functions

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
	
	if config_bridge:
		debug_log("  - config_path: " + str(config_bridge.config_path))
		debug_log("  - network_mode: " + str(config_bridge.network_mode))

func debug_check_signals():
	debug_log("Godot Checking signal connections...")

	# Check if signals are connected
	var config_updated_connected = config_bridge.is_connected("config_updated", _on_config_updated)
	var game_state_changed_connected = game_bridge.is_connected("game_state_changed", _on_game_state_changed)
	var game_error_connected = game_bridge.is_connected("game_error", _on_game_error)

	debug_log("- config_updated signal connected: " + str(config_updated_connected))
	debug_log("- game_state_changed signal connected: " + str(game_state_changed_connected))
	debug_log("- game_error signal connected: " + str(game_error_connected))


func _on_standalone_button_down() -> void:
	pass # Replace with function body.


func _on_host_button_down() -> void:
	pass # Replace with function body.


func _on_client_button_down() -> void:
	pass # Replace with function body.
