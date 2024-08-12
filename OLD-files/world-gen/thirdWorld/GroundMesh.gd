extends MeshInstance3D

# Exported variables to assign the section center nodes

@onready var section_1 = $"../Section1"
@onready var section_2 = $"../Section2"
@onready var section_3 = $"../Section3"

var material

func _ready():
	# Get the material
	material = get_active_material(0)
	
	# Ensure the material is a ShaderMaterial
	if material and material is ShaderMaterial:
		# Set initial positions
		_update_section_centers()

		# Connect to the node's transform change signals if they move dynamically
		if section_1:
			section_1.connect("transform_changed", Callable(self, "_update_section_centers"))
		
		if section_2:
			section_2.connect("transform_changed", Callable(self, "_update_section_centers"))

		if section_3:
			section_3.connect("transform_changed", Callable(self, "_update_section_centers"))

func _update_section_centers():
	if section_center_1_path:
		var section_center_1 = get_node(section_center_1_path)
		material.set_shader_param("section_center_1", section_center_1.global_transform.origin)

	if section_center_2_path:
		var section_center_2 = get_node(section_center_2_path)
		material.set_shader_param("section_center_2", section_center_2.global_transform.origin)

	if section_center_3_path:
		var section_center_3 = get_node(section_center_3_path)
		material.set_shader_param("section_center_3", section_center_3.global_transform.origin)
