# NetworkManager.gd
extends Node
class_name NetworkManager
var random = RandomNumberGenerator.new()



signal noise_seeds_received(noise_seeds: Dictionary)

func _ready() -> void:
	# Simulate receiving multiple seeds after a delay
	await get_tree().create_timer(1.0).timeout
	var seeds = {
		"global": 12345,
		"corral": 23456,
		"sand": 34567,
		"rock": 45678,
		"kelp": 56789,
		"blending": 67890
	}
	#emit_signal("noise_seeds_received", seeds)
	#print("NetworkManager: Noise seeds received from server:", seeds)
