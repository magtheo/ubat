extends Node3D

# Place your terrain mesh here
@onready var terrain = $"../clipmap/MeshInstance3D"
@onready var sub_viewport = $"../clipmap/SubViewport"
@onready var camera_3d = $"../clipmap/Camera3D"


# Define a dictionary mapping biome types to objects
var biome_objects = {
	"corral": preload("res://world-gen/secondWorld/sectionOne/corralBiome/coral_biome.tscn"),
	#"sand": preload("res://path_to_desert_object_scene.tscn"),
}


func _ready():
	if not terrain:
		print("Error: Terrain node not found")
		return
	
	# Create a viewport to render the terrain mesh
	var mesh_boundBox = terrain.get_aabb()
	var x_mesh_boundBox = mesh_boundBox.size.x
	var z_mesh_boundBox = mesh_boundBox.size.z
	var viewport_size = Vector2i(x_mesh_boundBox,z_mesh_boundBox) # Creates an empty vector

	#sub_viewport.render_target_v_flip = true
	add_child(sub_viewport)
	
	camera_3d.set_size(float(z_mesh_boundBox))
	camera_3d.position = Vector3(terrain.mesh.get_aabb().size.x / 2, 100, terrain.mesh.get_aabb().size.z / 2)
	camera_3d.look_at(Vector3(terrain.mesh.get_aabb().size.x / 2, 0, terrain.mesh.get_aabb().size.z / 2), Vector3(0, -1, 0))
	sub_viewport.add_child(camera_3d)
	
	# Create a mesh instance with the terrain material
	var mesh_instance = MeshInstance3D.new()
	mesh_instance.mesh = terrain.mesh
	mesh_instance.material_override = terrain.material_override
	sub_viewport.add_child(mesh_instance)
	
	# Capture the biome map
	var biome_map = sub_viewport.get_texture().get_image()
	biome_map.flip_y()
	
	# Clean up
	sub_viewport.queue_free()
	
	# Call function to place objects based on biome data
	place_objects_based_on_biomes()

# Placeholder for the function to place objects based on biome data
func place_objects_based_on_biomes():
	# Implement your logic here
	print("Placing objects based on biome data...")
