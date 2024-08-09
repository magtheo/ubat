# chunk.gd
extends Node3D
class_name Chunk

var mesh_instance
var shader_material
var x
var z
var chunk_size
var should_remove
var loaded

var areas: Array[Area3D]  # Areas representing different sections

var biomeNoise
var heightChanger
var heightmapSand
var heightmapCoral
var heightmapRock
var heightmapKelp
var heightmapLavaStone

var generalAmplitude
var sandAmplitude
var coralAmplitude
var rockAmplitude
var kelpAmplitude
var lavaStoneAmplitude
var biomeStrengthAmplifyer
var height_difference_amp

var sandCutof
var coralCutof
var rockCutof
var kelpCutof
var lavaStoneCutof

var textureSand
var textureCoral
var textureRock
var textureKelp
var textureLavaStone

func _init(
	shader, x_pos, z_pos, chunk_size,
	areas,  # Pass the areas to the chunk
	loaded
):
	self.shader_material = shader
	self.x = x_pos
	self.z = z_pos
	self.chunk_size = chunk_size
	self.areas = areas
	self.loaded = loaded
	
func _ready():
	generate_chunk()

func generate_chunk():
	print("loading chunk: ", " x:", x, " z:", z)  # Debugging statement

	# Create the plane mesh
	var plane_mesh = PlaneMesh.new()
	plane_mesh.size = Vector2(chunk_size, chunk_size)
	plane_mesh.subdivide_depth = chunk_size * 0.2
	plane_mesh.subdivide_width = chunk_size * 0.2

	var surface_tool = SurfaceTool.new()
	var data_tool = MeshDataTool.new()
	surface_tool.create_from(plane_mesh, 0)
	var array_plane = surface_tool.commit()
	var error = data_tool.create_from_surface(array_plane, 0)
	
	# Modify vertex heights and texture coordinates
	for i in range(data_tool.get_vertex_count()):
		var vertex = data_tool.get_vertex(i)
		var section = determine_section(Vector3(vertex.x, vertex.z, vertex.y))
		var texture_position = Vector2(vertex.x + x, vertex.z + z) * 0.5 / float(biomeNoise.get_image().get_width())
		var height = get_vertex_height(texture_position, section)
		var color = get_texture_color(texture_position, section)
		
		vertex.y = height
		data_tool.set_vertex(i, vertex)
		data_tool.set_vertex_color(i, color)

	# Create the final mesh instance
	mesh_instance = MeshInstance3D.new()
	mesh_instance.mesh = surface_tool.commit()
	mesh_instance.create_trimesh_collision()
	mesh_instance.mesh.surface_set_material(0, shader_material)
	mesh_instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF

	add_child(mesh_instance)
	print("Chunk loaded with global position: ", global_position)  # Debugging statement

func determine_section(point: Vector3) -> String:
	var space_state = get_world_3d().direct_space_state

	for area in areas:
		var shape = area.get_node("CollisionShape3D")  # Replace with your actual CollisionShape3D node path
		if shape:
			var shape_transform = area.global_transform
			var shape_shape = shape.shape

			# Prepare the query parameters
			var params = PhysicsShapeQueryParameters3D.new()
			params.shape = shape_shape
			params.transform = shape_transform
			params.position = point
			params.exclude = []  # You can exclude certain objects if needed
			
			# Perform the shape intersection check
			var results = space_state.intersect_shape(params, 1)

			# Check if the point is inside the shape
			if results.size() > 0:
				return area.name  # or any other identifier for the section

	return "Unknown"

