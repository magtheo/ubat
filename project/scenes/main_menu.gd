extends Control

# Path to game scene
const GAME_WORLD = "res://project/scenes/GameWorld.tscn"

# References
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
	# Verify that game_init_helper is available
	if not game_init_helper:
		push_error("GameInitHelper not available - check BridgeManager initialization")
		_show_error("Failed to initialize game components")
		return
	
	# Connect to BridgeManager signal for bridge initialization
	BridgeManager.connect("bridges_initialized", _on_bridges_initialized)
	
	# Initialize UI
	_setup_ui()
	
	# Make sure only main panel is visible at start
	_show_panel(main_panel)
	
	print("Main menu initialized")

# Called when bridges are initialized
func _on_bridges_initialized():
	print("Bridges initialized, main menu ready for game initialization")

# Set up UI controls with default values
func _setup_ui():
	print("Setting up UI with default values")
	
	# Standalone options defaults
	var seed_input = find_node_recursive(standalone_options, "SeedInput")
	if seed_input:
		seed_input.value = randi()
	
	var world_width_input = find_node_recursive(standalone_options, "WorldWidthInput")
	if world_width_input:
		world_width_input.value = 10000
		
	var world_height_input = find_node_recursive(standalone_options, "WorldHeightInput")
	if world_height_input:
		world_height_input.value = 10000
	
	# Host options defaults
	var host_seed_input = find_node_recursive(host_options, "SeedInput") 
	if host_seed_input:
		host_seed_input.value = randi()
	
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
		player_name.text = "Player"
		
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

# STANDALONE MODE HANDLERS
func _on_StartStandaloneButton_pressed():
	print("Start Standalone button pressed")
	
	# Show loading
	_show_panel(loading_overlay)
	
	# Find input elements
	var seed_input = find_node_recursive(standalone_options, "SeedInput")
	var width_input = find_node_recursive(standalone_options, "WorldWidthInput")
	var height_input = find_node_recursive(standalone_options, "WorldHeightInput")
	
	# Check if all required inputs were found
	if not seed_input or not width_input or not height_input:
		print("Required input elements not found in scene")
		_show_error("UI configuration error - please check scene setup")
		return
		
	# Get values from the input elements
	var seed_value = int(seed_input.value)
	var width_value = int(width_input.value)
	var height_value = int(height_input.value)
	
	print("Starting standalone game with options - Seed: %d, Width: %d, Height: %d" % [seed_value, width_value, height_value])
	
	# Create options dictionary
	var options = {
		"world_seed": seed_value if seed_value > 0 else randi(),
		"world_width": width_value,
		"world_height": height_value
	}
	
	# Use the GameInitHelper to handle the initialization
	var init_result = game_init_helper.init_standalone(options)
	if init_result:
		print("Game initialized successfully, waiting before scene transition")
		# Wait longer for terrain initialization to complete
		print("Waiting for terrain initialization to complete...")
		await get_tree().create_timer(2.0).timeout
		
		# Change to game scene
		print("Changing to game scene: " + GAME_WORLD)
		var error = get_tree().change_scene_to_file(GAME_WORLD)
		if error != OK:
			_show_error("Failed to load game scene (error " + str(error) + ")")
	else:
		print("Failed to initialize standalone game")
		_show_error("Failed to initialize game in standalone mode")
		
func _on_RandomSeedStandaloneButton_pressed():
	print("Random Seed button pressed")
	
	var seed_input = find_node_recursive(standalone_options, "SeedInput")
	if seed_input:
		seed_input.value = randi()

# HOST MODE HANDLERS
func _on_StartServerButton_pressed():
	print("Start Server button pressed")
	
	# Show loading
	_show_panel(loading_overlay)

	# Get input values
	var server_name_input = find_node_recursive(host_options, "ServerName")
	var port_input = find_node_recursive(host_options, "PortInput")
	var max_players_input = find_node_recursive(host_options, "MaxPlayersInput")
	var seed_input = find_node_recursive(host_options, "SeedInput")
	
	if not server_name_input or not port_input or not max_players_input or not seed_input:
		print("Required input elements not found in scene")
		_show_error("UI configuration error - please check scene setup")
		return
	
	var server_name = server_name_input.text
	var port = port_input.value
	var max_players = max_players_input.value
	var seed_value = seed_input.value
	
	print("Starting host game with: Server=%s, Port=%d, Players=%d, Seed=%d" % 
		  [server_name, port, max_players, seed_value])
	
	# Create options dictionary
	var options = {
		"world_seed": seed_value if seed_value > 0 else randi(),
		"server_port": port,
		"max_players": max_players,
		"server_name": server_name
	}
	
	# Use the GameInitHelper to handle the initialization
	var init_result = game_init_helper.init_host(options)
	if init_result:
		print("Game initialized successfully, waiting before scene transition")
		# Wait longer for terrain initialization to complete
		print("Waiting for terrain initialization to complete...")
		await get_tree().create_timer(2.0).timeout
		
		# Change to game scene
		print("Changing to game scene: " + GAME_WORLD)
		var error = get_tree().change_scene_to_file(GAME_WORLD)
		if error != OK:
			_show_error("Failed to load game scene (error " + str(error) + ")")
	else:
		print("Failed to initialize host game")
		_show_error("Failed to initialize game in host mode")

func _on_RandomSeedHostButton_pressed():
	var seed_input = find_node_recursive(host_options, "SeedInput")
	if seed_input:
		seed_input.value = randi()

# CLIENT MODE HANDLERS
func _on_ConnectButton_pressed():
	print("Connect button pressed")
	
	# Show loading
	_show_panel(loading_overlay)

	# Get input values
	var server_address_input = find_node_recursive(client_options, "ServerAddress")
	var player_name_input = find_node_recursive(client_options, "PlayerName")
	
	if not server_address_input or not player_name_input:
		print("Required input elements not found in scene")
		_show_error("UI configuration error - please check scene setup")
		return
	
	var server_address = server_address_input.text
	var player_name = player_name_input.text
	
	if server_address.strip_edges().is_empty():
		_show_error("Please enter a server address")
		return
	
	if player_name.strip_edges().is_empty():
		_show_error("Please enter a player name")
		return
	
	print("Connecting to server: %s as %s" % [server_address, player_name])
	
	# Create options dictionary
	var options = {
		"server_address": server_address,
		"player_name": player_name
	}
	
	# Use the GameInitHelper to handle the initialization
	var init_result = game_init_helper.init_client(options)
	if init_result:
		print("Game initialized successfully, waiting before scene transition")
		# Wait longer for terrain initialization to complete
		print("Waiting for terrain initialization to complete...")
		await get_tree().create_timer(2.0).timeout
		
		# Change to game scene
		print("Changing to game scene: " + GAME_WORLD)
		var error = get_tree().change_scene_to_file(GAME_WORLD)
		if error != OK:
			_show_error("Failed to load game scene (error " + str(error) + ")")
	else:
		print("Failed to initialize client game")
		_show_error("Failed to initialize game in client mode")

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
	if not error_dialog:
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
	
	push_error(message)
