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


	## make noise black or white, nothing in between(grayscale)
	var noise_image = noise_texture.get_image()
	print("noise image: ", noise_image)
	for y in range(noise_texture.get_height()):
		for x in range(noise_texture.get_width()):
			var pixel_val = noise_image.get_pixel(x,y)
			
			# TODO: Fix this shit, dumb fuck
			var dist_black = ((pixel_val.r - Color.BLACK.r)**2.0 + (pixel_val.g - Color.BLACK.g)**2.0 + (pixel_val.b - Color.BLACK.b)**2.0)**0.5
			var dist_white = ((pixel_val.r - Color.WHITE.r)**2.0 + (pixel_val.g - Color.WHITE.g)**2.0 + (pixel_val.b - Color.WHITE.b)**2.0)**0.5

			if dist_black < dist_white:  # If closest to black color
				noise_image.set_pixel(x, y, Color.BLACK)
			else:                       # Else closest to white color
				noise_image.set_pixel(x, y, Color.WHITE)
	
# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta):
	pass


