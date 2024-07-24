# chunk.gd
extends Node3D
class_name Chunk

var mesh_instance
var shader_material
var x
var z
var chunk_size
var should_remove

var areas: Array[Area3D]

var biomeNoise
var heightChanger
var heightmapSand
var heightmapCorral
var heightmapRock
var heightmapKelp
var heightmapLavaStone

var generalAmplitude
var sandAmplitude
var corralAmplitude
var rockAmplitude
var kelpAmplitude
var lavaStoneAmplitude
var biomeStrengthAmplifyer
var height_difference_amp

var sandCutof
var corralCutof
var rockCutof
var kelpCutof
var lavaStoneCutof

func _init(
	shader, x_pos, z_pos, chunk_size,
	areas, # Pass the areas to the chunk
	biomeNoise, heightChanger,
	heightmapSand, heightmapCorral,
	heightmapRock, heightmapKelp, heightmapLavaStone, generalAmplitude,
	sandAmplitude, corralAmplitude, rockAmplitude, kelpAmplitude, lavaStoneAmplitude,
	biomeStrengthAmplifyer, height_difference_amp,
	sandCutof, corralCutof, rockCutof, kelpCutof, lavaStoneCutof
):
	self.shader_material = shader
	self.x = x_pos
	self.z = z_pos
	self.chunk_size = chunk_size
	self.areas = areas
	self.biomeNoise = biomeNoise
	self.heightChanger = heightChanger
	self.heightmapSand = heightmapSand
	self.heightmapCorral = heightmapCorral
	self.heightmapRock = heightmapRock
	self.heightmapKelp = heightmapKelp
	self.heightmapLavaStone = heightmapLavaStone
	self.generalAmplitude = generalAmplitude
	self.sandAmplitude = sandAmplitude
	self.corralAmplitude = corralAmplitude
	self.rockAmplitude = rockAmplitude
	self.kelpAmplitude = kelpAmplitude
	self.lavaStoneAmplitude = lavaStoneAmplitude
	self.biomeStrengthAmplifyer = biomeStrengthAmplifyer
	self.height_difference_amp = height_difference_amp
	self.sandCutof = sandCutof
	self.corralCutof = corralCutof
	self.rockCutof = rockCutof
	self.kelpCutof = kelpCutof
	self.lavaStoneCutof = lavaStoneCutof

func _ready():
	generate_chunk()

func generate_chunk():
	print("Generating chunk: ", " x:", x, " z:", z)  # Debugging statement

	# Create the plane mesh
	var plane_mesh = PlaneMesh.new()
	plane_mesh.size = Vector2(chunk_size, chunk_size)
	plane_mesh.subdivide_depth = chunk_size * 0.5
	plane_mesh.subdivide_width = chunk_size * 0.5

	var surface_tool = SurfaceTool.new()
	var data_tool = MeshDataTool.new()
	surface_tool.create_from(plane_mesh, 0)
	var array_plane = surface_tool.commit()
	var error = data_tool.create_from_surface(array_plane, 0)
	
	# Modify vertex heights
	for i in range(data_tool.get_vertex_count()):
		var vertex = data_tool.get_vertex(i)
		vertex.y = get_vertex_height(vertex.x + x, vertex.z + z)
		data_tool.set_vertex(i, vertex)

	# Create the final mesh instance
	mesh_instance = MeshInstance3D.new()
	mesh_instance.mesh = surface_tool.commit()
	mesh_instance.create_trimesh_collision()
	mesh_instance.mesh.surface_set_material(0, shader_material)
	mesh_instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF

	add_child(mesh_instance)


func get_vertex_height(x, z):
	var biomeNoise_image = biomeNoise.get_image()
	var heightAmp_image = heightChanger.get_image()
	
	var texture_position = Vector2(x, z) * 0.5 / float(biomeNoise_image.get_width())
	
	var biomeStrength = biomeNoise_image.get_pixelv(texture_position).r * biomeStrengthAmplifyer
	var height_amplefier = heightAmp_image.get_pixelv(texture_position).r

	var sandWeight = smooth_step(sandCutof - 0.1, sandCutof + 0.1, biomeStrength)
	var corralWeight = smooth_step(corralCutof - 0.1, corralCutof + 0.1, biomeStrength)
	var rockWeight = smooth_step(rockCutof - 0.1, rockCutof + 0.1, biomeStrength)
	var kelpWeight = smooth_step(kelpCutof - 0.1, kelpCutof + 0.1, biomeStrength)
	var lavaStoneWeight = smooth_step(lavaStoneCutof - 0.1, lavaStoneCutof + 0.1, biomeStrength)

	var transitionWeight = 1.0 - sandWeight - corralWeight - rockWeight - kelpWeight - lavaStoneWeight

	var sandHeight = heightmapSand.get_image().get_pixelv(texture_position).r * sandAmplitude
	var corralHeight = heightmapCorral.get_image().get_pixelv(texture_position).r * corralAmplitude
	var rockHeight = heightmapRock.get_image().get_pixelv(texture_position).r * rockAmplitude
	var kelpHeight = heightmapKelp.get_image().get_pixelv(texture_position).r * kelpAmplitude
	var lavaStoneHeight = heightmapLavaStone.get_image().get_pixelv(texture_position).r * lavaStoneAmplitude

	return (
		sandHeight * sandWeight +
		corralHeight * corralWeight +
		rockHeight * rockWeight +
		kelpHeight * kelpWeight +
		lavaStoneHeight * lavaStoneWeight +
		sandHeight * transitionWeight
	) * generalAmplitude * (height_amplefier * height_difference_amp)




func smooth_step(edge0, edge1, x):
	var t = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0)
	return t * t * (3.0 - 2.0 * t)
