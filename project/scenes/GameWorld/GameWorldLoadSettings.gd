
# Autoload script to pass settings between MainMenu and GameWorld.
extends Node

# Dictionary to hold the settings for the next game session.
# Example keys: "network_mode", "world_seed", "world_width", "world_height",
#               "server_port", "max_players", "server_name",
#               "server_address", "player_name"
var settings: Dictionary = {}

# Optional: Add a function to clear settings if needed elsewhere
func clear():
	settings = {}
