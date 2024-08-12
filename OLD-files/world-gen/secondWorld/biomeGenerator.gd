extends Node3D

const sand_biome = preload("res://world-gen/secondWorld/sectionOne/sandBiome/sand_biome.tscn")
const corral_biome = preload("res://world-gen/secondWorld/sectionOne/corralBiome/corral_biome.tscn")


@onready var noise_texture_generator = $"../noiseTextureGenerator"

# Called when the node enters the scene tree for the first time.
func _ready():
	#var noise_texture = noise_texture_generator.get_texture()
	#await noise_texture.changed
	#var noise_image = noise_texture.get_image()
	#
	#analyse_biome(noise_image)
	pass


# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta):
	pass

func analyse_biome(img):
	var total = 0
	for y in range(img.get_height()):
		for x in range(img.get_width()):
		# Get the grayscale value of the pixel (it's a 3-channel color, so we take the first component)
			var rgb = img.get_pixel(x, y)
			if rgb == Color.BLACK:
				print(corral_biome)
				#place_objects(corral_biome, x, y)  # Average of gray scale around 128, so values under 64 are likely to be in this biome
			if rgb == Color.WHITE:
				print(sand_biome)
				#place_objects(sand_biome, x, y)  # Values above 192 are likely to be in this biome

func place_objects(obj, x, y):
	var biome_instance = obj.instantiate()
	print(obj," x:",x, " y: ",y)
	add_child(biome_instance)  # Add this node to your current scene
	biome_instance.global_position = Vector3(x, 10.0, y)
