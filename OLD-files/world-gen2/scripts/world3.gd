#world3.gd
# chunkgeneration at the start, then load during player movment
extends Node3D

@onready var main_mesh = $mainMesh
@onready var camera_controller = $"../../CameraController"

@export var areas: Array[Area3D]

const chunk_size = 64
const chunk_generation_radius = 8
const load_radius = 4

var player_translation 

var chunks = {}
var loaded_chunks = {}
var key
var thread
var thread_pool = []
var mesh_shader

func _ready():
	# Access the shader material
	mesh_shader = main_mesh.get_active_material(0) # Assuming the material index is 0
	if mesh_shader and mesh_shader is ShaderMaterial:
		print("Mesh shader valid:", mesh_shader)
		call_deferred("set_shader_parameters")
		call_deferred("generate_chunks")
	else:
		print("No valid shader found")

func _process(delta):
	update_chunks()
	clean_up_chunks()


func generate_chunks():
	for x in range(-chunk_generation_radius * chunk_size / 2, chunk_generation_radius * chunk_size / 2):
		for z in range(-chunk_generation_radius * chunk_size / 2, chunk_generation_radius * chunk_size / 2):
			#key = str(x) + "," + str(z)
			#key = str(x / chunk_size) + "," + str(z / chunk_size)
			key = get_chunk_key(x * chunk_size, z * chunk_size)
			generate_chunk(x, z, key)


func generate_chunk(x, z, key):
	print("generating chunk: ", key)
	var chunk = Chunk.new(
		mesh_shader,
		x * chunk_size, z * chunk_size, chunk_size,
		areas, # Pass the areas directly to the chunk
		false, # loaded
		mesh_shader.get_shader_parameter("biomeNoise"),
		mesh_shader.get_shader_parameter("heightChanger"),
		mesh_shader.get_shader_parameter("heightmapSand"),
		mesh_shader.get_shader_parameter("heightmapCoral"),
		mesh_shader.get_shader_parameter("heightmapRock"),
		mesh_shader.get_shader_parameter("heightmapKelp"),
		mesh_shader.get_shader_parameter("heightmapLavaStone"),
		mesh_shader.get_shader_parameter("generalAmplitude"),
		mesh_shader.get_shader_parameter("sandAmplitude"),
		mesh_shader.get_shader_parameter("coralAmplitude"),
		mesh_shader.get_shader_parameter("rockAmplitude"),
		mesh_shader.get_shader_parameter("kelpAmplitude"),
		mesh_shader.get_shader_parameter("lavaStoneAmplitude"),
		mesh_shader.get_shader_parameter("biomeStrengthAmplifyer"),
		mesh_shader.get_shader_parameter("height_difference_amp"),
		mesh_shader.get_shader_parameter("sandCutof"),
		mesh_shader.get_shader_parameter("coralCutof"),
		mesh_shader.get_shader_parameter("rockCutof"),
		mesh_shader.get_shader_parameter("kelpCutof"),
		mesh_shader.get_shader_parameter("lavaStoneCutof"),
		mesh_shader.get_shader_parameter("textureSand"),
		mesh_shader.get_shader_parameter("textureCoral"),
		mesh_shader.get_shader_parameter("textureKelp"),
		mesh_shader.get_shader_parameter("textureRock"),
		mesh_shader.get_shader_parameter("textureLavaStone")
	)
	chunk.position = Vector3(x*chunk_size, 0, z*chunk_size)
	chunks[key] = chunk
		
func start_thread(chunk):
	thread = Thread.new()
	thread.start(Callable(self, "load_chunk").bind(chunk))
	thread_pool.append(thread)

func load_chunk(chunk):
	#thread.wait_to_finish()  # Wait for any previous load to finish
	chunk.loaded = true 
	if not chunk.is_connected("block_updated", Callable(self, "on_block_updated")):
		chunk.connect("block_updated", Callable(self, "on_block_updated"))
	call_deferred("add_child", chunk)
	print("chunk loaded: ", chunk)

func _exit_tree():
	for thread in thread_pool:
		thread.wait_to_finish()

func get_chunk(key):
	if chunks.has(key):
		return chunks.get(key)
	else:
		print("chunk ", key, " not found in chunks")

func get_chunk_key(x: float, z: float) -> String:
	return "%d,%d" % [int(x / chunk_size), int(z / chunk_size)]


func update_chunks():
	player_translation = camera_controller.global_position # Update to retrieve submarine position
	print("player pos: ", player_translation)
	var p_x = int(player_translation.x) / chunk_size
	var p_z = int(player_translation.z) / chunk_size

	for x in range(p_x - load_radius * 0.5, p_x + load_radius * 0.5):
		for z in range(p_z - load_radius * 0.5, p_z + load_radius * 0.5):
			#print("add chunk()", x, z)
			#key = str(x) + "," + str(z)
			#key = str(x / chunk_size) + "," + str(z / chunk_size)
			key = get_chunk_key(x * chunk_size, z * chunk_size)
			var chunk = get_chunk(key) 
			if chunk.loaded == false:
				start_thread(chunk)
				#print("chunk: ", chunk)
				if chunk != null:
					chunk.should_remove = false

func clean_up_chunks():
	for i in chunks:
		var chunk = chunks[i]
		if chunk.should_remove:
			chunk.queue_free()
			chunk.loaded = false

