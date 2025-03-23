extends Control

# Path to game scene
const GAME_WORLD = "res://project/scenes/GameWorld.tscn"

# References to panels
@onready var main_panel = $MainPanel
@onready var standalone_options = $StandaloneOptions
@onready var host_options = $HostOptions
@onready var client_options = $ClientOptions
@onready var loading_overlay = $LoadingOverlay

# References to bridges
@onready var config_bridge = get_node("/root/Main/ConfigBridge")
@onready var game_bridge = get_node("/root/Main/GameManagerBridge")
@onready var network_bridge = get_node("/root/Main/NetworkManagerBridge")

# Called when the node enters the scene tree
func _ready():
	# Connect bridge signals
	config_bridge.connect("config_updated", _on_config_updated)
	game_bridge.connect("game_state_changed", _on_game_state_changed)
	game_bridge.connect("game_error", _on_game_error)

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
	_start_standalone_game()

func _on_HostButton_pressed():
	_show_panel(host_options)

func _on_ClientButton_pressed():
	_show_panel(client_options)

func _on_QuitButton_pressed():
	get_tree().quit()

# STANDALONE MODE LOGIC

func _start_standalone_game():
	# Show loading
	_show_panel(loading_overlay)

	# Configure standalone mode
	config_bridge.network_mode = 0  # 0 = Standalone
	config_bridge.apply_network_mode()

	# Generate random seed if needed
	if config_bridge.world_seed == 0:
		config_bridge.world_seed = randi()
		config_bridge.apply_world_seed()

	# Save configuration
	if not config_bridge.save_config():
		_show_error("Failed to save configuration")
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
