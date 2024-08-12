extends Node

#var image : Image = load(ProjectSettings.get_setting("shader_globals/heightmap").value).get_image()
#var image2 : NoiseTexture2D = load(ProjectSettings.get_setting("shader_globals/heightmap2").value)
#var amplitude : float = ProjectSettings.get_setting("shader_global/amplitude").value
var amplitude = 4

var noise_image
var size
var sprite
var current_scene = null
var noise_texture
var height_rect


func _ready():
	var root = get_tree().root
	current_scene = root.get_node("/root/main")
	
	if current_scene:
		height_rect = current_scene.get_node("World/Terrain/HeightRect")
		if height_rect != null:
			print("noise node found!")
			#print("1: ",height_rect)
			noise_texture = height_rect.get_texture()
			#print("2: ",noise_texture)
			size = noise_texture.get_width()
			#print(size)
		else:
			print("noise node NOT found")
	else:
		print("world node not found")

func get_height(x,z):
	#print("noise texture:", noise_texture)
	await noise_texture.changed
	noise_image = noise_texture.get_image()
	#print("noise image", noise_image)
	return noise_image.get_pixel(fposmod(x,size), fposmod(z,size)).r * amplitude
	#return noise_texture.get_pixel(fposmod(x,size), fposmod(z,size)).r * amplitude
