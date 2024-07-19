extends TextureRect

func _ready():
	generate_noise()

func generate_noise():
	# randomize the seed
	var rng = RandomNumberGenerator.new()
	var noise_texture = self.get_texture()
	print(noise_texture)
	noise_texture.noise.seed = rng.randi()
	await noise_texture.changed
