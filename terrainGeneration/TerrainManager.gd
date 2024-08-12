# TerrainManager.gd

extends Node3D

var chunk_manager: ChunkManager
var biome_manager: BiomeManager

func _ready():
	biome_manager = BiomeManager.new()
	chunk_manager = ChunkManager.new(biome_manager)
	add_child(chunk_manager)

func _process(delta):
	var player_position = get_player_position()
	chunk_manager.update_chunks(player_position)

func get_player_position() -> Vector3:
	# Implement this to return the player's current position
	# You'll need to reference your existing player script here
	pass
