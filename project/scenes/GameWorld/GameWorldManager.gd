# res://project/scripts/GameWorldController.gd
# Script to be attached to the root node of GameWorld.tscn
extends Node3D # Or Node3D if your GameWorld root is 3D

# Assuming BridgeManager is an Autoload providing access to helpers
@onready var game_init_helper = BridgeManager.game_init_helper

var GameSettings = GameWorldLoadSettings

# Optional: Reference to a loading indicator node within GameWorld.tscn
# Adjust the path based on your GameWorld scene structure
#@onready var loading_indicator = $HUD/LoadingIndicator # Example path

func _ready():
	# TODO: Ensure the loading indicator is initially visible or shown now
	#if loading_indicator:
		#loading_indicator.show()
	#else:
		#print("GameWorldController: Loading indicator node not found (optional).")


	# Retrieve settings from the Autoload
	if GameSettings.settings.is_empty():
		push_error("GameWorld loaded without settings! Returning to Main Menu.")
		# Optionally show an error message to the user here
		get_tree().change_scene_to_file("res://project/scenes/MainMenu.tscn") # Adjust path if needed
		return

	var game_options = GameSettings.settings
	GameSettings.settings = {} # Clear settings immediately after retrieving

	print("GameWorldController: Ready. Initializing game with settings: ", game_options)

	# Ensure the GameInitHelper is available
	if not game_init_helper:
		push_error("GameInitHelper not available in GameWorldController! Check BridgeManager.")
		# Handle critical error - maybe show message and quit/return to menu
		_handle_initialization_failure("Core initialization component missing.")
		return

	# Call the appropriate Rust initialization function based on the mode
	var init_result = false
	var mode = game_options.get("network_mode", -1) # Default to -1 (invalid)

	match mode:
		0: # Standalone
			print("GameWorldController: Initializing Standalone Mode...")
			init_result = game_init_helper.init_standalone(game_options)
		1: # Host
			print("GameWorldController: Initializing Host Mode...")
			init_result = game_init_helper.init_host(game_options)
		2: # Client
			print("GameWorldController: Initializing Client Mode...")
			init_result = game_init_helper.init_client(game_options)
		_:
			push_error("GameWorldController: Invalid network_mode found in settings: %s" % mode)
			_handle_initialization_failure("Invalid game mode specified.")
			return # Exit _ready if mode is invalid

	# Handle the result of the initialization
	if init_result:
		print("GameWorldController: Rust systems initialized successfully.")
		# Hide loading indicator
		#if loading_indicator:
			#loading_indicator.hide()
		# Proceed with game world setup, spawning player, etc.
		_start_game_logic()
	else:
		push_error("GameWorldController: Failed to initialize Rust systems for mode %s!" % mode)
		_handle_initialization_failure("Failed to set up the game session.")
		# No need to hide loading indicator if we're showing an error/changing scene

func _start_game_logic():
	# Placeholder: Add your logic here to spawn the player,
	# enable controls, start background processes, etc.
	print("GameWorldController: Starting game logic...")
	# Example: get_node("PlayerSpawner").spawn_player()

func _handle_initialization_failure(error_message : String):
	# Placeholder: Show an error message to the user and potentially
	# return them to the main menu.
	print("GameWorldController: Initialization Failed! Message: ", error_message)
	#if loading_indicator:
		#loading_indicator.set_text("Error: " + error_message + "\nReturning to menu...") # Example
		# Optionally wait a bit before changing scene
		#await get_tree().create_timer(3.0).timeout

	# Clean way to return to main menu
	# Ensure the path is correct
	var menu_scene = "res://project/scenes/MainMenu.tscn"
	var err = get_tree().change_scene_to_file(menu_scene)
	if err != OK:
		push_error("Failed to return to Main Menu scene!")
		# Fallback or quit?
		get_tree().quit()
