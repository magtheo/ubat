# res://project/scenes/MainMenu.gd (Modified)
extends Control

# Path to game scene
const GAME_WORLD = "res://project/scenes/GameWorld/GameWorld.tscn"

var GameSettings = GameWorldLoadSettings

# References (Keep if BridgeManager is needed for other things, otherwise remove)
# @onready var game_init_helper = BridgeManager.game_init_helper

# References to panels
@onready var main_panel = $MainPanel
@onready var standalone_options = $StandaloneOptions
@onready var host_options = $HostOptions
@onready var client_options = $ClientOptions
@onready var loading_overlay = $LoadingOverlay # Keep for UI feedback if desired
@onready var error_dialog = $ErrorDialog

# Called when the node enters the scene tree
func _ready():
	# Remove check for game_init_helper if it's no longer needed here
	# if not game_init_helper:
	#     push_error("GameInitHelper not available - check BridgeManager initialization")
	#     _show_error("Failed to initialize game components")
	#     return

	# Connect to BridgeManager signal for bridge initialization (Keep if needed)
	# BridgeManager.connect("bridges_initialized", _on_bridges_initialized)

	# Initialize UI
	_setup_ui()

	# Make sure only main panel is visible at start
	_show_panel(main_panel)

	print("Main menu initialized")

# Called when bridges are initialized (Keep if needed)
# func _on_bridges_initialized():
#    print("Bridges initialized, main menu ready for game initialization")

# Set up UI controls with default values
func _setup_ui():
	print("Setting up UI with default values")

	# Standalone options defaults
	var seed_input = find_node_recursive(standalone_options, "SeedInput")
	if seed_input:
		seed_input.value = randi() % 1000000 # Use modulo for potentially large randi results

	var world_width_input = find_node_recursive(standalone_options, "WorldWidthInput")
	if world_width_input:
		world_width_input.value = 10000

	var world_height_input = find_node_recursive(standalone_options, "WorldHeightInput")
	if world_height_input:
		world_height_input.value = 10000

	# Host options defaults
	var host_seed_input = find_node_recursive(host_options, "SeedInput")
	if host_seed_input:
		host_seed_input.value = randi() % 1000000

	var port_input = find_node_recursive(host_options, "PortInput")
	if port_input:
		port_input.value = 7878

	var max_players_input = find_node_recursive(host_options, "MaxPlayersInput")
	if max_players_input:
		max_players_input.value = 64

	# Client options defaults
	var server_address = find_node_recursive(client_options, "ServerAddress")
	if server_address:
		server_address.text = "127.0.0.1:7878"

	var player_name = find_node_recursive(client_options, "PlayerName")
	if player_name:
		player_name.text = "Player" + str(randi() % 1000) # Make default player name unique

# Helper function to find nodes recursively by name
func find_node_recursive(parent, node_name):
	for child in parent.get_children():
		if child.name == node_name:
			return child
		if child.get_child_count() > 0:
			var found = find_node_recursive(child, node_name)
			if found:
				return found
	return null

# Shows only the specified panel, hides others
func _show_panel(panel):
	main_panel.visible = (panel == main_panel)
	standalone_options.visible = (panel == standalone_options)
	host_options.visible = (panel == host_options)
	client_options.visible = (panel == client_options)
	# Only show loading overlay briefly before scene change if desired
	loading_overlay.visible = (panel == loading_overlay)

# --- Button Handlers ---

func _on_StandaloneButton_pressed():
	print("Standalone button pressed")
	_show_panel(standalone_options)

func _on_HostButton_pressed():
	print("Host button pressed")
	_show_panel(host_options)

func _on_ClientButton_pressed():
	print("Client button pressed")
	_show_panel(client_options)

func _on_QuitButton_pressed():
	print("Quit button pressed")
	get_tree().quit()

# --- Standalone Mode ---

func _on_StartStandaloneButton_pressed():
	print("Start Standalone button pressed")

	var seed_input = find_node_recursive(standalone_options, "SeedInput")
	var width_input = find_node_recursive(standalone_options, "WorldWidthInput")
	var height_input = find_node_recursive(standalone_options, "WorldHeightInput")

	if not seed_input or not width_input or not height_input:
		_show_error("UI configuration error - Standalone inputs missing.")
		return

	var seed_value = int(seed_input.value)
	var width_value = int(width_input.value)
	var height_value = int(height_input.value)

	print("Gathering standalone options - Seed: %d, Width: %d, Height: %d" % [seed_value, width_value, height_value])

	# Store settings in the Autoload
	GameSettings.settings = {
		"network_mode": 0, # 0 = Standalone
		"world_seed": seed_value if seed_value > 0 else randi() % 1000000,
		"world_width": width_value,
		"world_height": height_value
	}

	# Optionally show loading overlay briefly
	_show_panel(loading_overlay)
	# Use call_deferred to ensure UI updates before potential scene change hiccup
	call_deferred("_change_to_game_scene")


