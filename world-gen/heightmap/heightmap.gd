extends Node

var image : Image = load(ProjectSettings.get_setting("shader_globals/heightmap").value).get_image()
var image2 : NoiseTexture2D = load(ProjectSettings.get_setting("shader_globals/heightmap2").value)
#var amplitude : float = ProjectSettings.get_setting("shader_global/amplitude").value
var amplitude = 4
@onready var sprite_2d : Sprite2D = %Sprite2D # this is the object i want to retreve the texture from 
var noise_image = sprite_2d.texture.noise.get_image(sprite_2d.texture.get_width(), sprite_2d.texture.get_height(), false, true) # then i want to get the image
var size = image.get_width() # then its width

func get_height(x,z):
	return noise_image.get_pixel(fposmod(x,size), fposmod(z,size)).r * amplitude
# finnish collition with clipmap: https://youtu.be/Hgv9iAdazKg?t=394
