#world.gd
extends Node3D

@onready var main_mesh = $mainMesh
@onready var camera_controller = $"../../CameraController"

@export var areas: Array[Area3D]

const chunk_size = 64
const chunk_amount = 16

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

	if not is_thread_active:
		is_thread_active = true
		var callable = Callable(self, "load_chunk").bind([x, z])
		thread.start(callable)
		unready_chunks[key] = 1

func load_chunk(arr):
	var x = arr[0]
	var z = arr[1]

	print("loading chunk")
	var chunk = Chunk.new(
		mesh_shader,
		x * chunk_size, z * chunk_size, chunk_size,
		areas, # Pass the areas directly to the chunk
		mesh_shader.get_shader_parameter("biomeNoise"),
		mesh_shader.get_shader_parameter("heightChanger"),
		mesh_shader.get_shader_parameter("heightmapSand"),
		mesh_shader.get_shader_parameter("heightmapCorral"),
		mesh_shader.get_shader_parameter("heightmapRock"),
		mesh_shader.get_shader_parameter("heightmapKelp"),
		mesh_shader.get_shader_parameter("heightmapLavaStone"),
		mesh_shader.get_shader_parameter("generalAmplitude"),
		mesh_shader.get_shader_parameter("sandAmplitude"),
		mesh_shader.get_shader_parameter("corralAmplitude"),
		mesh_shader.get_shader_parameter("rockAmplitude"),
		mesh_shader.get_shader_parameter("kelpAmplitude"),
		mesh_shader.get_shader_parameter("lavaStoneAmplitude"),
		mesh_shader.get_shader_parameter("biomeStrengthAmplifyer"),
		mesh_shader.get_shader_parameter("height_difference_amp"),
		mesh_shader.get_shader_parameter("sandCutof"),
		mesh_shader.get_shader_parameter("corralCutof"),
		mesh_shader.get_shader_parameter("rockCutof"),
		mesh_shader.get_shader_parameter("kelpCutof"),
		mesh_shader.get_shader_parameter("lavaStoneCutof")
	)
	chunk.global_position = Vector3(x * chunk_size, 0, z * chunk_size)

	call_deferred("load_done", chunk)

func load_done(chunk):
	add_child(chunk)
	var key = str(chunk.x / chunk_size) + "," + str(chunk.z / chunk_size)
	chunks[key] = chunk
	unready_chunks.erase(key)
	is_thread_active = false

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
	var player_translation = camera_controller.global_position # Update to retrieve submarine position
	var p_x = int(player_translation.x) / chunk_size
	var p_z = int(player_translation.z) / chunk_size

	for x in range(p_x - chunk_amount * 0.5, p_x + chunk_amount * 0.5):
		for z in range(p_z - chunk_amount * 0.5, p_z + chunk_amount * 0.5):
			add_chunk(x, z)
			var chunk = get_chunk(x, z)
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
