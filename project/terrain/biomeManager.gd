extends Node
class_name BiomeManager

# 🗺️ Reference to BiomeMask
var biome_mask: BiomeMask

# 🌈 Section-to-Color Mapping
var section_colors := {}

# 🧩 Section-to-Biome Mapping
var sections := {}

# 🎨 Default Colors for Sections
const SECTION_1_COLOR = Color(0, 1, 0)  # Green
const SECTION_2_COLOR = Color(0, 0, 1)  # Blue
const SECTION_3_COLOR = Color(1, 0, 0)  # Red

# 🚀 Initialize with BiomeMask
func initialize(biome_mask_instance: BiomeMask):
	biome_mask = biome_mask_instance
	_setup_sections()

# 🗺️ Define Sections and Their Biomes
func _setup_sections():
	section_colors = {
		SECTION_1_COLOR: "Section 1",
		SECTION_2_COLOR: "Section 2",
		SECTION_3_COLOR: "Section 3"
	}

	sections = {
		"Section 1": ["corral", "sand"],
		"Section 2": ["rock", "kelp"],
		"Section 3": ["rock", "lavarock"]
	}

# 🧩 Get Section from Position
func get_section(world_x: float, world_y: float) -> String:
	var color = biome_mask.get_biome_color(world_x, world_y)
	return get_section_from_color(color)

# 🎨 Find Section by Mask Color
func get_section_from_color(color: Color) -> String:
	for sec_color in section_colors.keys():
		if _is_color_match(color, sec_color):
			return section_colors[sec_color]
	return "Unknown"

# 🧩 Return Biomes for a Section
func get_biomes_for_section(section_name: String) -> Array:
	if sections.has(section_name):
		return sections[section_name]
	return []

# 🌈 Return Biomes and Blend Weights
func get_biome_blend(world_x: float, world_y: float) -> Dictionary:
	var section_name = get_section(world_x, world_y)
	var biomes = get_biomes_for_section(section_name)

	if biomes.size() == 2:
		var blend_value = _get_blend_value(world_x, world_y)
		return {
			biomes[0]: 1.0 - blend_value,
			biomes[1]: blend_value
		}
	return {}

# 🎨 Compare Colors with a Tolerance
func _is_color_match(c1: Color, c2: Color, tolerance := 0.05) -> bool:
	return (
		abs(c1.r - c2.r) < tolerance and
		abs(c1.g - c2.g) < tolerance and
		abs(c1.b - c2.b) < tolerance
	)

# 🌈 Get Blend Value (Optional Section Noise)
func _get_blend_value(world_x: float, world_y: float) -> float:
	var blend_noise = noise_wrapper.get_noise_2d("blend", world_x, world_y)
	return clamp((blend_noise + 1.0) / 2.0, 0.0, 1.0)
