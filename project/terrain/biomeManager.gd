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
		# For Section 1, you may have a predefined color (optional)
		# "color": Color(0.8, 0.8, 0.8, 1.0),
		"biomes": {"corral": 0.5, "sand": 0.5}
	},
	"section2": {
		"biomes": {"rock": 0.5, "kelp": 0.5}
	},
	"section3": {
		"biomes": {"rock": 0.5, "lavarock": 0.5}
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
		# Boss areas might not blend; return an empty dictionary.
		return {}
	# Use a helper function to determine which section the color belongs to.
	var section = _get_section_from_color(color)
	# Return a copy of the weights (to avoid accidental modifications).
	return biome_sections[section]["biomes"].duplicate()

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
	if color.r > 0.7:
		return "section1"
	elif color.r > 0.4:
		return "section2"
	else:
		return "section3"


func _ready():
	emit_signal("biome_mask_ready")
	print("✅ BiomeManager is fully loaded and ready.")
