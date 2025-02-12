# NetworkManager.gd
extends Node
class_name NetworkManager
var random = RandomNumberGenerator.new()



signal noise_seed_received(seed)

func _ready() -> void:
	# In production, you would set up your network connection here.
	# For demonstration, we simulate receiving a noise seed after a 1.0-second delay.
	await get_tree().create_timer(1.0).timeout
	var seed: int = random.randi_range(0, 10000)
	emit_signal("noise_seed_received", seed)
	print("NetworkManager: Noise seed received from server: ", seed)
