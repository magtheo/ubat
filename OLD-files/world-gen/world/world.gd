#world.gd
extends Node3D

@onready var main_mesh = $mainMesh
@onready var camera_controller = $"../../CameraController"

@export var areas: Array[Area3D]

const chunk_size = 64
const chunk_amount = 8

var player_translation 

var chunks = {}
var unready_chunks = {}
var thread
var is_thread_active = false
var mesh_shader

func _ready():
	# Access the shader material
	mesh_shader = main_mesh.get_active_material(0) # Assuming the material index is 0
	if mesh_shader and mesh_shader is ShaderMaterial:
		print("Mesh shader valid:", mesh_shader)
	else:
		print("No valid shader found")
		
	thread = Thread.new()


func add_chunk(x, z):
	var key = str(x) + "," + str(z)
	if chunks.has(key) or unready_chunks.has(key):
		return
	
	#print("is thread active", is_thread_active)
	if not thread.is_started():
		is_thread_active = true
		var callable = Callable(self, "load_chunk").bind([x, z])
		thread.start(callable)
		unready_chunks[key] = 1

func load_chunk(arr):
	player_translation = camera_controller.global_position
	print("player_translation: ", player_translation)
	var x = arr[0]
	var z = arr[1]

	print("loading chunk", x, z)
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

	call_deferred("load_done", chunk)

func load_done(chunk):
	print("loadDone")
	add_child(chunk, true)
	var key = str(chunk.x / chunk_size) + "," + str(chunk.z / chunk_size)
	chunks[key] = chunk
	unready_chunks.erase(key)
	is_thread_active = false
	thread.wait_to_finish()

func get_chunk(x, z):
	var key = str(x) + "," + str(z)
	if chunks.has(key):
		return chunks.get(key)
	return null

func _process(delta):
	update_chunks()
	clean_up_chunks()
	reset_chunks()

func update_chunks():
	player_translation = camera_controller.global_position # Update to retrieve submarine position
	#print("player pos: ", player_translation)
	var p_x = int(player_translation.x) / chunk_size
	var p_z = int(player_translation.z) / chunk_size

	for x in range(p_x - chunk_amount * 0.5, p_x + chunk_amount * 0.5):
		for z in range(p_z - chunk_amount * 0.5, p_z + chunk_amount * 0.5):
			add_chunk(x, z)
			#print("add chunk()", x, z)
			var chunk = get_chunk(x, z) # potesial problem with getting chunk
			#print("chunk: ", chunk)
			if chunk != null:
				chunk.should_remove = false

func clean_up_chunks():
	for key in chunks:
		var chunk = chunks[key]
		if chunk.should_remove:
			chunk.queue_free()
			chunks.erase(key)

func reset_chunks():
	for key in chunks:
		chunks[key].should_remove = true
