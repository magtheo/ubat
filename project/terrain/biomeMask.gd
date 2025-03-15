extends Node

# 🖼️ Biome Mask Texture
var biome_image: Image = null
var mask_width: int = 0
var mask_height: int = 0

# 🌎 World Size (Determined from the mask)
var world_width: float = 10000.0
var world_height: float = 10000.0

var is_ready: bool = false

# ⚙️ Performance Cache
var color_cache := {}
var colorMutex = Mutex.new()

signal biome_mask_ready

# TODO: how is this image actualy placed in the world and on the and used to define the sections, is it infinate??
# 🗺️ Biome mask image path
const BIOME_MASK_IMAGE = "res://textures/biomeMask_image.png"

# 🚀 Initialize
func _ready():
	if load_mask(BIOME_MASK_IMAGE):
		is_ready = true
		emit_signal("biome_mask_ready")
		print("✅ BiomeMask is fully loaded and ready.")
	else:
		push_error("❌ BiomeMask failed to load.")

# 📂 Load Biome Mask
func load_mask(path: String) -> bool:
	var img_texture = load(path)
	print("image texture: ", img_texture)


	if img_texture:
		biome_image = img_texture.get_image()
		mask_width = biome_image.get_width()
		mask_height = biome_image.get_height()
		print("Biome image dimensions: ", mask_width, "x", mask_height)
		print("Biome image format: ", biome_image.get_format())
		return true
	else:
		push_error("Failed to load biome mask at: " + path)
		return false

# 🌎 Map World Coordinates to Biome Mask Coordinates
func world_to_mask_coords(world_x: float, world_y: float) -> Vector2i:
	var mask_x = int((world_x / world_width) * mask_width)
	var mask_y = int((world_y / world_height) * mask_height)
	return Vector2i(clamp(mask_x, 0, mask_width - 1), clamp(mask_y, 0, mask_height - 1))

# 🎨 Get the Biome Color from the Mask
func get_biome_color(world_x: float, world_y: float) -> Color:
	var coords = world_to_mask_coords(world_x, world_y)
	var key = str(coords.x) + "_" + str(coords.y)

	# 🚀 Use Cache for Performance
	colorMutex.lock()
	if key in color_cache:
		var cached_color = color_cache[key]
		colorMutex.unlock()
		return cached_color

	var color = biome_image.get_pixel(coords.x, coords.y)
	color_cache[key] = color
	colorMutex.unlock()
	return color

# 📏 Get World Boundaries
func get_world_bounds() -> Rect2:
	return Rect2(0, 0, world_width, world_height)

# 🧹 Clear Cache (useful if the mask is updated)
func clear_cache():
	color_cache.clear()
