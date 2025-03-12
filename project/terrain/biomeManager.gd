# BiomeManager.gd
extends Node

signal biome_manager_ready
# -----------------------------------------------------------------------------
# This dictionary defines biome sections.
# Each entry maps a section key to a dictionary that includes:
#   - an optional "color" field (if you want to match a unique color)
#   - a "biomes" dictionary that holds the names of the biomes to blend and their weights.
# -----------------------------------------------------------------------------
var biome_sections = {
	"section1": {
		"color": Color(0.8, 0.8, 0.8, 1.0),
		"biomes": ["corral", "sand"]
	},
	"section2": {
		"color": Color(0.8, 0.5, 0.5, 1.0),
		"biomes": ["rock", "kelp"]
	},
	"section3": {
		"color": Color(0.5, 0.5, 0.5, 1.0),
		"biomes": ["rock", "lavarock"]
	},
	"section_boss": {
		"color": Color(1.0, 0.0, 0.0, 1.0),  # Boss area is pure red
		"biomes": []  # No blending for boss area
	}
}

# The designated color for the boss area.
var boss_area_color: Color = Color(1, 0, 0, 1)

# -----------------------------------------------------------------------------
# Returns the biome name based on a given color.
# If the color represents a boss area, "Boss" is returned.
# Otherwise, if the color uniquely corresponds to a biome (or one of the blended types),
# this function can be expanded to choose which one to return.
# -----------------------------------------------------------------------------
func get_biome_name(color: Color) -> String:
	if is_boss_area(color):
		return "Boss"
	# For now, we simply determine the section based on an example heuristic.
	var section = _get_section_from_color(color)
	# As a simple approach, return the first biome name in the section's blend.
	for biome_name in biome_sections[section]["biomes"]:
		return biome_name
	return "Unknown"

# -----------------------------------------------------------------------------
# Returns a dictionary containing blending weights for the given color.
# If the color represents a blended area between two biomes, their weights are returned.
# -----------------------------------------------------------------------------
func get_biome_weights(color: Color) -> Dictionary:
	if is_boss_area(color):
		# Boss areas don't blend
		return {}
	
	var section = _get_section_from_color(color)
	if section.is_empty() or not biome_sections.has(section):
		return {}
	
	var weights = {}
	var biome_list = biome_sections[section]["biomes"]
	
	# Return weights of 0.5 for both biomes - actual blending will be done
	# using noise in the shader and height calculation
	if biome_list.size() >= 2:
		weights[biome_list[0]] = 0.5
		weights[biome_list[1]] = 0.5
	elif biome_list.size() == 1:
		weights[biome_list[0]] = 1.0
	
	return weights

# -----------------------------------------------------------------------------
# Returns true if the given color corresponds to the boss area.
# -----------------------------------------------------------------------------
func is_boss_area(color: Color) -> bool:
	return color == boss_area_color

# -----------------------------------------------------------------------------
# Helper function that determines the biome section based on the color.
# In a production system you would likely use a lookup table or match specific color ranges.
#
# For this example, we use a simple heuristic based on the red channel.
# Adjust the logic to match your biome mask image design.
# -----------------------------------------------------------------------------

func _get_section_from_color(color: Color) -> String:
	var closest_section = ""
	var closest_distance = 1.0  # Max possible distance
	var match_threshold = 0.15  # Threshold for matching colors
	
	for section_key in biome_sections:
		if section_key == "section_boss":
			continue  # Skip boss section, handled separately in is_boss_area
		
		if biome_sections[section_key].has("color"):
			var section_color = biome_sections[section_key]["color"]
			# Calculate color distance using RGB components
			var distance = sqrt(
				pow(color.r - section_color.r, 2) +
				pow(color.g - section_color.g, 2) +
				pow(color.b - section_color.b, 2)
			)
			
			if distance < match_threshold and distance < closest_distance:
				closest_distance = distance
				closest_section = section_key
	
	# Fallback logic if no section color was close enough
	if closest_section.is_empty():
		# Default fallback based on the red channel
		if color.r > 0.7:
			return "section2"
		else:
			return "section3"
	
	return closest_section
	
func _ready():
	emit_signal("biome_manager_ready")
	print("✅ BiomeManager is fully loaded and ready.")
