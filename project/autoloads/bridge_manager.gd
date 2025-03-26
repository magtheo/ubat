# BridgeManager.gd
extends Node

# References to the bridge instances
var config_bridge
var game_bridge
var network_bridge
var event_bridge

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
