#world2.gd
# chunkgeneration at the start, then load during player movment
extends Node3D

@onready var main_mesh = $mainMesh
@onready var camera_controller = $"../../CameraController"

@export var areas: Array[Area3D]

const chunk_size = 64
const chunk_amount = 8

var player_translation 

var chunk_dictionary = {}
var loaded_chunks = {}
var unready_chunks = {}
var key
var thread
var mesh_shader

func _ready():
	# Access the shader material
	mesh_shader = main_mesh.get_active_material(0) # Assuming the material index is 0
	if mesh_shader and mesh_shader is ShaderMaterial:
		print("Mesh shader valid:", mesh_shader)
		generate_chunks()
	else:
		print("No valid shader found")
		
func generate_chunks():
	for x in range(-chunk_amount * chunk_size / 2, chunk_amount * chunk_size / 2):
		for z in range(-chunk_amount * chunk_size / 2, chunk_amount * chunk_size / 2):
			key = str(x) + "," + str(z)
			#key = str(x / chunk_size) + "," + str(z / chunk_size)
			generate_chunk(x, z, key)


func generate_chunk(x, z, key):
	print("generating chunk: ", key)
	var chunk = Chunk.new(
		mesh_shader,
		x * chunk_size, z * chunk_size, chunk_size,
		areas, # Pass the areas directly to the chunk
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
	chunk.position = Vector3(x * chunk_size, 0, z * chunk_size )
	chunk_dictionary[key] = chunk

	
func start_thread(key):
	thread = Thread.new()
	var callable = Callable(self, "load_chunk").bind(key)
	thread.start(callable)

func load_chunk(key):
	#player_translation = camera_controller.global_position
	#print("player_translation: ", player_translation)

	if not loaded_chunks.has(key):  # Check if the chunk is already loaded
		if unready_chunks.has(key):
			thread.wait_to_finish()  # Wait for any previous load to finish
			unready_chunks.erase(key)

	else:
		var chunk = loaded_chunks.get(key)
		if not chunk.is_connected("block_updated", self, "on_block_updated"):
			chunk.connect("block_updated", self, "on_block_updated")

	# Load the chunk and notify the main thread when it's done
	await start_thread # error line
	chunk_finished(key)

func chunk_finished(key):
	print("Chunk finished loading: ", key)
	var chunk = loaded_chunks.get(key)
	add_child(chunk, true)  # Add the loaded chunk to the scene
	loaded_chunks.erase(key)
	unready_chunks.erase(key)


func get_chunk(key):
	if loaded_chunks.has(key):
		return loaded_chunks.get(key)
	return null

func _process(delta):
	update_chunks()
	clean_up_chunks()
	reset_chunks()

func update_chunks():
	player_translation = camera_controller.global_position # Update to retrieve submarine position
	print("player pos: ", player_translation)
	var p_x = int(player_translation.x) / chunk_size
	var p_z = int(player_translation.z) / chunk_size

	for x in range(p_x - chunk_amount * 0.5, p_x + chunk_amount * 0.5):
		for z in range(p_z - chunk_amount * 0.5, p_z + chunk_amount * 0.5):
			#print("add chunk()", x, z)
			key = str(x) + "," + str(z)
			#key = str(x / chunk_size) + "," + str(z / chunk_size)
			if loaded_chunks.has(key):
				return
			
			else:
				start_thread(key)
				var chunk = get_chunk(key) # potesial problem with getting chunk
				#print("chunk: ", chunk)
				if chunk != null:
					chunk.should_remove = false

func clean_up_chunks():
	for key in loaded_chunks:
		var chunk = loaded_chunks[key]
		if chunk.should_remove:
			chunk.queue_free()
			loaded_chunks.erase(key)

func reset_chunks():
	for key in loaded_chunks:
		loaded_chunks[key].should_remove = true
