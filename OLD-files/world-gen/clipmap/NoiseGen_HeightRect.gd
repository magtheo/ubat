extends TextureRect


# Called when the node enters the scene tree for the first time.
func _ready():
	var rng = RandomNumberGenerator.new()
	var noise_texture = self.get_texture()
	print(noise_texture)
	noise_texture.noise.seed = rng.randi()


# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta):
	pass
