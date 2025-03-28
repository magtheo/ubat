# BridgeManager.gd
extends Node
# References to the bridge instances
var config_bridge
var game_bridge
var network_bridge
var event_bridge
var game_init_helper

func _ready():
	print("BridgeManager: Initializing bridges...")
	
	# Try to create each bridge
	if ClassDB.class_exists("ConfigBridge"):
		config_bridge = ConfigBridge.new()
		add_child(config_bridge)
		config_bridge.debug_mode = true
		print("ConfigBridge created, node: ", config_bridge)
	else:
		push_error("ConfigBridge class not found")
	
	if ClassDB.class_exists("GameManagerBridge"):
		game_bridge = GameManagerBridge.new()
		add_child(game_bridge)
		game_bridge.debug_mode = true
		print("GameManagerBridge created, node: ", game_bridge)
	else:
		push_error("GameManagerBridge class not found")
	
	if ClassDB.class_exists("NetworkManagerBridge"):
		network_bridge = NetworkManagerBridge.new()
		add_child(network_bridge)
		network_bridge.debug_mode = true
		print("NetworkManagerBridge created, node: ", network_bridge)
	else:
		push_error("NetworkManagerBridge class not found")
	
	if ClassDB.class_exists("EventBridge"):
		event_bridge = EventBridge.new()
		add_child(event_bridge)
		event_bridge.debug_mode = true
		print("EventBridge created, node: ", event_bridge)
	else:
		push_error("EventBridge class not found")
	
	if ClassDB.class_exists("GameInitHelper"):
		game_init_helper = GameInitHelper.new()
		add_child(game_init_helper)
		game_init_helper.debug_mode = true
		print("GameInitHelper created, node: ", game_init_helper)
	else:
		push_error("GameInitHelper class not found")
	
	# IMPORTANT: Link the bridges together after all are created
	if game_init_helper and config_bridge and game_bridge:
		game_init_helper.set_bridges(config_bridge, game_bridge)
		print("BridgeManager: Connected bridges to GameInitHelper")
	else:
		push_error("BridgeManager: Cannot connect bridges - one or more components missing")
	
	# Optional: Initialize any default configuration
	if config_bridge:
		var config_success = config_bridge.create_default_config()
		if config_success:
			print("BridgeManager: Default configuration created")
		else:
			push_error("BridgeManager: Failed to create default configuration")