func get_texture_color(texture_position, section):
	var biomeNoise_image = biomeNoise.get_image()
	
	var biomeStrength = biomeNoise_image.get_pixelv(texture_position).r * biomeStrengthAmplifyer

	var sandWeight = 0.0
	var coralWeight = 0.0
	var rockWeight = 0.0
	var kelpWeight = 0.0
	var lavaStoneWeight = 0.0

	match section:
		"Section1":
			sandWeight = smooth_step(sandCutof - 0.1, sandCutof + 0.1, biomeStrength)
			coralWeight = smooth_step(coralCutof - 0.1, coralCutof + 0.1, biomeStrength)
		"Section2":
			rockWeight = smooth_step(rockCutof - 0.1, rockCutof + 0.1, biomeStrength)
			kelpWeight = smooth_step(kelpCutof - 0.1, kelpCutof + 0.1, biomeStrength)
		"Section3":
			rockWeight = smooth_step(rockCutof - 0.1, rockCutof + 0.1, biomeStrength)
			lavaStoneWeight = smooth_step(lavaStoneCutof - 0.1, lavaStoneCutof + 0.1, biomeStrength)

	var transitionWeight = 1.0 - sandWeight - coralWeight - rockWeight - kelpWeight - lavaStoneWeight

	var sandTexture = textureSand.get_image().get_pixelv(texture_position)
	var coralTexture = textureCoral.get_image().get_pixelv(texture_position)
	var rockTexture = textureRock.get_image().get_pixelv(texture_position)
	var kelpTexture = textureKelp.get_image().get_pixelv(texture_position)
	var lavaStoneTexture = textureLavaStone.get_image().get_pixelv(texture_position)

	return (
		sandTexture * sandWeight +
		coralTexture * coralWeight +
		rockTexture * rockWeight +
		kelpTexture * kelpWeight +
		lavaStoneTexture * lavaStoneWeight +
		sandTexture * transitionWeight
	)

func get_vertex_height(texture_position, section):
	var biomeStrength_image = biomeNoise.get_image()
	var heightAmp_image = heightChanger.get_image()
	
	var biomeStrength = biomeStrength_image.get_pixelv(texture_position).r * biomeStrengthAmplifyer
	var height_amplefier = heightAmp_image.get_pixelv(texture_position).r

	var sandWeight = 0.0
	var coralWeight = 0.0
	var rockWeight = 0.0
	var kelpWeight = 0.0
	var lavaStoneWeight = 0.0

	match section:
		"Section1":
			sandWeight = smooth_step(sandCutof - 0.1, sandCutof + 0.1, biomeStrength)
			coralWeight = smooth_step(coralCutof - 0.1, coralCutof + 0.1, biomeStrength)
		"Section2":
			rockWeight = smooth_step(rockCutof - 0.1, rockCutof + 0.1, biomeStrength)
			kelpWeight = smooth_step(kelpCutof - 0.1, kelpCutof + 0.1, biomeStrength)
		"Section3":
			rockWeight = smooth_step(rockCutof - 0.1, rockCutof + 0.1, biomeStrength)
			lavaStoneWeight = smooth_step(lavaStoneCutof - 0.1, lavaStoneCutof + 0.1, biomeStrength)

	var transitionWeight = 1.0 - sandWeight - coralWeight - rockWeight - kelpWeight - lavaStoneWeight

	var sandHeight = heightmapSand.get_image().get_pixelv(texture_position).r * sandAmplitude
	var coralHeight = heightmapCoral.get_image().get_pixelv(texture_position).r * coralAmplitude
	var rockHeight = heightmapRock.get_image().get_pixelv(texture_position).r * rockAmplitude
	var kelpHeight = heightmapKelp.get_image().get_pixelv(texture_position).r * kelpAmplitude
	var lavaStoneHeight = heightmapLavaStone.get_image().get_pixelv(texture_position).r * lavaStoneAmplitude

	return (
		sandHeight * sandWeight +
		coralHeight * coralWeight +
		rockHeight * rockWeight +
		kelpHeight * kelpWeight +
		lavaStoneHeight * lavaStoneWeight +
		sandHeight * transitionWeight
	) * generalAmplitude * (height_amplefier * height_difference_amp)

func smooth_step(edge0, edge1, x):
	var t = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0)
	return t * t * (3.0 - 2.0 * t)
