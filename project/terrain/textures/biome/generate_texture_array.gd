@tool # Allows this script to run in the Godot Editor
extends Node
# scripts/generate_texture_array.gd

#extends EditorScript # Or Node, if you prefer attaching to a scene node to run

# --- This script is used to make a texture2DArray from textures.
# - usage: add to a node, and run game once, file is then generated, and can be used at every game launch
# - the exported file is used in the ground mesh material

func _ready():
	# --- Configuration: MODIFY THESE ---
	var texture_paths = [
		"res://project/terrain/textures/biome/lavaRockFloor.png",
		"res://project/terrain/textures/biome/coralFloor.png",             # Layer 1 / Biome ID 1
		"res://project/terrain/textures/biome/sandFloor.png",              # Layer 2 / Biome ID 2
		"res://project/terrain/textures/biome/rockFloor.png",              # Layer 3 / Biome ID 3
		"res://project/terrain/textures/biome/kelpFloor.png",              # Layer 4 / Biome ID 4
		"res://project/terrain/textures/biome/lavaRockFloor.png"           # Layer 5 / Biome ID 5
		# Add paths for all your biome textures IN ORDER of Biome ID
	]
	var output_path = "project/terrain/textures/biome/biome_texture_array.tres"
	# --- End Configuration ---

	print("Generating Texture2DArray...")

	var layers = []
	var first_texture_size = Vector2i.ZERO
	#var first_texture_mipmaps = false

	# Load all textures and check consistency
	for i in range(texture_paths.size()):
		var path = texture_paths[i]
		var texture = load(path) as Texture2D
		if !texture:
			printerr("Failed to load texture at path: ", path)
			return # Stop generation if any texture fails

		if i == 0:
			first_texture_size = texture.get_size()
			# first_texture_mipmaps = texture.has_mipmaps() # <--- COMMENT OUT or DELETE this line
			print("  Base Texture Size: ", first_texture_size)
			# print("  Base Texture Mipmaps: ", first_texture_mipmaps) # <--- COMMENT OUT or DELETE this line
		else:
			if texture.get_size() != first_texture_size:
				printerr("Texture size mismatch! Path: ", path, " Size: ", texture.get_size(), " Expected: ", first_texture_size)
				return
			# Note: Godot 4 automatically handles mipmap consistency in Texture2DArray
			# if texture.has_mipmaps() != first_texture_mipmaps: # <--- COMMENT OUT or DELETE this check block
			# 	printerr("Texture mipmap mismatch! Path: ", path)
			# 	return


		# We need the image data to create the array
		var image = texture.get_image()
		if !image:
			printerr("Failed to get image data from texture: ", path)
			return
		layers.append(image)
		print("  Loaded texture for layer ", i, ": ", path)

	if layers.is_empty():
		printerr("No textures loaded successfully.")
		return

	# Create the Texture2DArray from the image data
	var texture_array = Texture2DArray.new()
	# _create() is deprecated in Godot 4. Use create_from_images instead.
	var error = texture_array.create_from_images(layers)

	if error != OK:
		printerr("Failed to create Texture2DArray from images. Error code: ", error)
		return

	print("Texture2DArray created successfully with ", layers.size(), " layers.")

	# Save the resource
	error = ResourceSaver.save(texture_array, output_path)
	if error != OK:
		printerr("Failed to save Texture2DArray resource to: ", output_path, " Error code: ", error)
	else:
		print("Successfully saved Texture2DArray to: ", output_path)
