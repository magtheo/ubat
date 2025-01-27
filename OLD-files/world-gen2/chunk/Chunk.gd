extends Node3D
class_name Chunk2

var mesh_instance
var shader_material
var x
var z
var chunk_size
var should_remove
var loaded

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

var sandCutoff
var coralCutoff
var rockCutoff
var kelpCutoff
var lavaStoneCutoff

var textureSand
var textureCoral
var textureRock
var textureKelp
var textureLavaStone

func _init(
	shader, x_pos, z_pos, chunk_size,
	loaded,
	biomeNoise, heightChanger,
	heightmapSand, heightmapCoral,
	heightmapRock, heightmapKelp, heightmapLavaStone, generalAmplitude,
	sandAmplitude, coralAmplitude, rockAmplitude, kelpAmplitude, lavaStoneAmplitude,
	biomeStrengthAmplifyer, height_difference_amp,
	sandCutoff, coralCutoff, rockCutoff, kelpCutoff, lavaStoneCutoff,
	textureSand, textureCoral, textureRock, textureKelp, textureLavaStone
):
	self.shader_material = shader
	self.x = x_pos
	self.z = z_pos
	self.chunk_size = chunk_size
	self.loaded = loaded
	self.biomeNoise = biomeNoise
	self.heightChanger = heightChanger
	self.heightmapSand = heightmapSand
	self.heightmapCoral = heightmapCoral
	self.heightmapRock = heightmapRock
	self.heightmapKelp = heightmapKelp
	self.heightmapLavaStone = heightmapLavaStone
	self.generalAmplitude = generalAmplitude
	self.sandAmplitude = sandAmplitude
	self.coralAmplitude = coralAmplitude
	self.rockAmplitude = rockAmplitude
	self.kelpAmplitude = kelpAmplitude
	self.lavaStoneAmplitude = lavaStoneAmplitude
	self.biomeStrengthAmplifyer = biomeStrengthAmplifyer
	self.height_difference_amp = height_difference_amp
	self.sandCutoff = sandCutoff
	self.coralCutoff = coralCutoff
	self.rockCutoff = rockCutoff
	self.kelpCutoff = kelpCutoff
	self.lavaStoneCutoff = lavaStoneCutoff
	self.textureSand = textureSand
	self.textureCoral = textureCoral
	self.textureRock = textureRock
	self.textureKelp = textureKelp
	self.textureLavaStone = textureLavaStone

func _ready():
	generate_chunk()

func generate_chunk():
	print("loading chunk: ", " x:", x, " z:", z)

	var plane_mesh = PlaneMesh.new()
	plane_mesh.size = Vector2(chunk_size, chunk_size)
	plane_mesh.subdivide_depth = chunk_size * 0.2
	plane_mesh.subdivide_width = chunk_size * 0.2

	var surface_tool = SurfaceTool.new()
	var data_tool = MeshDataTool.new()
	surface_tool.create_from(plane_mesh, 0)
	var array_plane = surface_tool.commit()
	var error = data_tool.create_from_surface(array_plane, 0)
	
	for i in range(data_tool.get_vertex_count()):
		var vertex = data_tool.get_vertex(i)
		var texture_position = Vector2(vertex.x + x, vertex.z + z) * 0.5 / float(biomeNoise.get_width())
		var height = get_vertex_height(texture_position)
		var color = get_texture_color(texture_position)
		
		vertex.y = height
		data_tool.set_vertex(i, vertex)
		data_tool.set_vertex_color(i, color)

	mesh_instance = MeshInstance3D.new()
	mesh_instance.mesh = surface_tool.commit()
	mesh_instance.create_trimesh_collision()
	mesh_instance.mesh.surface_set_material(0, shader_material)
	mesh_instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF

	add_child(mesh_instance)
	print("Chunk loaded with global position: ", global_position)

func get_texture_color(texture_position):
	var biome_strength = biomeNoise.get_pixelv(texture_position).r * biomeStrengthAmplifyer

	var sand_weight = smooth_step(sandCutoff - 0.1, sandCutoff + 0.1, biome_strength)
	var coral_weight = smooth_step(coralCutoff - 0.1, coralCutoff + 0.1, biome_strength)
	var rock_weight = smooth_step(rockCutoff - 0.1, rockCutoff + 0.1, biome_strength)
	var kelp_weight = smooth_step(kelpCutoff - 0.1, kelpCutoff + 0.1, biome_strength)
	var lava_stone_weight = smooth_step(lavaStoneCutoff - 0.1, lavaStoneCutoff + 0.1, biome_strength)

	var transition_weight = 1.0 - sand_weight - coral_weight - rock_weight - kelp_weight - lava_stone_weight

	var sand_texture = textureSand.get_pixelv(texture_position)
	var coral_texture = textureCoral.get_pixelv(texture_position)
	var rock_texture = textureRock.get_pixelv(texture_position)
	var kelp_texture = textureKelp.get_pixelv(texture_position)
	var lava_stone_texture = textureLavaStone.get_pixelv(texture_position)

	return (
		sand_texture * sand_weight +
		coral_texture * coral_weight +
		rock_texture * rock_weight +
		kelp_texture * kelp_weight +
		lava_stone_texture * lava_stone_weight +
		sand_texture * transition_weight
	)

func get_vertex_height(texture_position):
	var biome_strength = biomeNoise.get_pixelv(texture_position).r * biomeStrengthAmplifyer
	var height_amplifier = heightChanger.get_pixelv(texture_position).r

	var sand_weight = smooth_step(sandCutoff - 0.1, sandCutoff + 0.1, biome_strength)
	var coral_weight = smooth_step(coralCutoff - 0.1, coralCutoff + 0.1, biome_strength)
	var rock_weight = smooth_step(rockCutoff - 0.1, rockCutoff + 0.1, biome_strength)
	var kelp_weight = smooth_step(kelpCutoff - 0.1, kelpCutoff + 0.1, biome_strength)
	var lava_stone_weight = smooth_step(lavaStoneCutoff - 0.1, lavaStoneCutoff + 0.1, biome_strength)

	var transition_weight = 1.0 - sand_weight - coral_weight - rock_weight - kelp_weight - lava_stone_weight

	var sand_height = heightmapSand.get_pixelv(texture_position).r * sandAmplitude
	var coral_height = heightmapCoral.get_pixelv(texture_position).r * coralAmplitude
	var rock_height = heightmapRock.get_pixelv(texture_position).r * rockAmplitude
	var kelp_height = heightmapKelp.get_pixelv(texture_position).r * kelpAmplitude
	var lava_stone_height = heightmapLavaStone.get_pixelv(texture_position).r * lavaStoneAmplitude

	return (
		sand_height * sand_weight +
		coral_height * coral_weight +
		rock_height * rock_weight +
		kelp_height * kelp_weight +
		lava_stone_height * lava_stone_weight +
		sand_height * transition_weight
	) * generalAmplitude * (height_amplifier * height_difference_amp)

func smooth_step(edge0, edge1, x):
	var t = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0)
	return t * t * (3.0 - 2.0 * t)