func _on_RandomSeedStandaloneButton_pressed():
	print("Random Seed button pressed")
	var seed_input = find_node_recursive(standalone_options, "SeedInput")
	if seed_input:
		seed_input.value = randi() % 1000000

# --- Host Mode ---

func _on_StartServerButton_pressed():
	print("Start Server button pressed")

	var server_name_input = find_node_recursive(host_options, "ServerName")
	var port_input = find_node_recursive(host_options, "PortInput")
	var max_players_input = find_node_recursive(host_options, "MaxPlayersInput")
	var seed_input = find_node_recursive(host_options, "SeedInput")

	if not server_name_input or not port_input or not max_players_input or not seed_input:
		_show_error("UI configuration error - Host inputs missing.")
		return

	var server_name = server_name_input.text
	var port = int(port_input.value)
	var max_players = int(max_players_input.value)
	var seed_value = int(seed_input.value)

	print("Gathering host options: Server=%s, Port=%d, Players=%d, Seed=%d" %
		  [server_name, port, max_players, seed_value])

	# Store settings in the Autoload
	GameSettings.settings = {
		"network_mode": 1, # 1 = Host
		"world_seed": seed_value if seed_value > 0 else randi() % 1000000,
		"server_port": port,
		"max_players": max_players,
		"server_name": server_name
		# Add world width/height if host mode needs them too
		#"world_width": default_width_if_needed,
		#"world_height": default_height_if_needed
	}

	_show_panel(loading_overlay)
	call_deferred("_change_to_game_scene")


func _on_RandomSeedHostButton_pressed():
	var seed_input = find_node_recursive(host_options, "SeedInput")
	if seed_input:
		seed_input.value = randi() % 1000000

# --- Client Mode ---

func _on_ConnectButton_pressed():
	print("Connect button pressed")

	var server_address_input = find_node_recursive(client_options, "ServerAddress")
	var player_name_input = find_node_recursive(client_options, "PlayerName")

	if not server_address_input or not player_name_input:
		_show_error("UI configuration error - Client inputs missing.")
		return

	var server_address = server_address_input.text
	var player_name = player_name_input.text

	if server_address.strip_edges().is_empty():
		_show_error("Please enter a server address")
		return

	if player_name.strip_edges().is_empty():
		_show_error("Please enter a player name")
		return

	print("Gathering client options: Server=%s, Player=%s" % [server_address, player_name])

	# Store settings in the Autoload
	GameSettings.settings = {
		"network_mode": 2, # 2 = Client
		"server_address": server_address,
		"player_name": player_name
	}

	_show_panel(loading_overlay)
	call_deferred("_change_to_game_scene")


# --- Back Buttons ---

func _on_StandaloneOptions_BackButton_pressed():
	_show_panel(main_panel)

func _on_HostOptions_BackButton_pressed():
	_show_panel(main_panel)

func _on_ClientOptions_BackButton_pressed():
	_show_panel(main_panel)

# --- Scene Change ---

func _change_to_game_scene():
	print("Settings stored, changing scene to: " + GAME_WORLD)
	var error = get_tree().change_scene_to_file(GAME_WORLD)
	if error != OK:
		# Hide loading overlay on error
		_show_panel(main_panel)
		_show_error("Failed to load game scene (error " + str(error) + ")")
		# Clear stored settings on failure?
		GameSettings.settings = {}


# --- Error Handling ---

func _show_error(message):
	# Ensure error dialog exists (good practice)
	if not error_dialog:
		var dialog = AcceptDialog.new()
		dialog.name = "ErrorDialog"
		dialog.title = "Error"
		add_child(dialog)
		error_dialog = dialog # Assign if newly created

	if not is_instance_valid(error_dialog):
		push_error("Error Dialog node is invalid!")
		return

	# Ensure we return to the main panel view
	_show_panel(main_panel)

	error_dialog.dialog_text = message
	error_dialog.popup_centered()

	push_error(message) # Log the error as well
